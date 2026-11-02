use tally_app::{
    AccountsService, AppError, CategoriesService, CreateAccountInput, CreateAdjustmentInput,
    CreateCategoryInput, CreateExpenseInput, CreateIncomeInput, CreateTransferInput,
    MovementFilter, MovementsRepository, MovementsService, NewMovement, UpdateMovementInput,
};
use tally_domain::ledger::Posting;
use tally_domain::money::Money;
use tally_domain::movement::MovementKind;
use tally_infra::{
    Database, SqliteAccountsRepository, SqliteCategoriesRepository, SqliteMovementsRepository,
};
use tempfile::TempDir;

const SYSTEM_INCOME: &str = "00000000-0000-7000-8000-000000000002";
const SYSTEM_EQUITY: &str = "00000000-0000-7000-8000-000000000001";

struct Fixture {
    _dir: TempDir,
    db: Database,
    accounts: AccountsService<SqliteAccountsRepository>,
    categories: CategoriesService<SqliteCategoriesRepository>,
    movements: MovementsService<SqliteMovementsRepository>,
}

async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("temp dir");
    let db = Database::open(dir.path().join("tally.db"))
        .await
        .expect("database should open");
    Fixture {
        _dir: dir,
        accounts: AccountsService::new(SqliteAccountsRepository::new(db.clone())),
        categories: CategoriesService::new(SqliteCategoriesRepository::new(db.clone())),
        movements: MovementsService::new(SqliteMovementsRepository::new(db.clone())),
        db,
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

async fn category(fixture: &Fixture, name: &str, kind: &str) -> String {
    fixture
        .categories
        .create(CreateCategoryInput {
            name: name.to_owned(),
            kind: kind.to_owned(),
        })
        .await
        .expect("category should be created")
        .id
}

fn expense_input(
    account_id: &str,
    category_id: &str,
    cents: i64,
    date: &str,
    time: Option<&str>,
    manual: Option<&str>,
    description: &str,
) -> CreateExpenseInput {
    CreateExpenseInput {
        account_id: account_id.to_owned(),
        category_id: category_id.to_owned(),
        amount_cents: cents,
        occurred_on: date.to_owned(),
        occurred_time: time.map(str::to_owned),
        manual_period: manual.map(str::to_owned),
        description: Some(description.to_owned()),
        payment_method: "pix".to_owned(),
    }
}

fn empty_filter() -> MovementFilter {
    MovementFilter {
        account_id: None,
        category_id: None,
        kind: None,
        period: None,
        from: None,
        to: None,
    }
}

async fn balance(fixture: &Fixture, id: &str) -> i64 {
    fixture
        .accounts
        .list(true)
        .await
        .expect("accounts")
        .into_iter()
        .find(|account| account.id == id)
        .expect("account should exist")
        .balance_cents
}

async fn total_postings(fixture: &Fixture) -> i64 {
    fixture
        .db
        .call(|connection| {
            connection.query_row(
                "SELECT COALESCE(SUM(amount_cents), 0) FROM postings",
                [],
                |row| row.get::<_, i64>(0),
            )
        })
        .await
        .expect("postings should sum")
}

async fn count_entries(fixture: &Fixture) -> i64 {
    fixture
        .db
        .call(|connection| {
            connection.query_row("SELECT COUNT(*) FROM journal_entries", [], |row| {
                row.get::<_, i64>(0)
            })
        })
        .await
        .expect("entries should count")
}

#[tokio::test]
async fn golden_phase_one_ledger_balances_and_derives_balances() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 100_000).await;
    let savings = account(&fixture, "Poupança", 0).await;
    let market = category(&fixture, "Mercado", "expense").await;
    let salary = category(&fixture, "Salário", "income").await;

    fixture
        .movements
        .create_expense(expense_input(
            &bank,
            &market,
            10_000,
            "2026-11-01",
            Some("12:00"),
            None,
            "Feira",
        ))
        .await
        .expect("expense");

    fixture
        .movements
        .create_income(CreateIncomeInput {
            account_id: bank.clone(),
            category_id: salary,
            amount_cents: 50_000,
            occurred_on: "2026-11-01".to_owned(),
            occurred_time: None,
            manual_period: Some("noite".to_owned()),
            description: Some("Pagamento".to_owned()),
            payment_method: "pix".to_owned(),
        })
        .await
        .expect("income");

    fixture
        .movements
        .create_transfer(CreateTransferInput {
            from_account_id: bank.clone(),
            to_account_id: savings.clone(),
            amount_cents: 20_000,
            occurred_on: "2026-11-02".to_owned(),
            occurred_time: Some("09:00".to_owned()),
            manual_period: None,
            description: Some("Reserva".to_owned()),
        })
        .await
        .expect("transfer");

    fixture
        .movements
        .create_adjustment(CreateAdjustmentInput {
            account_id: bank.clone(),
            amount_cents: -5_000,
            occurred_on: "2026-11-03".to_owned(),
            description: Some("Correção".to_owned()),
        })
        .await
        .expect("adjustment");

    assert_eq!(balance(&fixture, &bank).await, 115_000);
    assert_eq!(balance(&fixture, &savings).await, 20_000);
    assert_eq!(total_postings(&fixture).await, 0);
    assert_eq!(count_entries(&fixture).await, 5);

    let movements = fixture.movements.list(None).await.expect("movements");
    assert_eq!(movements.len(), 4);
    assert!(
        movements
            .iter()
            .all(|movement| movement.kind != "opening_balance"),
        "saldo inicial não pode aparecer na lista de movimentações"
    );

    let income_balance: i64 = fixture
        .db
        .call(|connection| {
            connection.query_row(
                "SELECT SUM(amount_cents) FROM postings WHERE account_id = ?1",
                [SYSTEM_INCOME],
                |row| row.get(0),
            )
        })
        .await
        .expect("income balance");
    assert_eq!(income_balance, -50_000);
}

