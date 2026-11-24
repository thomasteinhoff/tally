use chrono::Local;
use rusqlite::{OptionalExtension, params};
use tally_app::{
    AccountsService, AppError, CategoriesService, CreateAccountInput, CreateAdjustmentInput,
    CreateCardPaymentInput, CreateCategoryInput, CreateExpenseInput, CreateIncomeInput,
    CreateSavingsYieldInput, CreateTransferInput, MovementFilter, MovementsService, ReportFilter,
    ReportsService, ReverseSavingsYieldInput, SavingsService, UpdateMovementInput,
};
use tally_infra::{
    Database, SqliteAccountsRepository, SqliteCardsRepository, SqliteCategoriesRepository,
    SqliteMovementsRepository, SqliteReportsRepository, SqliteSavingsRepository,
};
use tempfile::TempDir;

struct Fixture {
    _dir: TempDir,
    db: Database,
    accounts: AccountsService<SqliteAccountsRepository>,
    categories: CategoriesService<SqliteCategoriesRepository>,
    cards: tally_app::CardsService<SqliteCardsRepository>,
    movements: MovementsService<SqliteMovementsRepository>,
    reports: ReportsService<SqliteReportsRepository>,
    savings: SavingsService<SqliteSavingsRepository>,
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
        cards: tally_app::CardsService::new(SqliteCardsRepository::new(db.clone())),
        movements: MovementsService::new(SqliteMovementsRepository::new(db.clone())),
        reports: ReportsService::new(SqliteReportsRepository::new(db.clone())),
        savings: SavingsService::new(SqliteSavingsRepository::new(db.clone())),
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
            is_savings: None,
            yield_mode: None,
            cdi_percent: None,
        })
        .await
        .expect("account should be created")
        .id
}

async fn savings(fixture: &Fixture, name: &str, mode: &str, percent: f64) -> String {
    fixture
        .accounts
        .create(CreateAccountInput {
            name: name.to_owned(),
            account_type: "savings".to_owned(),
            opening_balance_cents: 0,
            credit_limit_cents: None,
            is_savings: Some(true),
            yield_mode: Some(mode.to_owned()),
            cdi_percent: Some(percent),
        })
        .await
        .expect("savings should be created")
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

async fn transfer(fixture: &Fixture, from: &str, to: &str, amount_cents: i64, occurred_on: &str) {
    fixture
        .movements
        .create_transfer(CreateTransferInput {
            from_account_id: from.to_owned(),
            to_account_id: to.to_owned(),
            amount_cents,
            occurred_on: occurred_on.to_owned(),
            occurred_time: Some("12:00".to_owned()),
            manual_period: None,
            description: None,
        })
        .await
        .expect("transfer should be created");
}

async fn yield_at(fixture: &Fixture, account_id: &str, as_of: &str) -> tally_app::SavingsYieldDto {
    fixture
        .savings
        .create_yield(CreateSavingsYieldInput {
            account_id: account_id.to_owned(),
            as_of: as_of.to_owned(),
        })
        .await
        .expect("yield should be created")
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

fn movement_filter() -> MovementFilter {
    MovementFilter {
        account_id: None,
        category_id: None,
        kind: None,
        period: None,
        payment_method: None,
        from: None,
        to: None,
    }
}

fn report_filter() -> ReportFilter {
    ReportFilter {
        from: Some("2026-11-01".to_owned()),
        to: Some("2026-11-30".to_owned()),
        period: None,
        category_id: None,
        account_id: None,
        payment_method: None,
    }
}

#[tokio::test]
async fn yield_uses_beginning_of_day_balance_and_stays_out_of_movements() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;

    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;
    transfer(&fixture, &bank, &pot, 50_000, "2026-11-03").await;
    let result = yield_at(&fixture, &pot, "2026-11-03").await;

    assert_eq!(result.base_cents, 100_000);
    assert_eq!(result.mode, "calendar");
    assert_eq!(result.contracted_percent_bps, 10_000);
    assert!(result.cdi_annual_bps > 0);
    assert!(result.daily_rate > 0.0);
    assert_eq!(result.days, 1);
    assert!(result.yield_cents > 0);
    assert_eq!(result.status, "confirmed");
    assert_eq!(balance(&fixture, &pot).await, 150_000 + result.yield_cents);

    let movements = fixture
        .movements
        .list(Some(movement_filter()))
        .await
        .expect("movements");
    assert!(movements.iter().all(|item| item.id != result.entry_id));
    assert_eq!(movements.len(), 2);
}

#[tokio::test]
async fn consecutive_yields_use_each_day_starting_balance_and_preserve_snapshots() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 130.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;

    let first = yield_at(&fixture, &pot, "2026-11-03").await;
    assert_eq!(first.contracted_percent_bps, 13_000);
    assert_eq!(first.base_cents, 100_000);

    // Aplicação no dia 04 entra na base do dia 05, não na do dia 03.
    transfer(&fixture, &bank, &pot, 50_000, "2026-11-04").await;
    let second = yield_at(&fixture, &pot, "2026-11-05").await;
    assert_eq!(second.base_cents, 100_000 + first.yield_cents + 50_000);
    assert!(second.yield_cents > first.yield_cents);

    let history = fixture
        .savings
        .overview(&pot)
        .await
        .expect("overview")
        .yields;
    assert_eq!(history.len(), 2);
    let stored = history
        .iter()
        .find(|item| item.id == first.id)
        .expect("primeiro snapshot");
    assert_eq!(stored.base_cents, first.base_cents);
    assert_eq!(stored.yield_cents, first.yield_cents);
    assert_eq!(stored.daily_rate, first.daily_rate);
    assert_eq!(stored.days, 1);
}

