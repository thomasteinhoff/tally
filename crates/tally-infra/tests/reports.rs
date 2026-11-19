use chrono::{Datelike, Duration, Local, Months, NaiveDate};
use tally_app::{
    AccountsService, CardsService, CategoriesService, CreateAccountInput, CreateCardPaymentInput,
    CreateCardPurchaseInput, CreateCategoryInput, CreateExpenseInput, CreateIncomeInput,
    CreateTransferInput, MovementsService, ReportFilter, ReportsService,
};
use tally_infra::{
    Database, SqliteAccountsRepository, SqliteCardsRepository, SqliteCategoriesRepository,
    SqliteMovementsRepository, SqliteReportsRepository,
};
use tempfile::TempDir;

struct Fixture {
    _dir: TempDir,
    accounts: AccountsService<SqliteAccountsRepository>,
    categories: CategoriesService<SqliteCategoriesRepository>,
    cards: CardsService<SqliteCardsRepository>,
    movements: MovementsService<SqliteMovementsRepository>,
    reports: ReportsService<SqliteReportsRepository>,
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
        reports: ReportsService::new(SqliteReportsRepository::new(db)),
        _dir: dir,
    }
}

async fn account(fixture: &Fixture, name: &str, kind: &str, opening: i64) -> String {
    fixture
        .accounts
        .create(CreateAccountInput {
            name: name.to_owned(),
            account_type: kind.to_owned(),
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
            icon: String::new(),
        })
        .await
        .expect("category should be created")
        .id
}

async fn expense(
    fixture: &Fixture,
    account_id: &str,
    category_id: &str,
    amount_cents: i64,
    occurred_on: &str,
    occurred_time: &str,
) {
    fixture
        .movements
        .create_expense(CreateExpenseInput {
            account_id: account_id.to_owned(),
            category_id: category_id.to_owned(),
            amount_cents,
            occurred_on: occurred_on.to_owned(),
            occurred_time: Some(occurred_time.to_owned()),
            manual_period: None,
            description: Some("Despesa".to_owned()),
            payment_method: "pix".to_owned(),
        })
        .await
        .expect("expense should be created");
}

async fn expense_manual(
    fixture: &Fixture,
    account_id: &str,
    category_id: &str,
    amount_cents: i64,
    occurred_on: &str,
    manual_period: &str,
) {
    fixture
        .movements
        .create_expense(CreateExpenseInput {
            account_id: account_id.to_owned(),
            category_id: category_id.to_owned(),
            amount_cents,
            occurred_on: occurred_on.to_owned(),
            occurred_time: None,
            manual_period: Some(manual_period.to_owned()),
            description: Some("Despesa".to_owned()),
            payment_method: "pix".to_owned(),
        })
        .await
        .expect("expense with manual period should be created");
}

async fn income(
    fixture: &Fixture,
    account_id: &str,
    category_id: &str,
    amount_cents: i64,
    occurred_on: &str,
) {
    fixture
        .movements
        .create_income(CreateIncomeInput {
            account_id: account_id.to_owned(),
            category_id: category_id.to_owned(),
            amount_cents,
            occurred_on: occurred_on.to_owned(),
            occurred_time: Some("10:00".to_owned()),
            manual_period: None,
            description: Some("Receita".to_owned()),
            payment_method: "pix".to_owned(),
        })
        .await
        .expect("income should be created");
}

async fn transfer(
    fixture: &Fixture,
    from_account_id: &str,
    to_account_id: &str,
    amount_cents: i64,
    occurred_on: &str,
) {
    fixture
        .movements
        .create_transfer(CreateTransferInput {
            from_account_id: from_account_id.to_owned(),
            to_account_id: to_account_id.to_owned(),
            amount_cents,
            occurred_on: occurred_on.to_owned(),
            occurred_time: Some("11:00".to_owned()),
            manual_period: None,
            description: Some("Transferencia".to_owned()),
        })
        .await
        .expect("transfer should be created");
}