#[tokio::test]
async fn transfer_is_neutral_and_appears_on_both_statements() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 100_000).await;
    let savings = account(&fixture, "Poupança", 0).await;
    let before = balance(&fixture, &bank).await + balance(&fixture, &savings).await;

    fixture
        .movements
        .create_transfer(CreateTransferInput {
            from_account_id: bank.clone(),
            to_account_id: savings.clone(),
            amount_cents: 40_000,
            occurred_on: "2026-11-01".to_owned(),
            occurred_time: Some("10:00".to_owned()),
            manual_period: None,
            description: Some("Transferência".to_owned()),
        })
        .await
        .expect("transfer");

    let after = balance(&fixture, &bank).await + balance(&fixture, &savings).await;
    assert_eq!(before, after);
    assert_eq!(balance(&fixture, &bank).await, 60_000);
    assert_eq!(balance(&fixture, &savings).await, 40_000);

    let bank_statement = fixture
        .movements
        .statement(&bank, None, None)
        .await
        .expect("bank statement");
    let savings_statement = fixture
        .movements
        .statement(&savings, None, None)
        .await
        .expect("savings statement");
    assert!(
        bank_statement
            .lines
            .iter()
            .any(|line| line.kind == "transfer" && line.amount_cents == -40_000)
    );
    assert!(
        savings_statement
            .lines
            .iter()
            .any(|line| line.kind == "transfer" && line.amount_cents == 40_000)
    );
}