#[tokio::test]
async fn reversal_preserves_original_and_allows_same_day_replacement() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 115.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;

    let original = yield_at(&fixture, &pot, "2026-11-03").await;
    yield_at(&fixture, &pot, "2026-11-04").await;

    let reversal = fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: original.id.clone(),
            as_of: "2026-11-05".to_owned(),
        })
        .await
        .expect("reversal should work");
    assert_eq!(reversal.status, "reversed");
    assert!(reversal.reversal_entry_id.is_some());

    let original_entry_posted: Option<String> = fixture
        .db
        .call(move |connection| {
            connection
                .query_row(
                    "SELECT posted_at FROM journal_entries WHERE id = ?1",
                    [original.entry_id.clone()],
                    |row| row.get(0),
                )
                .optional()
        })
        .await
        .expect("entry")
        .flatten();
    assert!(original_entry_posted.is_some());

    let replacement = yield_at(&fixture, &pot, "2026-11-03").await;
    assert_eq!(replacement.status, "confirmed");
    assert_ne!(replacement.id, original.id);

    let duplicate = fixture
        .savings
        .create_yield(CreateSavingsYieldInput {
            account_id: pot.clone(),
            as_of: "2026-11-03".to_owned(),
        })
        .await;
    assert!(matches!(
        duplicate,
        Err(AppError::Validation { .. }) | Err(AppError::Conflict { .. })
    ));

    let history = fixture
        .savings
        .overview(&pot)
        .await
        .expect("overview")
        .yields;
    assert_eq!(history.len(), 3);
    assert_eq!(
        history
            .iter()
            .filter(|item| item.status == "confirmed" && item.as_of == "2026-11-03")
            .count(),
        1
    );
}

#[tokio::test]
async fn reversal_is_rejected_when_the_yield_was_already_withdrawn() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;
    let result = yield_at(&fixture, &pot, "2026-11-03").await;

    let withdrawn = balance(&fixture, &pot).await;
    transfer(&fixture, &pot, &bank, withdrawn, "2026-11-04").await;
    assert_eq!(balance(&fixture, &pot).await, 0);

    let postings_before: i64 = fixture
        .db
        .call(|connection| {
            connection.query_row("SELECT COUNT(*) FROM postings", [], |row| row.get(0))
        })
        .await
        .expect("count");

    let rejected = fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: result.id,
            as_of: "2026-11-05".to_owned(),
        })
        .await;
    assert!(matches!(rejected, Err(AppError::Validation { .. })));

    let postings_after: i64 = fixture
        .db
        .call(|connection| {
            connection.query_row("SELECT COUNT(*) FROM postings", [], |row| row.get(0))
        })
        .await
        .expect("count");
    assert_eq!(postings_after, postings_before);
}