async fn purchase(
    fixture: &Fixture,
    card_id: &str,
    category_id: &str,
    total_cents: i64,
    count: i64,
    first_due_on: &str,
) {
    fixture
        .cards
        .create_purchase(CreateCardPurchaseInput {
            card_account_id: card_id.to_owned(),
            category_id: category_id.to_owned(),
            total_cents,
            installment_count: count,
            first_due_on: first_due_on.to_owned(),
            occurred_on: "2026-01-15".to_owned(),
            occurred_time: Some("12:00".to_owned()),
            manual_period: None,
            description: Some("Compra parcelada".to_owned()),
            payment_method: "credit".to_owned(),
        })
        .await
        .expect("purchase should be created");
}

async fn pay_card(
    fixture: &Fixture,
    card_id: &str,
    bank_id: &str,
    amount_cents: i64,
    occurred_on: &str,
) {
    fixture
        .cards
        .create_payment(CreateCardPaymentInput {
            card_account_id: card_id.to_owned(),
            bank_account_id: bank_id.to_owned(),
            amount_cents,
            occurred_on: occurred_on.to_owned(),
            occurred_time: Some("10:00".to_owned()),
            manual_period: None,
            description: Some("Pagamento".to_owned()),
        })
        .await
        .expect("payment should be created");
}

fn filter(from: &str, to: &str) -> ReportFilter {
    ReportFilter {
        from: Some(from.to_owned()),
        to: Some(to.to_owned()),
        period: None,
        category_id: None,
        account_id: None,
        payment_method: None,
    }
}

#[tokio::test]
async fn spending_and_income_group_by_category_and_apply_filters() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", "bank", 0).await;
    let mercado = category(&fixture, "Mercado", "expense").await;
    let lazer = category(&fixture, "Lazer", "expense").await;
    let salario = category(&fixture, "Salario", "income").await;

    expense(&fixture, &bank, &mercado, 5_000, "2026-02-03", "14:00").await;
    expense(&fixture, &bank, &mercado, 2_000, "2026-02-04", "09:00").await;
    expense(&fixture, &bank, &lazer, 3_000, "2026-02-05", "20:00").await;
    expense(&fixture, &bank, &mercado, 7_000, "2026-03-01", "14:00").await;
    income(&fixture, &bank, &salario, 9_000, "2026-02-06").await;

    let spending = fixture
        .reports
        .spending(filter("2026-02-01", "2026-02-28"))
        .await
        .expect("spending report");
    assert_eq!(spending.total_cents, 10_000);
    assert_eq!(spending.entry_count, 3);
    assert_eq!(spending.categories.len(), 2);
    assert_eq!(spending.categories[0].category_name, "Mercado");
    assert_eq!(spending.categories[0].total_cents, 7_000);
    assert_eq!(spending.categories[0].entry_count, 2);
    assert_eq!(spending.categories[1].category_name, "Lazer");

    let afternoon = fixture
        .reports
        .spending(ReportFilter {
            period: Some("tarde".to_owned()),
            ..filter("2026-02-01", "2026-02-28")
        })
        .await
        .expect("period filter");
    assert_eq!(afternoon.total_cents, 5_000);
    assert_eq!(afternoon.entry_count, 1);

    let only_lazer = fixture
        .reports
        .spending(ReportFilter {
            category_id: Some(lazer.clone()),
            ..filter("2026-02-01", "2026-02-28")
        })
        .await
        .expect("category filter");
    assert_eq!(only_lazer.total_cents, 3_000);

    let income_report = fixture
        .reports
        .income(filter("2026-02-01", "2026-02-28"))
        .await
        .expect("income report");
    assert_eq!(income_report.total_cents, 9_000);
    assert_eq!(income_report.entry_count, 1);
    assert_eq!(income_report.categories[0].category_name, "Salario");
}