#[tokio::test]
async fn derives_every_period_boundary_and_validates_manual_period() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 0).await;
    let market = category(&fixture, "Mercado", "expense").await;

    for (time, expected) in [
        ("00:00", "madrugada"),
        ("04:59", "madrugada"),
        ("05:00", "manha"),
        ("11:59", "manha"),
        ("12:00", "tarde"),
        ("18:59", "tarde"),
        ("19:00", "noite"),
        ("23:59", "noite"),
    ] {
        let movement = fixture
            .movements
            .create_expense(expense_input(
                &bank,
                &market,
                100,
                "2026-11-01",
                Some(time),
                None,
                time,
            ))
            .await
            .expect("expense with time");
        assert_eq!(movement.period.as_deref(), Some(expected), "horário {time}");
        assert_eq!(movement.occurred_time.as_deref(), Some(time));
        assert!(movement.manual_period.is_none());
    }

    let manual = fixture
        .movements
        .create_expense(expense_input(
            &bank,
            &market,
            100,
            "2026-11-01",
            None,
            Some("custom"),
            "manual",
        ))
        .await
        .expect("manual period");
    assert_eq!(manual.period.as_deref(), Some("custom"));
    assert_eq!(manual.manual_period.as_deref(), Some("custom"));

    let both = fixture
        .movements
        .create_expense(expense_input(
            &bank,
            &market,
            100,
            "2026-11-01",
            Some("12:00"),
            Some("tarde"),
            "inválido",
        ))
        .await;
    assert!(matches!(both, Err(AppError::Validation { .. })));

    let neither = fixture
        .movements
        .create_expense(expense_input(
            &bank,
            &market,
            100,
            "2026-11-01",
            None,
            None,
            "inválido",
        ))
        .await;
    assert!(matches!(neither, Err(AppError::Validation { .. })));

    let card = CreateExpenseInput {
        payment_method: "credit".to_owned(),
        ..expense_input(
            &bank,
            &market,
            100,
            "2026-11-01",
            Some("12:00"),
            None,
            "cartão",
        )
    };
    assert!(matches!(
        fixture.movements.create_expense(card).await,
        Err(AppError::Validation { .. })
    ));
}

#[tokio::test]
async fn archived_category_preserves_history_and_blocks_new_use() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 10_000).await;
    let market = category(&fixture, "Mercado", "expense").await;

    fixture
        .movements
        .create_expense(expense_input(
            &bank,
            &market,
            1_000,
            "2026-11-01",
            Some("12:00"),
            None,
            "Compra",
        ))
        .await
        .expect("expense");

    fixture
        .categories
        .set_archived(&market, true)
        .await
        .expect("archive category");

    let movements = fixture.movements.list(None).await.expect("movements");
    assert_eq!(movements[0].category_name.as_deref(), Some("Mercado"));

    assert!(matches!(
        fixture
            .movements
            .create_expense(expense_input(
                &bank,
                &market,
                1_000,
                "2026-11-02",
                Some("12:00"),
                None,
                "Nova",
            ))
            .await,
        Err(AppError::Validation { .. })
    ));
    assert!(matches!(
        fixture.categories.delete(&market).await,
        Err(AppError::Conflict { .. })
    ));

    fixture
        .categories
        .set_archived(&market, false)
        .await
        .expect("restore category");
    assert_eq!(
        fixture.categories.list(false).await.expect("active").len(),
        1
    );
}

#[tokio::test]
async fn update_movement_is_atomic_and_recalculates_balance() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 100_000).await;
    let savings = account(&fixture, "Poupança", 0).await;
    let market = category(&fixture, "Mercado", "expense").await;

    let movement = fixture
        .movements
        .create_expense(expense_input(
            &bank,
            &market,
            10_000,
            "2026-11-01",
            Some("04:59"),
            None,
            "Antiga",
        ))
        .await
        .expect("expense");
    assert_eq!(movement.period.as_deref(), Some("madrugada"));

    let updated = fixture
        .movements
        .update(
            &movement.id,
            UpdateMovementInput {
                kind: "expense".to_owned(),
                primary_account_id: bank.clone(),
                counterparty_account_id: None,
                category_id: Some(market.clone()),
                amount_cents: 25_000,
                occurred_on: "2026-11-02".to_owned(),
                occurred_time: Some("12:00".to_owned()),
                manual_period: None,
                description: Some("Nova".to_owned()),
                payment_method: Some("debit".to_owned()),
            },
        )
        .await
        .expect("update should work");

    assert_eq!(updated.amount_cents, -25_000);
    assert_eq!(updated.period.as_deref(), Some("tarde"));
    assert_eq!(balance(&fixture, &bank).await, 75_000);
    assert_eq!(total_postings(&fixture).await, 0);

    let movement_id = movement.id.clone();
    let postings: i64 = fixture
        .db
        .call(move |connection| {
            connection.query_row(
                "SELECT COUNT(*) FROM postings WHERE entry_id = ?1",
                [&movement_id],
                |row| row.get(0),
            )
        })
        .await
        .expect("postings");
    assert_eq!(postings, 2, "edição não pode duplicar postings");

    let transfer_id = fixture
        .movements
        .create_transfer(CreateTransferInput {
            from_account_id: bank.clone(),
            to_account_id: savings.clone(),
            amount_cents: 5_000,
            occurred_on: "2026-11-03".to_owned(),
            occurred_time: Some("10:00".to_owned()),
            manual_period: None,
            description: None,
        })
        .await
        .expect("transfer")
        .id;

    let updated_transfer = fixture
        .movements
        .update(
            &transfer_id,
            UpdateMovementInput {
                kind: "transfer".to_owned(),
                primary_account_id: bank.clone(),
                counterparty_account_id: Some(savings.clone()),
                category_id: None,
                amount_cents: 7_000,
                occurred_on: "2026-11-04".to_owned(),
                occurred_time: None,
                manual_period: Some("manha".to_owned()),
                description: Some("Volta".to_owned()),
                payment_method: None,
            },
        )
        .await
        .expect("transfer update");
    assert_eq!(updated_transfer.amount_cents, -7_000);
    assert_eq!(updated_transfer.period.as_deref(), Some("manha"));
    assert_eq!(balance(&fixture, &bank).await, 68_000);
    assert_eq!(balance(&fixture, &savings).await, 7_000);

    assert!(matches!(
        fixture
            .movements
            .update(
                &movement.id,
                UpdateMovementInput {
                    kind: "income".to_owned(),
                    primary_account_id: bank.clone(),
                    counterparty_account_id: None,
                    category_id: Some(market.clone()),
                    amount_cents: 100,
                    occurred_on: "2026-11-02".to_owned(),
                    occurred_time: Some("12:00".to_owned()),
                    manual_period: None,
                    description: None,
                    payment_method: Some("pix".to_owned()),
                },
            )
            .await,
        Err(AppError::Validation { .. })
    ));
}

