use tally_app::{
    AccountsService, AppError, CardsService, CategoriesService, CreateAccountInput,
    CreateCardPaymentInput, CreateCardPurchaseInput, CreateCategoryInput, MovementsService,
    UpdateInstallmentPlanInput,
};
use tally_infra::{
    Database, SqliteAccountsRepository, SqliteCardsRepository, SqliteCategoriesRepository,
    SqliteMovementsRepository,
};
use tempfile::TempDir;

struct Fixture {
    _dir: TempDir,
    accounts: AccountsService<SqliteAccountsRepository>,
    categories: CategoriesService<SqliteCategoriesRepository>,
    cards: CardsService<SqliteCardsRepository>,
    movements: MovementsService<SqliteMovementsRepository>,
    db: Database,
}

async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("temp dir");
    let db = Database::open(dir.path().join("tally.db"))
        .await
        .expect("database should open");
    Fixture {
        accounts: AccountsService::new(SqliteAccountsRepository::new(db.clone())),
        categories: CategoriesService::new(SqliteCategoriesRepository::new(db.clone())),
        cards: CardsService::new(SqliteCardsRepository::new(db.clone())),
        movements: MovementsService::new(SqliteMovementsRepository::new(db.clone())),
        db,
        _dir: dir,
    }
}

async fn account(fixture: &Fixture, name: &str, opening: i64) -> String {
    fixture
        .accounts
        .create(CreateAccountInput {
            name: name.to_owned(),
            account_type: "bank".to_owned(),
            opening_balance_cents: opening,
            credit_limit_cents: None,
        })
        .await
        .expect("account should be created")
        .id
}

async fn card(fixture: &Fixture, name: &str, opening_debt: i64, limit: Option<i64>) -> String {
    fixture
        .accounts
        .create(CreateAccountInput {
            name: name.to_owned(),
            account_type: "credit_card".to_owned(),
            opening_balance_cents: opening_debt,
            credit_limit_cents: limit,
        })
        .await
        .expect("card should be created")
        .id
}

async fn category(fixture: &Fixture, name: &str) -> String {
    fixture
        .categories
        .create(CreateCategoryInput {
            name: name.to_owned(),
            kind: "expense".to_owned(),
            icon: String::new(),
        })
        .await
        .expect("category should be created")
        .id
}

fn purchase_input(
    card_id: &str,
    category_id: &str,
    total_cents: i64,
    count: i64,
    first_due_on: &str,
) -> CreateCardPurchaseInput {
    CreateCardPurchaseInput {
        card_account_id: card_id.to_owned(),
        category_id: category_id.to_owned(),
        total_cents,
        installment_count: count,
        first_due_on: first_due_on.to_owned(),
        occurred_on: "2026-01-15".to_owned(),
        occurred_time: Some("12:00".to_owned()),
        manual_period: None,
        description: Some("Compra".to_owned()),
        payment_method: "credit".to_owned(),
    }
}

fn payment_input(
    card_id: &str,
    bank_id: &str,
    amount_cents: i64,
    occurred_on: &str,
) -> CreateCardPaymentInput {
    CreateCardPaymentInput {
        card_account_id: card_id.to_owned(),
        bank_account_id: bank_id.to_owned(),
        amount_cents,
        occurred_on: occurred_on.to_owned(),
        occurred_time: Some("10:00".to_owned()),
        manual_period: None,
        description: Some("Pagamento".to_owned()),
    }
}

async fn scalar(fixture: &Fixture, sql: &'static str) -> i64 {
    fixture
        .db
        .call(move |connection| connection.query_row(sql, [], |row| row.get::<_, i64>(0)))
        .await
        .expect("scalar should work")
}

async fn net_worth(fixture: &Fixture) -> i64 {
    scalar(
        fixture,
        "SELECT COALESCE(SUM(p.amount_cents), 0)
         FROM postings p
         JOIN journal_entries e ON e.id = p.entry_id
         JOIN accounts a ON a.id = p.account_id
         WHERE e.posted_at IS NOT NULL AND a.class IN ('asset', 'liability')",
    )
    .await
}