#[tokio::test]
async fn withdrawal_cannot_make_the_savings_balance_negative() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;

    let rejected = fixture
        .movements
        .create_transfer(CreateTransferInput {
            from_account_id: pot.clone(),
            to_account_id: bank,
            amount_cents: 100_001,
            occurred_on: "2026-11-03".to_owned(),
            occurred_time: Some("12:00".to_owned()),
            manual_period: None,
            description: None,
        })
        .await;
    assert!(matches!(rejected, Err(AppError::Validation { .. })));
    assert_eq!(balance(&fixture, &pot).await, 100_000);
}

#[tokio::test]
async fn invalid_operations_on_savings_are_blocked() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho A", "calendar", 100.0).await;
    let other_pot = savings(&fixture, "Cofrinho B", "business", 100.0).await;
    let expense_category = category(&fixture, "Mercado", "expense").await;
    let income_category = category(&fixture, "Salário", "income").await;

    let invalid_expense = fixture
        .movements
        .create_expense(CreateExpenseInput {
            account_id: pot.clone(),
            category_id: expense_category,
            amount_cents: 100,
            occurred_on: "2026-11-03".to_owned(),
            occurred_time: Some("12:00".to_owned()),
            manual_period: None,
            description: None,
            payment_method: "pix".to_owned(),
        })
        .await;
    assert!(matches!(invalid_expense, Err(AppError::Validation { .. })));

    let invalid_income = fixture
        .movements
        .create_income(CreateIncomeInput {
            account_id: pot.clone(),
            category_id: income_category,
            amount_cents: 100,
            occurred_on: "2026-11-03".to_owned(),
            occurred_time: Some("12:00".to_owned()),
            manual_period: None,
            description: None,
            payment_method: "pix".to_owned(),
        })
        .await;
    assert!(matches!(invalid_income, Err(AppError::Validation { .. })));

    let invalid_transfer = fixture
        .movements
        .create_transfer(CreateTransferInput {
            from_account_id: pot.clone(),
            to_account_id: other_pot,
            amount_cents: 100,
            occurred_on: "2026-11-03".to_owned(),
            occurred_time: Some("12:00".to_owned()),
            manual_period: None,
            description: None,
        })
        .await;
    assert!(matches!(invalid_transfer, Err(AppError::Validation { .. })));

    let card = account(&fixture, "Cartão", 0).await;
    let card_for_update = card.clone();
    fixture
        .db
        .call(move |connection| {
            connection.execute(
                "UPDATE accounts SET class = 'liability', type = 'credit_card',
                 credit_limit_cents = 100000 WHERE id = ?1",
                params![card_for_update],
            )
        })
        .await
        .expect("card account update");
    let invalid_payment = fixture
        .cards
        .create_payment(CreateCardPaymentInput {
            card_account_id: card,
            bank_account_id: pot.clone(),
            amount_cents: 100,
            occurred_on: "2026-11-03".to_owned(),
            occurred_time: Some("12:00".to_owned()),
            manual_period: None,
            description: None,
        })
        .await;
    assert!(matches!(invalid_payment, Err(AppError::Validation { .. })));

    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;
    assert_eq!(balance(&fixture, &pot).await, 100_000);
}

#[tokio::test]
async fn savings_transfers_do_not_change_income_expense_or_cash_flow() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    let expense_category = category(&fixture, "Mercado", "expense").await;
    let income_category = category(&fixture, "Salário", "income").await;

    fixture
        .movements
        .create_expense(CreateExpenseInput {
            account_id: bank.clone(),
            category_id: expense_category,
            amount_cents: 1_000,
            occurred_on: "2026-11-02".to_owned(),
            occurred_time: Some("12:00".to_owned()),
            manual_period: None,
            description: None,
            payment_method: "pix".to_owned(),
        })
        .await
        .expect("expense");
    fixture
        .movements
        .create_income(CreateIncomeInput {
            account_id: bank.clone(),
            category_id: income_category,
            amount_cents: 5_000,
            occurred_on: "2026-11-02".to_owned(),
            occurred_time: Some("12:00".to_owned()),
            manual_period: None,
            description: None,
            payment_method: "pix".to_owned(),
        })
        .await
        .expect("income");

    let spending_before = fixture
        .reports
        .spending(report_filter())
        .await
        .expect("spending")
        .total_cents;
    let income_before = fixture
        .reports
        .income(report_filter())
        .await
        .expect("income")
        .total_cents;
    let flow_before = fixture
        .reports
        .cash_flow(report_filter())
        .await
        .expect("cash flow")
        .net_cents;

    transfer(&fixture, &bank, &pot, 100_000, "2026-11-03").await;
    transfer(&fixture, &pot, &bank, 20_000, "2026-11-04").await;
    fixture
        .movements
        .create_adjustment(CreateAdjustmentInput {
            account_id: pot.clone(),
            amount_cents: 10_000,
            occurred_on: "2026-11-05".to_owned(),
            description: None,
        })
        .await
        .expect("positive adjustment");
    fixture
        .movements
        .create_adjustment(CreateAdjustmentInput {
            account_id: pot.clone(),
            amount_cents: -5_000,
            occurred_on: "2026-11-06".to_owned(),
            description: None,
        })
        .await
        .expect("negative adjustment");
    yield_at(&fixture, &pot, "2026-11-07").await;

    assert_eq!(
        fixture
            .reports
            .spending(report_filter())
            .await
            .expect("spending")
            .total_cents,
        spending_before
    );
    assert_eq!(
        fixture
            .reports
            .income(report_filter())
            .await
            .expect("income")
            .total_cents,
        income_before
    );
    assert_eq!(
        fixture
            .reports
            .cash_flow(report_filter())
            .await
            .expect("cash flow")
            .net_cents,
        flow_before
    );
}