#[tokio::test]
async fn delete_movement_removes_postings_and_cannot_touch_opening_balance() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 10_000).await;
    let market = category(&fixture, "Mercado", "expense").await;

    let movement = fixture
        .movements
        .create_expense(expense_input(
            &bank,
            &market,
            2_000,
            "2026-11-01",
            Some("12:00"),
            None,
            "Compra",
        ))
        .await
        .expect("expense");

    fixture
        .movements
        .delete(&movement.id)
        .await
        .expect("delete");
    assert_eq!(balance(&fixture, &bank).await, 10_000);
    assert_eq!(count_entries(&fixture).await, 1);
    assert_eq!(fixture.movements.list(None).await.expect("list").len(), 0);

    let opening_id: String = fixture
        .db
        .call(|connection| {
            connection.query_row(
                "SELECT id FROM journal_entries WHERE kind = 'opening_balance'",
                [],
                |row| row.get(0),
            )
        })
        .await
        .expect("opening entry");
    assert!(matches!(
        fixture.movements.delete(&opening_id).await,
        Err(AppError::Conflict { .. })
    ));
}

#[tokio::test]
async fn statement_tracks_running_balance_and_date_filter() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 100_000).await;
    let market = category(&fixture, "Mercado", "expense").await;

    for (date, cents, description) in [
        ("2027-01-10", 10_000, "Primeira"),
        ("2027-01-20", 15_000, "Segunda"),
    ] {
        fixture
            .movements
            .create_expense(expense_input(
                &bank,
                &market,
                cents,
                date,
                Some("12:00"),
                None,
                description,
            ))
            .await
            .expect("expense");
    }

    let all = fixture
        .movements
        .statement(&bank, None, None)
        .await
        .expect("statement");
    assert_eq!(all.account.balance_cents, 75_000);
    assert_eq!(all.opening_balance_cents, 0);
    assert_eq!(all.closing_balance_cents, 75_000);
    assert_eq!(all.lines.len(), 3);
    assert_eq!(all.lines[0].kind, "opening_balance");
    assert_eq!(all.lines[0].running_balance_cents, 100_000);
    assert_eq!(all.lines[1].category_id.as_deref(), Some(market.as_str()));
    assert_eq!(all.lines[1].running_balance_cents, 90_000);
    assert_eq!(all.lines[2].running_balance_cents, 75_000);

    let filtered = fixture
        .movements
        .statement(&bank, Some("2027-01-20"), Some("2027-01-31"))
        .await
        .expect("filtered statement");
    assert_eq!(filtered.opening_balance_cents, 90_000);
    assert_eq!(filtered.lines.len(), 1);
    assert_eq!(filtered.lines[0].amount_cents, -15_000);
    assert_eq!(filtered.closing_balance_cents, 75_000);

    assert!(matches!(
        fixture.movements.statement("missing", None, None).await,
        Err(AppError::NotFound { .. })
    ));
    assert!(matches!(
        fixture
            .movements
            .statement(&bank, Some("2027-02-01"), Some("2027-01-01"))
            .await,
        Err(AppError::Validation { .. })
    ));
}