async fn recognized_expense(fixture: &Fixture) -> i64 {
    scalar(
        fixture,
        "SELECT COALESCE(SUM(p.amount_cents), 0)
         FROM postings p
         JOIN journal_entries e ON e.id = p.entry_id
         WHERE e.posted_at IS NOT NULL
           AND p.account_id = '00000000-0000-7000-8000-000000000003'",
    )
    .await
}

#[tokio::test]
async fn purchase_splits_into_three_installments_and_debt_is_correct() {
    let fixture = fixture().await;
    let card_id = card(&fixture, "Cartao", 5_000, Some(500_000)).await;
    let category_id = category(&fixture, "Compras").await;

    let plan = fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            3,
            "2026-02-10",
        ))
        .await
        .expect("purchase should work");

    assert_eq!(plan.installment_count, 3);
    assert_eq!(
        plan.installments
            .iter()
            .map(|item| item.amount_cents)
            .collect::<Vec<_>>(),
        vec![30_000, 30_000, 30_000]
    );
    assert_eq!(plan.total_cents, 90_000);
    assert_eq!(recognized_expense(&fixture).await, 90_000);

    let summary = fixture
        .cards
        .card_summary(&card_id)
        .await
        .expect("summary should work");
    assert_eq!(summary.debt_cents, 95_000);
    assert_eq!(summary.initial_debt_cents, 5_000);
    assert_eq!(summary.available_limit_cents, Some(405_000));
    assert!(!summary.over_limit);
    assert_eq!(net_worth(&fixture).await, -95_000);

    let movements = fixture.movements.list(None).await.expect("movements");
    assert_eq!(
        movements.len(),
        1,
        "reconhecimentos nao aparecem no extrato"
    );
}

#[tokio::test]
async fn payment_reduces_debt_and_never_becomes_an_expense() {
    let fixture = fixture().await;
    let bank_id = account(&fixture, "Banco", 100_000).await;
    let card_id = card(&fixture, "Cartao", 0, Some(500_000)).await;
    let category_id = category(&fixture, "Compras").await;

    fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            3,
            "2026-02-10",
        ))
        .await
        .expect("purchase should work");
    let expense_before = recognized_expense(&fixture).await;

    let payment = fixture
        .cards
        .create_payment(payment_input(&card_id, &bank_id, 30_000, "2026-03-05"))
        .await
        .expect("payment should work");

    assert_eq!(payment.amount_cents, 30_000);
    assert_eq!(payment.allocations.len(), 1);
    assert_eq!(payment.allocations[0].amount_cents, 30_000);
    assert_eq!(payment.allocations[0].seq, 1);
    assert_eq!(recognized_expense(&fixture).await, expense_before);

    let summary = fixture.cards.card_summary(&card_id).await.expect("summary");
    assert_eq!(summary.debt_cents, 60_000);
    assert_eq!(net_worth(&fixture).await, 10_000);
    assert_eq!(
        fixture
            .accounts
            .list(false)
            .await
            .expect("accounts")
            .into_iter()
            .find(|account| account.id == bank_id)
            .expect("bank")
            .balance_cents,
        70_000
    );
}