#[tokio::test]
async fn net_worth_separates_savings_keeps_archived_and_matches_dashboard() {
    let fixture = fixture().await;
    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
    let bank = account(&fixture, "Banco", 500_000).await;
    let first = savings(&fixture, "Cofrinho A", "calendar", 100.0).await;
    let second = savings(&fixture, "Cofrinho B", "business", 115.0).await;

    transfer(&fixture, &bank, &first, 100_000, &today).await;
    transfer(&fixture, &bank, &second, 50_000, &today).await;
    fixture
        .accounts
        .set_archived(&first, true)
        .await
        .expect("archive should work");

    let net_worth = fixture
        .reports
        .net_worth(Some(today.clone()))
        .await
        .expect("net worth");
    assert_eq!(net_worth.net_worth_cents, 350_000);
    assert_eq!(net_worth.assets_cents, 350_000);
    assert_eq!(net_worth.savings_cents, 150_000);
    assert_eq!(net_worth.total_with_savings_cents, 500_000);
    assert!(
        net_worth
            .accounts
            .iter()
            .all(|item| item.account_id != first)
    );

    let history = fixture
        .reports
        .net_worth_history(Some(today.clone()))
        .await
        .expect("history");
    let last = history.points.last().expect("last point");
    assert_eq!(last.net_worth_cents, 350_000);
    assert_eq!(last.savings_cents, 150_000);
    assert_eq!(last.total_with_savings_cents, 500_000);

    let dashboard = fixture.reports.dashboard().await.expect("dashboard");
    assert_eq!(dashboard.net_worth_cents, net_worth.net_worth_cents);
    assert_eq!(dashboard.savings_cents, net_worth.savings_cents);
    assert_eq!(
        dashboard.total_with_savings_cents,
        net_worth.total_with_savings_cents
    );
}

#[tokio::test]
async fn savings_opening_balance_is_persisted_and_stays_out_of_net_worth() {
    let fixture = fixture().await;
    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
    let pot = fixture
        .accounts
        .create(CreateAccountInput {
            name: "Cofrinho com saldo".to_owned(),
            account_type: "savings".to_owned(),
            opening_balance_cents: 100_000,
            credit_limit_cents: None,
            is_savings: Some(true),
            yield_mode: Some("calendar".to_owned()),
            cdi_percent: Some(100.0),
        })
        .await
        .expect("savings should be created")
        .id;

    assert_eq!(balance(&fixture, &pot).await, 100_000);

    let pot_id = pot.clone();
    let (entries, kind, pot_cents, equity_cents) = fixture
        .db
        .call(move |connection| {
            let entries: i64 =
                connection
                    .query_row("SELECT COUNT(*) FROM journal_entries", [], |row| row.get(0))?;
            let kind: String = connection.query_row(
                "SELECT kind FROM journal_entries WHERE primary_account_id = ?1",
                [&pot_id],
                |row| row.get(0),
            )?;
            let pot_cents: i64 = connection.query_row(
                "SELECT COALESCE(SUM(amount_cents), 0) FROM postings WHERE account_id = ?1",
                [&pot_id],
                |row| row.get(0),
            )?;
            let equity_cents: i64 = connection.query_row(
                "SELECT COALESCE(SUM(amount_cents), 0) FROM postings
                 WHERE account_id = '00000000-0000-7000-8000-000000000001'",
                [],
                |row| row.get(0),
            )?;
            Ok((entries, kind, pot_cents, equity_cents))
        })
        .await
        .expect("ledger should be readable");

    assert_eq!(entries, 1);
    assert_eq!(kind, "opening_balance");
    assert_eq!(pot_cents, 100_000);
    assert_eq!(equity_cents, -100_000);

    let net_worth = fixture
        .reports
        .net_worth(Some(today))
        .await
        .expect("net worth");
    assert_eq!(net_worth.net_worth_cents, 0);
    assert_eq!(net_worth.savings_cents, 100_000);
    assert_eq!(net_worth.total_with_savings_cents, 100_000);
    assert!(net_worth.accounts.iter().all(|item| item.account_id != pot));
}