#[tokio::test]
async fn spending_filters_by_account_and_payment_method_in_combination() {
    let fixture = fixture().await;
    let bank_a = account(&fixture, "Banco A", "bank", 0).await;
    let bank_b = account(&fixture, "Banco B", "bank", 0).await;
    let card = account(&fixture, "Cartao", "credit_card", 0).await;
    let mercado = category(&fixture, "Mercado", "expense").await;
    let compras = category(&fixture, "Compras", "expense").await;

    expense(&fixture, &bank_a, &mercado, 5_000, "2026-02-03", "14:00").await;
    expense(&fixture, &bank_b, &mercado, 2_000, "2026-02-04", "14:00").await;
    purchase(&fixture, &card, &compras, 30_000, 3, "2026-02-10").await;

    let only_bank_a = fixture
        .reports
        .spending(ReportFilter {
            account_id: Some(bank_a.clone()),
            ..filter("2026-02-01", "2026-02-28")
        })
        .await
        .expect("account filter");
    assert_eq!(only_bank_a.total_cents, 5_000);
    assert_eq!(only_bank_a.entry_count, 1);

    let only_card = fixture
        .reports
        .spending(ReportFilter {
            account_id: Some(card.clone()),
            ..filter("2026-02-01", "2026-02-28")
        })
        .await
        .expect("card account filter");
    assert_eq!(only_card.total_cents, 10_000);
    assert_eq!(only_card.entry_count, 1);
    assert_eq!(only_card.categories[0].category_name, "Compras");

    let only_credit = fixture
        .reports
        .spending(ReportFilter {
            payment_method: Some("credit".to_owned()),
            ..filter("2026-02-01", "2026-02-28")
        })
        .await
        .expect("method filter");
    assert_eq!(only_credit.total_cents, 10_000);

    let combined = fixture
        .reports
        .spending(ReportFilter {
            period: Some("tarde".to_owned()),
            category_id: Some(compras.clone()),
            account_id: Some(card.clone()),
            payment_method: Some("credit".to_owned()),
            ..filter("2026-02-01", "2026-02-28")
        })
        .await
        .expect("combined filters");
    assert_eq!(combined.total_cents, 10_000);
    assert_eq!(combined.entry_count, 1);

    let bank_b_pix = fixture
        .reports
        .spending(ReportFilter {
            account_id: Some(bank_b.clone()),
            payment_method: Some("pix".to_owned()),
            ..filter("2026-02-01", "2026-02-28")
        })
        .await
        .expect("combined bank filter");
    assert_eq!(bank_b_pix.total_cents, 2_000);
    assert_eq!(bank_b_pix.entry_count, 1);
}

#[tokio::test]
async fn cash_flow_filters_by_account_and_payment_method() {
    let fixture = fixture().await;
    let bank_a = account(&fixture, "Banco A", "bank", 0).await;
    let bank_b = account(&fixture, "Banco B", "bank", 0).await;
    let mercado = category(&fixture, "Mercado", "expense").await;
    let salario = category(&fixture, "Salario", "income").await;

    expense(&fixture, &bank_a, &mercado, 5_000, "2026-02-03", "14:00").await;
    expense(&fixture, &bank_b, &mercado, 2_000, "2026-02-04", "14:00").await;
    income(&fixture, &bank_a, &salario, 3_000, "2026-02-05").await;

    let bank_a_report = fixture
        .reports
        .cash_flow(ReportFilter {
            account_id: Some(bank_a.clone()),
            ..filter("2026-02-01", "2026-02-28")
        })
        .await
        .expect("account cash flow");
    assert_eq!(bank_a_report.inflow_cents, 3_000);
    assert_eq!(bank_a_report.outflow_cents, 5_000);
    assert_eq!(bank_a_report.net_cents, -2_000);

    let bank_b_report = fixture
        .reports
        .cash_flow(ReportFilter {
            account_id: Some(bank_b.clone()),
            payment_method: Some("pix".to_owned()),
            ..filter("2026-02-01", "2026-02-28")
        })
        .await
        .expect("combined cash flow");
    assert_eq!(bank_b_report.inflow_cents, 0);
    assert_eq!(bank_b_report.outflow_cents, 2_000);

    let credit_report = fixture
        .reports
        .cash_flow(ReportFilter {
            payment_method: Some("credit".to_owned()),
            ..filter("2026-02-01", "2026-02-28")
        })
        .await
        .expect("card method cash flow");
    assert_eq!(credit_report.inflow_cents, 0);
    assert_eq!(credit_report.outflow_cents, 0);
    assert!(credit_report.groups.is_empty());
}