#[tokio::test]
async fn list_applies_account_category_kind_period_and_date_filters() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 100_000).await;
    let savings = account(&fixture, "Poupança", 0).await;
    let market = category(&fixture, "Mercado", "expense").await;
    let salary = category(&fixture, "Salário", "income").await;

    fixture
        .movements
        .create_expense(expense_input(
            &bank,
            &market,
            1_000,
            "2027-01-10",
            Some("12:00"),
            None,
            "Mercado",
        ))
        .await
        .expect("expense");
    fixture
        .movements
        .create_income(CreateIncomeInput {
            account_id: bank.clone(),
            category_id: salary,
            amount_cents: 5_000,
            occurred_on: "2027-02-10".to_owned(),
            occurred_time: None,
            manual_period: Some("manha".to_owned()),
            description: Some("Salário".to_owned()),
            payment_method: "pix".to_owned(),
        })
        .await
        .expect("income");
    fixture
        .movements
        .create_transfer(CreateTransferInput {
            from_account_id: bank.clone(),
            to_account_id: savings.clone(),
            amount_cents: 2_000,
            occurred_on: "2027-03-10".to_owned(),
            occurred_time: Some("19:00".to_owned()),
            manual_period: None,
            description: Some("Transferência".to_owned()),
        })
        .await
        .expect("transfer");

    let by_account = fixture
        .movements
        .list(Some(MovementFilter {
            account_id: Some(savings.clone()),
            ..empty_filter()
        }))
        .await
        .expect("account filter");
    assert_eq!(by_account.len(), 1);
    assert_eq!(by_account[0].kind, "transfer");

    let by_category = fixture
        .movements
        .list(Some(MovementFilter {
            category_id: Some(market.clone()),
            ..empty_filter()
        }))
        .await
        .expect("category filter");
    assert_eq!(by_category.len(), 1);

    let by_kind = fixture
        .movements
        .list(Some(MovementFilter {
            kind: Some("income".to_owned()),
            ..empty_filter()
        }))
        .await
        .expect("kind filter");
    assert_eq!(by_kind.len(), 1);
    assert_eq!(by_kind[0].period.as_deref(), Some("manha"));

    let by_period = fixture
        .movements
        .list(Some(MovementFilter {
            period: Some("noite".to_owned()),
            ..empty_filter()
        }))
        .await
        .expect("period filter");
    assert_eq!(by_period.len(), 1);

    let by_date = fixture
        .movements
        .list(Some(MovementFilter {
            from: Some("2027-02-01".to_owned()),
            to: Some("2027-02-28".to_owned()),
            ..empty_filter()
        }))
        .await
        .expect("date filter");
    assert_eq!(by_date.len(), 1);
    assert_eq!(by_date[0].category_name.as_deref(), Some("Salário"));

    assert!(matches!(
        fixture
            .movements
            .list(Some(MovementFilter {
                from: Some("2027-03-01".to_owned()),
                to: Some("2027-02-01".to_owned()),
                ..empty_filter()
            }))
            .await,
        Err(AppError::Validation { .. })
    ));
}