#[tokio::test]
async fn migration_0005_repairs_legacy_trigger_for_savings_opening_balance() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("legacy.db");

    {
        let connection = rusqlite::Connection::open(&path).expect("legacy db");
        connection
            .execute_batch(include_str!("../migrations/0001_init.sql"))
            .expect("schema v1");
        connection
            .execute_batch(include_str!("../migrations/0002_cards.sql"))
            .expect("schema v2");
        connection
            .execute_batch(include_str!("../migrations/0003_category_icons.sql"))
            .expect("schema v3");
        connection
            .execute_batch(include_str!("../migrations/0004_savings.sql"))
            .expect("schema v4");
        connection
            .execute_batch("PRAGMA user_version = 4;")
            .expect("legacy version");

        connection
            .execute_batch(
                "INSERT INTO accounts (
                    id, name, class, type, is_system, credit_limit_cents,
                    is_savings, yield_mode, cdi_percent_bps, archived_at,
                    created_at, updated_at
                 ) VALUES (
                    'legacy-pot', 'Cofrinho legado', 'asset', 'savings', 0, NULL,
                    1, 'calendar', 10000, NULL,
                    '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'
                 );",
            )
            .expect("legacy account");
        let error = connection
            .execute_batch(
                "INSERT INTO journal_entries (
                    id, kind, occurred_on, occurred_time, manual_period, description,
                    primary_account_id, counterparty_account_id, category_id,
                    payment_method, posted_at, created_at, updated_at
                 ) VALUES (
                    'legacy-opening', 'opening_balance', '2026-01-01', NULL, NULL,
                    'Saldo inicial', 'legacy-pot', NULL, NULL,
                    NULL, NULL, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'
                 );",
            )
            .expect_err("legacy trigger should reject the opening balance");
        assert!(
            error
                .to_string()
                .contains("savings accounts only accept transfers with a regular account"),
            "unexpected SQLite error: {error}"
        );
    }

    let db = Database::open(&path)
        .await
        .expect("migration 0005 should apply");
    let (version, trigger_sql): (i64, String) = db
        .call(|connection| {
            let version = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
            let trigger_sql = connection.query_row(
                "SELECT sql FROM sqlite_master
                 WHERE type = 'trigger' AND name = 'trg_savings_entry_kind'",
                [],
                |row| row.get(0),
            )?;
            Ok((version, trigger_sql))
        })
        .await
        .expect("schema should be readable");
    assert_eq!(version, 6);
    assert!(trigger_sql.contains("NEW.kind <> 'opening_balance'"));
    assert!(trigger_sql.contains("NEW.kind <> 'adjustment'"));

    let accounts = AccountsService::new(SqliteAccountsRepository::new(db.clone()));
    let pot = accounts
        .create(CreateAccountInput {
            name: "Cofrinho migrado".to_owned(),
            account_type: "savings".to_owned(),
            opening_balance_cents: 100_000,
            credit_limit_cents: None,
            is_savings: Some(true),
            yield_mode: Some("calendar".to_owned()),
            cdi_percent: Some(100.0),
        })
        .await
        .expect("opening balance should work after migration");
    assert_eq!(pot.balance_cents, 100_000);

    let pot_id = pot.id.clone();
    let (entries, kind, pot_cents, equity_cents) = db
        .call(move |connection| {
            let entries: i64 =
                connection
                    .query_row("SELECT COUNT(*) FROM journal_entries", [], |row| row.get(0))?;
            let kind: String = connection.query_row(
                "SELECT kind FROM journal_entries WHERE primary_account_id = ?1",
                [&pot_id],
                |row| row.get(0),
            )?;
            let pot_cents: i64 = connection.query_row(
                "SELECT COALESCE(SUM(amount_cents), 0) FROM postings WHERE account_id = ?1",
                [&pot_id],
                |row| row.get(0),
            )?;
            let equity_cents: i64 = connection.query_row(
                "SELECT COALESCE(SUM(amount_cents), 0) FROM postings
                 WHERE account_id = '00000000-0000-7000-8000-000000000001'",
                [],
                |row| row.get(0),
            )?;
            Ok((entries, kind, pot_cents, equity_cents))
        })
        .await
        .expect("ledger should be readable");

    assert_eq!(entries, 1);
    assert_eq!(kind, "opening_balance");
    assert_eq!(pot_cents, 100_000);
    assert_eq!(equity_cents, -100_000);
}