#[tokio::test]
async fn overpayment_stays_unallocated_and_never_becomes_an_expense() {
    let fixture = fixture().await;
    let bank_id = account(&fixture, "Banco", 100_000).await;
    let card_id = card(&fixture, "Cartao", 0, Some(500_000)).await;
    let category_id = category(&fixture, "Compras").await;

    fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            3,
            "2035-02-10",
        ))
        .await
        .expect("purchase should work");
    let expense_before = recognized_expense(&fixture).await;

    let payment = fixture
        .cards
        .create_payment(payment_input(&card_id, &bank_id, 100_000, "2026-03-05"))
        .await
        .expect("payment should work");

    assert_eq!(
        payment
            .allocations
            .iter()
            .map(|allocation| allocation.amount_cents)
            .sum::<i64>(),
        90_000
    );
    assert_eq!(payment.unallocated_cents, 10_000);
    assert_eq!(recognized_expense(&fixture).await, expense_before);
    assert_eq!(
        scalar(
            &fixture,
            "SELECT COUNT(*) FROM journal_entries WHERE kind = 'expense' AND posted_at IS NOT NULL",
        )
        .await,
        3
    );

    let summary = fixture.cards.card_summary(&card_id).await.expect("summary");
    assert_eq!(summary.debt_cents, 0);
    assert_eq!(summary.credit_cents, 10_000);
    assert_eq!(summary.available_limit_cents, Some(500_000));
    assert_eq!(net_worth(&fixture).await, 10_000);
}

#[tokio::test]
async fn payment_is_fifo_and_covers_the_initial_debt_first() {
    let fixture = fixture().await;
    let bank_id = account(&fixture, "Banco", 200_000).await;
    let card_id = card(&fixture, "Cartao", 5_000, None).await;
    let category_id = category(&fixture, "Compras").await;

    fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            3,
            "2026-01-10",
        ))
        .await
        .expect("purchase should work");

    let payment = fixture
        .cards
        .create_payment(payment_input(&card_id, &bank_id, 35_000, "2026-02-01"))
        .await
        .expect("payment should work");

    assert_eq!(payment.initial_balance_cents, 5_000);
    assert_eq!(payment.allocations.len(), 1);
    assert_eq!(payment.allocations[0].seq, 1);

    let summary = fixture.cards.card_summary(&card_id).await.expect("summary");
    assert_eq!(summary.debt_cents, 60_000);
    assert_eq!(summary.initial_debt_cents, 0);

    let plan = fixture
        .cards
        .get_plan(&payment.allocations[0].plan_id)
        .await
        .expect("plan");
    assert_eq!(plan.installments[0].paid_cents, 30_000);
    assert_eq!(plan.installments[1].paid_cents, 0);
}

#[tokio::test]
async fn partial_payment_marks_only_the_installment_as_partial() {
    let fixture = fixture().await;
    let bank_id = account(&fixture, "Banco", 200_000).await;
    let card_id = card(&fixture, "Cartao", 0, None).await;
    let category_id = category(&fixture, "Compras").await;

    let plan = fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            3,
            "2035-01-10",
        ))
        .await
        .expect("purchase should work");

    fixture
        .cards
        .create_payment(payment_input(&card_id, &bank_id, 10_000, "2026-02-01"))
        .await
        .expect("payment should work");

    let plan = fixture.cards.get_plan(&plan.id).await.expect("plan");
    assert_eq!(plan.installments[0].paid_cents, 10_000);
    assert_eq!(plan.installments[0].status, "partial");
    assert_eq!(plan.installments[1].paid_cents, 0);
    assert_eq!(plan.paid_cents, 10_000);
    assert_eq!(plan.outstanding_cents, 80_000);
    assert_eq!(
        fixture
            .cards
            .card_summary(&card_id)
            .await
            .expect("summary")
            .debt_cents,
        80_000
    );
}

#[tokio::test]
async fn deleting_a_payment_reopens_the_installment() {
    let fixture = fixture().await;
    let bank_id = account(&fixture, "Banco", 200_000).await;
    let card_id = card(&fixture, "Cartao", 0, None).await;
    let category_id = category(&fixture, "Compras").await;

    let plan = fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            3,
            "2035-01-10",
        ))
        .await
        .expect("purchase should work");
    let payment = fixture
        .cards
        .create_payment(payment_input(&card_id, &bank_id, 30_000, "2026-02-01"))
        .await
        .expect("payment should work");

    let worth_before = net_worth(&fixture).await;
    fixture
        .cards
        .delete_payment(&payment.id)
        .await
        .expect("payment should be removed");

    assert!(
        fixture
            .cards
            .list_payments(None)
            .await
            .expect("payments")
            .is_empty()
    );
    let plan = fixture.cards.get_plan(&plan.id).await.expect("plan");
    assert_eq!(plan.installments[0].paid_cents, 0);
    assert_eq!(
        fixture
            .cards
            .card_summary(&card_id)
            .await
            .expect("summary")
            .debt_cents,
        90_000
    );
    assert_eq!(net_worth(&fixture).await, worth_before);
}