#[tokio::test]
async fn installments_are_counted_by_due_date_not_by_card_purchase_total() {
    let fixture = fixture().await;
    let card = account(&fixture, "Cartao", "credit_card", 0).await;
    let compras = category(&fixture, "Compras", "expense").await;

    purchase(&fixture, &card, &compras, 30_000, 3, "2026-02-10").await;

    let january = fixture
        .reports
        .spending(filter("2026-01-01", "2026-01-31"))
        .await
        .expect("january");
    assert_eq!(january.total_cents, 0);
    assert_eq!(january.entry_count, 0);

    let february = fixture
        .reports
        .spending(filter("2026-02-01", "2026-02-28"))
        .await
        .expect("february");
    assert_eq!(february.total_cents, 10_000);
    assert_eq!(february.entry_count, 1);
    assert_eq!(february.categories[0].category_name, "Compras");
}

#[tokio::test]
async fn cash_flow_excludes_transfers_card_purchases_and_recognition_entries() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", "bank", 0).await;
    let savings = account(&fixture, "Poupanca", "savings", 0).await;
    let card = account(&fixture, "Cartao", "credit_card", 0).await;
    let mercado = category(&fixture, "Mercado", "expense").await;
    let salario = category(&fixture, "Salario", "income").await;
    let compras = category(&fixture, "Compras", "expense").await;

    fixture
        .movements
        .create_adjustment(tally_app::CreateAdjustmentInput {
            account_id: bank.clone(),
            amount_cents: 100_000,
            occurred_on: "2026-01-05".to_owned(),
            description: Some("Saldo inicial".to_owned()),
        })
        .await
        .expect("adjustment");
    expense(&fixture, &bank, &mercado, 5_000, "2026-02-03", "14:00").await;
    income(&fixture, &bank, &salario, 30_000, "2026-02-05").await;
    transfer(&fixture, &bank, &savings, 7_000, "2026-02-06").await;
    purchase(&fixture, &card, &compras, 30_000, 3, "2026-02-10").await;
    pay_card(&fixture, &card, &bank, 10_000, "2026-02-20").await;

    let report = fixture
        .reports
        .cash_flow(filter("2026-02-01", "2026-02-28"))
        .await
        .expect("cash flow");

    assert_eq!(report.inflow_cents, 30_000);
    assert_eq!(report.outflow_cents, 15_000);
    assert_eq!(report.net_cents, 15_000);
    assert_eq!(report.months.len(), 1);
    assert_eq!(report.months[0].month, "2026-02");
    assert_eq!(report.months[0].net_cents, 15_000);

    let labels = report
        .groups
        .iter()
        .map(|group| (group.label.as_str(), group.kind.as_str()))
        .collect::<Vec<_>>();
    assert!(labels.contains(&("Salario", "income")));
    assert!(labels.contains(&("Mercado", "expense")));
    assert!(labels.contains(&("Pagamento de cartão", "card_payment")));
    assert!(!labels.iter().any(|(label, _)| *label == "Transferencia"));
}