#[tokio::test]
async fn failed_update_rolls_back_unpost_and_posting_changes() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 100_000).await;
    let market = category(&fixture, "Mercado", "expense").await;

    let movement = fixture
        .movements
        .create_expense(expense_input(
            &bank,
            &market,
            10_000,
            "2027-01-10",
            Some("12:00"),
            None,
            "Original",
        ))
        .await
        .expect("expense");

    let repository = SqliteMovementsRepository::new(fixture.db.clone());
    let invalid = NewMovement {
        id: movement.id.clone(),
        kind: MovementKind::Expense,
        occurred_on: "2027-01-11".to_owned(),
        occurred_time: Some("12:00".to_owned()),
        manual_period: None,
        description: Some("Inválida".to_owned()),
        primary_account_id: bank.clone(),
        counterparty_account_id: None,
        category_id: Some(market.clone()),
        payment_method: Some(tally_domain::movement::PaymentMethod::Pix),
        postings: vec![
            Posting::new("posting-a", bank.clone(), Money::from_cents(-20_000)),
            Posting::new("posting-b", "missing-account", Money::from_cents(20_000)),
        ],
    };

    assert!(repository.update(&movement.id, invalid).await.is_err());
    assert_eq!(balance(&fixture, &bank).await, 90_000);
    assert_eq!(total_postings(&fixture).await, 0);

    let listed = fixture.movements.list(None).await.expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].amount_cents, -10_000);
    assert_eq!(listed[0].description.as_deref(), Some("Original"));

    let movement_id = movement.id.clone();
    let posted: i64 = fixture
        .db
        .call(move |connection| {
            connection.query_row(
                "SELECT COUNT(*) FROM journal_entries WHERE id = ?1 AND posted_at IS NOT NULL",
                [&movement_id],
                |row| row.get(0),
            )
        })
        .await
        .expect("posted");
    assert_eq!(posted, 1);
}

#[tokio::test]
async fn adjustment_changes_equity_without_becoming_income_or_expense() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 10_000).await;

    fixture
        .movements
        .create_adjustment(CreateAdjustmentInput {
            account_id: bank.clone(),
            amount_cents: -3_000,
            occurred_on: "2027-01-10".to_owned(),
            description: Some("Correção de saldo".to_owned()),
        })
        .await
        .expect("adjustment");

    assert_eq!(balance(&fixture, &bank).await, 7_000);

    let (income, expense, equity): (i64, i64, i64) = fixture
        .db
        .call(|connection| {
            let balance = |account_id: &str| {
                connection.query_row(
                    "SELECT COALESCE(SUM(amount_cents), 0) FROM postings WHERE account_id = ?1",
                    [account_id],
                    |row| row.get::<_, i64>(0),
                )
            };

            Ok((
                balance(SYSTEM_INCOME)?,
                balance("00000000-0000-7000-8000-000000000003")?,
                balance(SYSTEM_EQUITY)?,
            ))
        })
        .await
        .expect("system balances");
    assert_eq!(income, 0);
    assert_eq!(expense, 0);
    assert_eq!(equity, -7_000);

    let movement = fixture.movements.list(None).await.expect("list").remove(0);
    assert_eq!(movement.kind, "adjustment");
    assert_eq!(movement.amount_cents, -3_000);
}

#[tokio::test]
async fn opening_balance_is_not_listed_as_income() {
    let fixture = fixture().await;
    account(&fixture, "Banco", 100_000).await;

    let movements = fixture.movements.list(None).await.expect("list");
    assert!(movements.is_empty());

    let income_balance: i64 = fixture
        .db
        .call(|connection| {
            connection.query_row(
                "SELECT COALESCE(SUM(amount_cents), 0) FROM postings WHERE account_id = ?1",
                [SYSTEM_INCOME],
                |row| row.get(0),
            )
        })
        .await
        .expect("income balance");
    assert_eq!(income_balance, 0);
}

#[tokio::test]
async fn category_duplicates_are_conflicts_and_kind_is_immutable() {
    let fixture = fixture().await;
    let market = category(&fixture, "Mercado", "expense").await;

    assert!(matches!(
        fixture
            .categories
            .create(CreateCategoryInput {
                name: "mercado".to_owned(),
                kind: "expense".to_owned(),
            })
            .await,
        Err(AppError::Conflict { .. })
    ));

    let renamed = fixture
        .categories
        .rename(&market, "  Supermercado  ")
        .await
        .expect("rename");
    assert_eq!(renamed.name, "Supermercado");
    assert_eq!(renamed.kind, "expense");

    fixture
        .categories
        .delete(&market)
        .await
        .expect("delete fresh category");
    assert!(
        fixture
            .categories
            .list(true)
            .await
            .expect("list")
            .is_empty()
    );
}