#[tokio::test]
async fn business_mode_rejects_weekends_and_generates_on_business_days() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "business", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-18").await;

    let weekend = fixture
        .savings
        .create_yield(CreateSavingsYieldInput {
            account_id: pot.clone(),
            as_of: "2026-11-21".to_owned(),
        })
        .await;
    assert!(matches!(weekend, Err(AppError::Validation { .. })));

    let holiday = fixture
        .savings
        .create_yield(CreateSavingsYieldInput {
            account_id: pot.clone(),
            as_of: "2026-12-25".to_owned(),
        })
        .await;
    assert!(matches!(holiday, Err(AppError::Validation { .. })));

    let black_consciousness_day = fixture
        .savings
        .create_yield(CreateSavingsYieldInput {
            account_id: pot.clone(),
            as_of: "2026-11-20".to_owned(),
        })
        .await;
    assert!(matches!(
        black_consciousness_day,
        Err(AppError::Validation { .. })
    ));

    let business_day = yield_at(&fixture, &pot, "2026-11-19").await;
    assert_eq!(business_day.mode, "business");
}

#[tokio::test]
async fn yield_entries_cannot_be_edited_or_deleted_as_generic_movements() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;
    let result = yield_at(&fixture, &pot, "2026-11-03").await;

    let input = UpdateMovementInput {
        kind: "adjustment".to_owned(),
        primary_account_id: pot.clone(),
        counterparty_account_id: None,
        category_id: None,
        amount_cents: result.yield_cents,
        occurred_on: "2026-11-03".to_owned(),
        occurred_time: None,
        manual_period: None,
        description: None,
        payment_method: None,
    };
    let edited = fixture.movements.update(&result.entry_id, input).await;
    assert!(matches!(edited, Err(AppError::Conflict { .. })));

    let deleted = fixture.movements.delete(&result.entry_id).await;
    assert!(matches!(deleted, Err(AppError::Conflict { .. })));
}

#[tokio::test]
async fn persistence_blocks_invalid_savings_entries_even_outside_the_service() {
    let fixture = fixture().await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    let entry_id = "20000000-0000-7000-8000-000000000001";

    let rejected = fixture
        .db
        .call(move |connection| {
            connection.execute(
                "INSERT INTO journal_entries (
                    id, kind, occurred_on, description, primary_account_id,
                    posted_at, created_at, updated_at
                 ) VALUES (?1, 'expense', '2026-11-03', 'invalido', ?2, NULL,
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                    strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                params![entry_id, pot],
            )
        })
        .await;
    assert!(rejected.is_err());
}

#[tokio::test]
async fn savings_accepts_positive_and_negative_adjustments_and_blocks_negative_balance() {
    let fixture = fixture().await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;

    let positive = fixture
        .movements
        .create_adjustment(CreateAdjustmentInput {
            account_id: pot.clone(),
            amount_cents: 100_000,
            occurred_on: "2026-11-02".to_owned(),
            description: Some("Correção positiva".to_owned()),
        })
        .await
        .expect("positive adjustment");
    assert_eq!(positive.kind, "adjustment");
    assert_eq!(balance(&fixture, &pot).await, 100_000);

    let negative = fixture
        .movements
        .create_adjustment(CreateAdjustmentInput {
            account_id: pot.clone(),
            amount_cents: -40_000,
            occurred_on: "2026-11-03".to_owned(),
            description: None,
        })
        .await
        .expect("negative adjustment");
    assert_eq!(negative.kind, "adjustment");
    assert_eq!(balance(&fixture, &pot).await, 60_000);

    let excessive = fixture
        .movements
        .create_adjustment(CreateAdjustmentInput {
            account_id: pot.clone(),
            amount_cents: -60_001,
            occurred_on: "2026-11-04".to_owned(),
            description: None,
        })
        .await;
    assert!(matches!(excessive, Err(AppError::Validation { .. })));
    assert_eq!(balance(&fixture, &pot).await, 60_000);

    let movements = fixture
        .movements
        .list(Some(movement_filter()))
        .await
        .expect("movements");
    assert_eq!(movements.len(), 2);
    assert!(movements.iter().any(|item| item.id == positive.id));
    assert!(movements.iter().any(|item| item.id == negative.id));
}