#[tokio::test]
async fn cash_flow_ignores_opening_balances() {
    let fixture = fixture().await;
    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
    account(&fixture, "Banco", "bank", 50_000).await;

    let report = fixture
        .reports
        .cash_flow(filter(&today, &today))
        .await
        .expect("cash flow");

    assert_eq!(report.inflow_cents, 0);
    assert_eq!(report.outflow_cents, 0);
    assert!(report.groups.is_empty());
}

#[tokio::test]
async fn net_worth_uses_assets_minus_liabilities_and_exposes_deferred_expense() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", "bank", 0).await;
    let savings = account(&fixture, "Poupanca", "savings", 0).await;
    let card = account(&fixture, "Cartao", "credit_card", 0).await;
    let mercado = category(&fixture, "Mercado", "expense").await;
    let salario = category(&fixture, "Salario", "income").await;
    let compras = category(&fixture, "Compras", "expense").await;

    fixture
        .movements
        .create_adjustment(tally_app::CreateAdjustmentInput {
            account_id: bank.clone(),
            amount_cents: 100_000,
            occurred_on: "2026-01-05".to_owned(),
            description: Some("Saldo inicial".to_owned()),
        })
        .await
        .expect("adjustment");
    expense(&fixture, &bank, &mercado, 5_000, "2026-02-03", "14:00").await;
    income(&fixture, &bank, &salario, 30_000, "2026-02-05").await;
    transfer(&fixture, &bank, &savings, 7_000, "2026-02-06").await;
    purchase(&fixture, &card, &compras, 30_000, 3, "2026-02-10").await;
    pay_card(&fixture, &card, &bank, 10_000, "2026-02-20").await;

    let report = fixture
        .reports
        .net_worth(Some("2026-03-31".to_owned()))
        .await
        .expect("net worth");

    assert_eq!(report.as_of, "2026-03-31");
    assert_eq!(report.assets_cents, 125_000);
    assert_eq!(report.liabilities_cents, 20_000);
    assert_eq!(report.net_worth_cents, 105_000);
    assert_eq!(report.deferred_expense_cents, 10_000);
    assert!(
        report
            .accounts
            .iter()
            .any(|account| account.name == "Banco")
    );
    assert!(
        report
            .accounts
            .iter()
            .any(|account| account.name == "Cartao")
    );
    assert!(
        !report
            .accounts
            .iter()
            .any(|account| account.name == "Despesa a reconhecer")
    );
}

#[tokio::test]
async fn commitments_only_include_unpaid_installments_after_the_reference_month() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", "bank", 0).await;
    let card = account(&fixture, "Cartao", "credit_card", 0).await;
    let compras = category(&fixture, "Compras", "expense").await;

    purchase(&fixture, &card, &compras, 30_000, 3, "2026-02-10").await;
    pay_card(&fixture, &card, &bank, 10_000, "2026-02-20").await;

    let report = fixture
        .reports
        .commitments(Some("2026-03-15".to_owned()), Some(2))
        .await
        .expect("commitments");

    assert_eq!(report.as_of, "2026-03-15");
    assert_eq!(report.start_after, "2026-03-31");
    assert_eq!(report.horizon_end, "2026-05-31");
    assert_eq!(report.overdue_cents, 10_000);
    assert_eq!(report.total_cents, 10_000);
    assert_eq!(report.months.len(), 1);
    assert_eq!(report.months[0].month, "2026-04");
    assert_eq!(report.months[0].items[0].outstanding_cents, 10_000);
    assert_eq!(report.months[0].items[0].seq, 3);
}