#[tokio::test]
async fn rounding_keeps_the_purchase_total_exact() {
    let fixture = fixture().await;
    let card_id = card(&fixture, "Cartao", 0, None).await;
    let category_id = category(&fixture, "Compras").await;

    let plan = fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            10_000,
            3,
            "2035-01-10",
        ))
        .await
        .expect("purchase should work");

    assert_eq!(
        plan.installments
            .iter()
            .map(|item| item.amount_cents)
            .collect::<Vec<_>>(),
        vec![3_334, 3_333, 3_333]
    );
    assert_eq!(
        plan.installments
            .iter()
            .map(|item| item.amount_cents)
            .sum::<i64>(),
        10_000
    );
    assert_eq!(recognized_expense(&fixture).await, 10_000);
}

#[tokio::test]
async fn month_end_dates_are_clamped_without_losing_the_original_day() {
    let fixture = fixture().await;
    let card_id = card(&fixture, "Cartao", 0, None).await;
    let category_id = category(&fixture, "Compras").await;

    let plan = fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            4,
            "2027-01-31",
        ))
        .await
        .expect("purchase should work");

    assert_eq!(
        plan.installments
            .iter()
            .map(|item| item.due_on.as_str())
            .collect::<Vec<_>>(),
        vec!["2027-01-31", "2027-02-28", "2027-03-31", "2027-04-30"]
    );
}

#[tokio::test]
async fn over_limit_purchase_is_allowed_but_reported() {
    let fixture = fixture().await;
    let card_id = card(&fixture, "Cartao", 0, Some(50_000)).await;
    let category_id = category(&fixture, "Compras").await;

    fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            1,
            "2035-01-10",
        ))
        .await
        .expect("purchase should be allowed over the limit");

    let summary = fixture.cards.card_summary(&card_id).await.expect("summary");
    assert!(summary.over_limit);
    assert_eq!(summary.available_limit_cents, Some(-40_000));
}

#[tokio::test]
async fn future_installments_do_not_duplicate_the_debt() {
    let fixture = fixture().await;
    let card_id = card(&fixture, "Cartao", 0, None).await;
    let category_id = category(&fixture, "Compras").await;

    fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            6,
            "2035-01-10",
        ))
        .await
        .expect("purchase should work");

    let summary = fixture.cards.card_summary(&card_id).await.expect("summary");
    assert_eq!(summary.debt_cents, 90_000);
    assert_eq!(summary.future_cents, 90_000);
    assert_eq!(summary.overdue_cents, 0);
    assert_eq!(summary.current_month_cents, 0);
    assert_eq!(net_worth(&fixture).await, -90_000);
}

#[tokio::test]
async fn each_installment_is_recognized_exactly_once() {
    let fixture = fixture().await;
    let card_id = card(&fixture, "Cartao", 0, None).await;
    let category_id = category(&fixture, "Compras").await;

    let plan = fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            3,
            "2035-01-10",
        ))
        .await
        .expect("purchase should work");

    let mut installment_ids = plan
        .installments
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    installment_ids.sort();
    installment_ids.dedup();
    assert_eq!(installment_ids.len(), 3);
    assert_eq!(
        scalar(
            &fixture,
            "SELECT COUNT(*) FROM journal_entries WHERE kind = 'expense' AND posted_at IS NOT NULL",
        )
        .await,
        3
    );
    assert_eq!(recognized_expense(&fixture).await, 90_000);
}