#[tokio::test]
async fn savings_adjustments_keep_conventional_edit_and_delete() {
    let fixture = fixture().await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    let adjustment = fixture
        .movements
        .create_adjustment(CreateAdjustmentInput {
            account_id: pot.clone(),
            amount_cents: 100_000,
            occurred_on: "2026-11-02".to_owned(),
            description: None,
        })
        .await
        .expect("adjustment");

    let updated = fixture
        .movements
        .update(
            &adjustment.id,
            UpdateMovementInput {
                kind: "adjustment".to_owned(),
                primary_account_id: pot.clone(),
                counterparty_account_id: None,
                category_id: None,
                amount_cents: 70_000,
                occurred_on: "2026-11-02".to_owned(),
                occurred_time: None,
                manual_period: None,
                description: None,
                payment_method: None,
            },
        )
        .await
        .expect("adjustment should be editable");
    assert_eq!(updated.amount_cents, 70_000);
    assert_eq!(balance(&fixture, &pot).await, 70_000);

    fixture
        .movements
        .delete(&adjustment.id)
        .await
        .expect("adjustment should be deletable");
    assert_eq!(balance(&fixture, &pot).await, 0);
}

#[tokio::test]
async fn savings_with_only_opening_balance_can_be_deleted() {
    let fixture = fixture().await;
    let pot = fixture
        .accounts
        .create(CreateAccountInput {
            name: "Cofrinho só saldo inicial".to_owned(),
            account_type: "savings".to_owned(),
            opening_balance_cents: 100_000,
            credit_limit_cents: None,
            is_savings: Some(true),
            yield_mode: Some("calendar".to_owned()),
            cdi_percent: Some(100.0),
        })
        .await
        .expect("savings should be created")
        .id;

    fixture
        .accounts
        .delete(&pot)
        .await
        .expect("savings with only opening balance should be deletable");

    assert!(
        fixture
            .accounts
            .list(true)
            .await
            .expect("accounts")
            .iter()
            .all(|account| account.id != pot)
    );
    let entries: i64 = fixture
        .db
        .call(|connection| {
            connection.query_row("SELECT COUNT(*) FROM journal_entries", [], |row| row.get(0))
        })
        .await
        .expect("entries");
    assert_eq!(entries, 0);
}

#[tokio::test]
async fn savings_with_history_requires_archiving_and_keeps_yield_snapshots() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;

    let rejected = fixture.accounts.delete(&pot).await;
    let message = match rejected {
        Err(AppError::Conflict { message }) => message,
        other => panic!("delete deveria conflitar: {other:?}"),
    };
    assert!(message.contains("arquive"), "mensagem: {message}");

    fixture
        .movements
        .create_adjustment(CreateAdjustmentInput {
            account_id: pot.clone(),
            amount_cents: 10_000,
            occurred_on: "2026-11-03".to_owned(),
            description: None,
        })
        .await
        .expect("adjustment");
    assert!(matches!(
        fixture.accounts.delete(&pot).await,
        Err(AppError::Conflict { .. })
    ));

    let yield_result = yield_at(&fixture, &pot, "2026-11-04").await;
    assert!(matches!(
        fixture.accounts.delete(&pot).await,
        Err(AppError::Conflict { .. })
    ));

    let history = fixture
        .savings
        .overview(&pot)
        .await
        .expect("overview")
        .yields;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].id, yield_result.id);
    assert_eq!(history[0].yield_cents, yield_result.yield_cents);
    assert_eq!(
        balance(&fixture, &pot).await,
        110_000 + yield_result.yield_cents
    );
}