#[tokio::test]
async fn dashboard_composes_current_month_balances_and_commitments() {
    let fixture = fixture().await;
    let today = Local::now().date_naive();
    let today_text = today.format("%Y-%m-%d").to_string();
    let next_month = today
        .with_day(1)
        .and_then(|date| date.checked_add_months(Months::new(1)))
        .expect("next month");
    let first_due = (next_month + Duration::days(3))
        .format("%Y-%m-%d")
        .to_string();

    let bank = account(&fixture, "Banco", "bank", 0).await;
    let card = account(&fixture, "Cartao", "credit_card", 0).await;
    let mercado = category(&fixture, "Mercado", "expense").await;
    let salario = category(&fixture, "Salario", "income").await;
    let compras = category(&fixture, "Compras", "expense").await;

    expense(&fixture, &bank, &mercado, 4_000, &today_text, "14:00").await;
    income(&fixture, &bank, &salario, 9_000, &today_text).await;
    fixture
        .cards
        .create_purchase(CreateCardPurchaseInput {
            card_account_id: card,
            category_id: compras,
            total_cents: 30_000,
            installment_count: 3,
            first_due_on: first_due,
            occurred_on: today_text,
            occurred_time: Some("12:00".to_owned()),
            manual_period: None,
            description: Some("Compra".to_owned()),
            payment_method: "credit".to_owned(),
        })
        .await
        .expect("purchase");

    let dashboard = fixture.reports.dashboard().await.expect("dashboard");

    assert_eq!(dashboard.month_from, today.with_day(1).unwrap().to_string());
    assert_eq!(dashboard.month_income_cents, 9_000);
    assert_eq!(dashboard.month_expense_cents, 4_000);
    assert_eq!(dashboard.month_result_cents, 5_000);
    assert_eq!(dashboard.card_debt_cents, 30_000);
    assert_eq!(dashboard.overdue_installments_cents, 0);
    assert_eq!(dashboard.future_commitments_cents, 30_000);
    assert_eq!(dashboard.net_worth_cents, 5_000);
    assert_eq!(dashboard.top_expenses.len(), 1);
    assert_eq!(dashboard.top_expenses[0].total_cents, 4_000);
}

#[tokio::test]
async fn period_spending_counts_active_days_and_keeps_custom_separate() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", "bank", 0).await;
    let mercado = category(&fixture, "Mercado", "expense").await;
    let salario = category(&fixture, "Salario", "income").await;

    expense(&fixture, &bank, &mercado, 1_000, "2026-02-03", "14:00").await;
    expense(&fixture, &bank, &mercado, 2_000, "2026-02-03", "20:00").await;
    expense(&fixture, &bank, &mercado, 3_000, "2026-02-10", "09:00").await;
    expense_manual(&fixture, &bank, &mercado, 500, "2026-02-11", "custom").await;
    expense_manual(&fixture, &bank, &mercado, 500, "2026-02-12", "custom").await;
    income(&fixture, &bank, &salario, 9_000, "2026-02-03").await;

    let report = fixture
        .reports
        .period_spending(Some("2026-02-15".to_owned()))
        .await
        .expect("period spending");

    assert_eq!(report.from, "2026-02-01");
    assert_eq!(report.to, "2026-02-28");
    assert_eq!(report.total_cents, 7_000);
    assert_eq!(report.entry_count, 5);
    assert_eq!(report.days_with_expense, 4);
    assert_eq!(
        report
            .buckets
            .iter()
            .map(|bucket| bucket.period.as_str())
            .collect::<Vec<_>>(),
        ["madrugada", "manha", "tarde", "noite"]
    );

    let manha = &report.buckets[1];
    assert_eq!(manha.total_cents, 3_000);
    assert_eq!(manha.active_days, 1);
    assert_eq!(manha.average_cents, 3_000);
    assert_eq!(manha.frequency_percent, 25.0);

    let tarde = &report.buckets[2];
    assert_eq!(tarde.total_cents, 1_000);
    assert_eq!(tarde.average_cents, 1_000);

    let noite = &report.buckets[3];
    assert_eq!(noite.total_cents, 2_000);

    assert_eq!(report.buckets[0].total_cents, 0);
    assert_eq!(report.buckets[0].active_days, 0);
    assert_eq!(report.buckets[0].frequency_percent, 0.0);

    assert_eq!(report.unclassified.total_cents, 1_000);
    assert_eq!(report.unclassified.entry_count, 2);
    assert_eq!(report.unclassified.active_days, 2);
    assert_eq!(report.unclassified.average_cents, 500);
    assert_eq!(report.unclassified.frequency_percent, 50.0);

    let empty = fixture
        .reports
        .period_spending(Some("2026-03-10".to_owned()))
        .await
        .expect("empty month");
    assert_eq!(empty.total_cents, 0);
    assert_eq!(empty.days_with_expense, 0);
    assert!(empty.buckets.iter().all(|bucket| bucket.total_cents == 0));
    assert_eq!(empty.unclassified.total_cents, 0);
}