#[tokio::test]
async fn deleting_a_plan_with_payments_is_rejected() {
    let fixture = fixture().await;
    let bank_id = account(&fixture, "Banco", 200_000).await;
    let card_id = card(&fixture, "Cartao", 0, None).await;
    let category_id = category(&fixture, "Compras").await;

    let plan = fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            3,
            "2035-01-10",
        ))
        .await
        .expect("purchase should work");
    fixture
        .cards
        .create_payment(payment_input(&card_id, &bank_id, 30_000, "2026-02-01"))
        .await
        .expect("payment should work");

    assert!(matches!(
        fixture.cards.delete_plan(&plan.id).await,
        Err(AppError::Conflict { .. })
    ));
    assert_eq!(
        fixture
            .cards
            .card_summary(&card_id)
            .await
            .expect("summary")
            .debt_cents,
        60_000
    );
}

#[tokio::test]
async fn deleting_an_unpaid_plan_restores_the_card_debt() {
    let fixture = fixture().await;
    let card_id = card(&fixture, "Cartao", 0, None).await;
    let category_id = category(&fixture, "Compras").await;

    let plan = fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            3,
            "2035-01-10",
        ))
        .await
        .expect("purchase should work");

    fixture
        .cards
        .delete_plan(&plan.id)
        .await
        .expect("plan should be removed");

    assert_eq!(
        fixture
            .cards
            .card_summary(&card_id)
            .await
            .expect("summary")
            .debt_cents,
        0
    );
    assert_eq!(recognized_expense(&fixture).await, 0);
    assert_eq!(net_worth(&fixture).await, 0);
}

#[tokio::test]
async fn purchase_requires_a_card_and_an_expense_category() {
    let fixture = fixture().await;
    let bank_id = account(&fixture, "Banco", 100_000).await;
    let category_id = category(&fixture, "Compras").await;
    let income_id = fixture
        .categories
        .create(CreateCategoryInput {
            name: "Salario".to_owned(),
            kind: "income".to_owned(),
            icon: String::new(),
        })
        .await
        .expect("category")
        .id;

    assert!(matches!(
        fixture
            .cards
            .create_purchase(purchase_input(
                &bank_id,
                &category_id,
                90_000,
                3,
                "2035-01-10"
            ))
            .await,
        Err(AppError::Validation { .. })
    ));
    assert!(matches!(
        fixture
            .cards
            .create_purchase(purchase_input(
                "missing",
                &category_id,
                90_000,
                3,
                "2035-01-10"
            ))
            .await,
        Err(AppError::NotFound { .. })
    ));
    let card_id = card(&fixture, "Cartao", 0, None).await;
    assert!(matches!(
        fixture
            .cards
            .create_purchase(purchase_input(
                &card_id,
                &income_id,
                90_000,
                3,
                "2035-01-10"
            ))
            .await,
        Err(AppError::Validation { .. })
    ));
}

#[tokio::test]
async fn payment_rejects_a_liability_as_the_source_account() {
    let fixture = fixture().await;
    let card_id = card(&fixture, "Cartao", 0, None).await;
    let other_card = card(&fixture, "Outro cartao", 0, None).await;

    assert!(matches!(
        fixture
            .cards
            .create_payment(payment_input(&card_id, &other_card, 10_000, "2026-02-01"))
            .await,
        Err(AppError::Validation { .. })
    ));
}