#[tokio::test]
async fn expense_trend_zero_fills_six_months_across_the_year_boundary() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", "bank", 0).await;
    let savings = account(&fixture, "Poupanca", "savings", 0).await;
    let mercado = category(&fixture, "Mercado", "expense").await;
    let salario = category(&fixture, "Salario", "income").await;

    expense(&fixture, &bank, &mercado, 1_000, "2025-11-10", "14:00").await;
    expense(&fixture, &bank, &mercado, 2_000, "2025-12-10", "14:00").await;
    expense(&fixture, &bank, &mercado, 3_000, "2026-01-10", "14:00").await;
    expense(&fixture, &bank, &mercado, 4_000, "2026-02-10", "14:00").await;
    income(&fixture, &bank, &salario, 9_000, "2026-02-10").await;
    transfer(&fixture, &bank, &savings, 5_000, "2026-02-11").await;

    let trend = fixture
        .reports
        .expense_trend(Some("2026-02-20".to_owned()))
        .await
        .expect("expense trend");

    assert_eq!(trend.from_month, "2025-09");
    assert_eq!(trend.to_month, "2026-02");
    assert_eq!(
        trend
            .months
            .iter()
            .map(|month| month.month.as_str())
            .collect::<Vec<_>>(),
        [
            "2025-09", "2025-10", "2025-11", "2025-12", "2026-01", "2026-02"
        ]
    );
    assert_eq!(
        trend
            .months
            .iter()
            .map(|month| month.total_cents)
            .collect::<Vec<_>>(),
        [0, 0, 1_000, 2_000, 3_000, 4_000]
    );
    assert_eq!(trend.months[5].entry_count, 1);
}

#[tokio::test]
async fn net_worth_history_has_six_monthly_points_ending_on_the_selected_month() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", "bank", 0).await;
    let mercado = category(&fixture, "Mercado", "expense").await;
    let salario = category(&fixture, "Salario", "income").await;

    fixture
        .movements
        .create_adjustment(tally_app::CreateAdjustmentInput {
            account_id: bank.clone(),
            amount_cents: 100_000,
            occurred_on: "2025-11-05".to_owned(),
            description: Some("Saldo inicial".to_owned()),
        })
        .await
        .expect("adjustment");
    expense(&fixture, &bank, &mercado, 10_000, "2025-12-10", "14:00").await;
    income(&fixture, &bank, &salario, 5_000, "2026-01-20").await;

    let history = fixture
        .reports
        .net_worth_history(Some("2026-03-10".to_owned()))
        .await
        .expect("net worth history");

    assert_eq!(history.from_month, "2025-10");
    assert_eq!(history.to_month, "2026-03");
    assert_eq!(history.points.len(), 6);
    assert_eq!(history.points[5].as_of, "2026-03-31");
    assert_eq!(
        history
            .points
            .iter()
            .map(|point| point.net_worth_cents)
            .collect::<Vec<_>>(),
        [0, 100_000, 90_000, 95_000, 95_000, 95_000]
    );
    assert_eq!(history.points[5].assets_cents, 95_000);
    assert_eq!(history.points[5].liabilities_cents, 0);
}

#[allow(dead_code)]
fn date(raw: &str) -> NaiveDate {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").expect("date")
}