#[tokio::test]
async fn updating_an_unpaid_plan_rebuilds_installments_and_debt() {
    let fixture = fixture().await;
    let card_id = card(&fixture, "Cartao", 0, None).await;
    let category_id = category(&fixture, "Compras").await;
    let other_category = category(&fixture, "Casa").await;

    let plan = fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            3,
            "2035-01-10",
        ))
        .await
        .expect("purchase should work");

    let updated = fixture
        .cards
        .update_plan(
            &plan.id,
            UpdateInstallmentPlanInput {
                category_id: other_category.clone(),
                total_cents: 120_000,
                installment_count: 4,
                first_due_on: "2035-02-15".to_owned(),
                period: "noite".to_owned(),
                description: Some("Compra revisada".to_owned()),
            },
        )
        .await
        .expect("plan should be updated");

    assert_eq!(updated.total_cents, 120_000);
    assert_eq!(updated.category_id, other_category);
    assert_eq!(updated.installments.len(), 4);
    assert_eq!(
        updated
            .installments
            .iter()
            .map(|item| item.amount_cents)
            .sum::<i64>(),
        120_000
    );
    assert_eq!(updated.installments[0].due_on, "2035-02-15");
    assert_eq!(updated.description.as_deref(), Some("Compra revisada"));
    assert_eq!(recognized_expense(&fixture).await, 120_000);
    assert_eq!(
        fixture
            .cards
            .card_summary(&card_id)
            .await
            .expect("summary")
            .debt_cents,
        120_000
    );
    assert_eq!(net_worth(&fixture).await, -120_000);
    assert_eq!(
        scalar(
            &fixture,
            "SELECT COUNT(*) FROM journal_entries WHERE kind = 'expense' AND posted_at IS NOT NULL",
        )
        .await,
        4
    );
}

#[tokio::test]
async fn updating_a_paid_plan_allows_only_the_description() {
    let fixture = fixture().await;
    let bank_id = account(&fixture, "Banco", 200_000).await;
    let card_id = card(&fixture, "Cartao", 0, None).await;
    let category_id = category(&fixture, "Compras").await;

    let plan = fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            3,
            "2035-01-10",
        ))
        .await
        .expect("purchase should work");
    fixture
        .cards
        .create_payment(payment_input(&card_id, &bank_id, 30_000, "2026-02-01"))
        .await
        .expect("payment should work");

    let updated = fixture
        .cards
        .update_plan(
            &plan.id,
            UpdateInstallmentPlanInput {
                category_id,
                total_cents: 90_000,
                installment_count: 3,
                first_due_on: "2035-01-10".to_owned(),
                period: plan.period.clone(),
                description: Some("Nome novo".to_owned()),
            },
        )
        .await
        .expect("description should be updated");
    assert_eq!(updated.description.as_deref(), Some("Nome novo"));
    assert_eq!(updated.installments[0].paid_cents, 30_000);

    assert!(matches!(
        fixture
            .cards
            .update_plan(
                &plan.id,
                UpdateInstallmentPlanInput {
                    category_id: updated.category_id.clone(),
                    total_cents: 120_000,
                    installment_count: 3,
                    first_due_on: "2035-01-10".to_owned(),
                    period: plan.period.clone(),
                    description: None,
                },
            )
            .await,
        Err(AppError::Conflict { .. })
    ));
    assert_eq!(recognized_expense(&fixture).await, 90_000);
}

#[tokio::test]
async fn recognized_entries_stay_out_of_the_movements_list() {
    let fixture = fixture().await;
    let card_id = card(&fixture, "Cartao", 0, None).await;
    let category_id = category(&fixture, "Compras").await;

    fixture
        .cards
        .create_purchase(purchase_input(
            &card_id,
            &category_id,
            90_000,
            3,
            "2035-01-10",
        ))
        .await
        .expect("purchase should work");

    let movements = fixture.movements.list(None).await.expect("movements");
    assert_eq!(movements.len(), 1);
    assert_eq!(movements[0].kind, "card_purchase");
    assert_eq!(movements[0].category_id, None);
    assert_eq!(movements[0].amount_cents, -90_000);
    assert_eq!(movements[0].installment_count, Some(3));
    assert_eq!(movements[0].first_installment_cents, Some(30_000));
    assert!(movements[0].installment_plan_id.is_some());
    assert_eq!(
        scalar(
            &fixture,
            "SELECT COUNT(*) FROM journal_entries WHERE kind = 'expense'",
        )
        .await,
        3
    );
    assert_eq!(
        scalar(
            &fixture,
            "SELECT COUNT(*) FROM postings
             WHERE account_id = '00000000-0000-7000-8000-000000000003'",
        )
        .await,
        3
    );
}
