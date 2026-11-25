use async_trait::async_trait;
use chrono::{Datelike, Duration, Local};
use rusqlite::{OptionalExtension, params};
use tally_app::savings::{YieldMode, calculate_yield};
use tally_app::{
    AccountsService, AppError, CategoriesService, CdiRate, CdiRateSnapshot, CdiRatesRepository,
    CdiStatusDto, CreateAccountInput, CreateAdjustmentInput, CreateCardPaymentInput,
    CreateCategoryInput, CreateExpenseInput, CreateIncomeInput, CreateSavingsYieldInput,
    CreateTransferInput, MovementFilter, MovementsService, ReplaceSavingsYieldInput, ReportFilter,
    ReportsService, ReverseSavingsYieldInput, SavingsService, UpdateMovementInput, movements_csv,
};
use tally_infra::{
    Database, SqliteAccountsRepository, SqliteCardsRepository, SqliteCategoriesRepository,
    SqliteCdiRepository, SqliteMovementsRepository, SqliteReportsRepository,
    SqliteSavingsRepository,
};
use tempfile::TempDir;

/// Fonte de taxas fixa para os testes: uma publicação em 19/11/2026 serve
/// tanto à modalidade business (data exata) quanto à calendar (última anterior).
struct FixedCdi;

#[async_trait]
impl CdiRatesRepository for FixedCdi {
    async fn status(&self) -> Result<CdiStatusDto, AppError> {
        unreachable!("status do CDI não é usado nos testes de cofrinho")
    }

    async fn info(&self) -> Result<tally_app::CdiCacheInfo, AppError> {
        unreachable!("info do CDI não é usado nos testes de cofrinho")
    }

    async fn replace(
        &self,
        _series: &str,
        _rates: Vec<tally_app::CdiRate>,
        _fetched_at: &str,
    ) -> Result<(), AppError> {
        unreachable!("refresh do CDI não é usado nos testes de cofrinho")
    }

    async fn record_failure(&self, _attempted_at: &str, _error: &str) -> Result<(), AppError> {
        unreachable!("falha do CDI não é usada nos testes de cofrinho")
    }

    async fn rate_on_or_before(
        &self,
        _on_or_before: &str,
    ) -> Result<Option<CdiRateSnapshot>, AppError> {
        Ok(Some(rate()))
    }

    async fn rate_on(&self, _date: &str) -> Result<Option<CdiRateSnapshot>, AppError> {
        Ok(Some(rate()))
    }
}

fn rate() -> CdiRateSnapshot {
    CdiRateSnapshot {
        rate_date: "2026-11-19".to_owned(),
        daily_rate: 0.00050788,
        fetched_at: "2026-11-23T00:00:00Z".to_owned(),
    }
}

fn cdi_service(fixture: &Fixture) -> SavingsService<SqliteSavingsRepository, SqliteCdiRepository> {
    SavingsService::new(
        SqliteSavingsRepository::new(fixture.db.clone()),
        SqliteCdiRepository::new(fixture.db.clone()),
    )
}

async fn seed_rates(fixture: &Fixture, rates: &[(&str, f64)]) {
    let values = rates
        .iter()
        .map(|(date, daily_rate)| CdiRate {
            rate_date: (*date).to_owned(),
            raw_value: (daily_rate * 100.0).to_string(),
            daily_rate: *daily_rate,
        })
        .collect();
    SqliteCdiRepository::new(fixture.db.clone())
        .replace("12", values, "2026-11-23T00:00:00+00:00")
        .await
        .expect("taxas.should.be.seeded");
}

struct Fixture {
    _dir: TempDir,
    db: Database,
    accounts: AccountsService<SqliteAccountsRepository>,
    categories: CategoriesService<SqliteCategoriesRepository>,
    cards: tally_app::CardsService<SqliteCardsRepository>,
    movements: MovementsService<SqliteMovementsRepository>,
    reports: ReportsService<SqliteReportsRepository>,
    savings: SavingsService<SqliteSavingsRepository, FixedCdi>,
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
        savings: SavingsService::new(SqliteSavingsRepository::new(db.clone()), FixedCdi),
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

/// Simula um rendimento criado antes do bloqueio de data futura, inclusive sem
/// a provenância da taxa que passou a ser gravada nas versões seguintes.
async fn mark_yield_as_legacy_future(fixture: &Fixture, id: &str, as_of: &str) {
    let id = id.to_owned();
    let as_of = as_of.to_owned();
    fixture
        .db
        .call(move |connection| {
            connection.execute(
                "UPDATE savings_yields
                 SET as_of = ?1, rate_date = NULL, rate_series = NULL,
                     cdi_daily_rate = NULL, rate_fetched_at = NULL
                 WHERE id = ?2",
                params![as_of, id],
            )
        })
        .await
        .expect("legacy future yield");
}

async fn entry_occurred_on(fixture: &Fixture, entry_id: &str) -> String {
    let entry_id = entry_id.to_owned();
    fixture
        .db
        .call(move |connection| {
            connection.query_row(
                "SELECT occurred_on FROM journal_entries WHERE id = ?1",
                [entry_id],
                |row| row.get(0),
            )
        })
        .await
        .expect("entry")
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
async fn yield_uses_balance_through_date_and_appears_in_movements() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;

    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;
    transfer(&fixture, &bank, &pot, 50_000, "2026-11-03").await;
    let result = yield_at(&fixture, &pot, "2026-11-03").await;

    assert_eq!(result.base_cents, 150_000);
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
    assert_eq!(movements.len(), 3);
    let listed = movements
        .iter()
        .find(|item| item.id == result.entry_id)
        .expect("rendimento na listagem");
    assert_eq!(listed.kind, "savings_yield");
    assert!(listed.primary_account_is_savings);
    assert!(!listed.counterparty_account_is_savings);
    assert_eq!(listed.amount_cents, result.yield_cents);
}

#[tokio::test]
async fn savings_operations_are_classified_filtered_and_exported_once() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;

    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;
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
    let yield_result = yield_at(&fixture, &pot, "2026-11-04").await;
    let reversal = fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: yield_result.id.clone(),
            as_of: "2026-11-05".to_owned(),
        })
        .await
        .expect("reversal");

    let movements = fixture
        .movements
        .list(Some(movement_filter()))
        .await
        .expect("movements");
    assert_eq!(movements.len(), 4);

    let applied = movements
        .iter()
        .find(|item| item.kind == "transfer")
        .expect("application");
    assert!(!applied.primary_account_is_savings);
    assert!(applied.counterparty_account_is_savings);
    let adjustment = movements
        .iter()
        .find(|item| item.kind == "adjustment")
        .expect("adjustment");
    assert!(adjustment.primary_account_is_savings);

    let listed_yield = movements
        .iter()
        .find(|item| item.kind == "savings_yield")
        .expect("yield");
    assert_eq!(listed_yield.id, yield_result.entry_id);
    assert!(listed_yield.primary_account_is_savings);
    let listed_reversal = movements
        .iter()
        .find(|item| item.kind == "savings_yield_reversal")
        .expect("reversal entry");
    assert_eq!(
        listed_reversal.id,
        reversal
            .reversal_entry_id
            .clone()
            .expect("reversal entry id")
    );

    for (kind, expected) in [
        ("savings_yield", 1),
        ("savings_yield_reversal", 1),
        ("adjustment", 1),
        ("transfer", 1),
    ] {
        let filtered = fixture
            .movements
            .list(Some(MovementFilter {
                kind: Some(kind.to_owned()),
                ..movement_filter()
            }))
            .await
            .expect("filtered movements");
        assert_eq!(filtered.len(), expected, "filtro {kind}");
        assert!(filtered.iter().all(|item| item.kind == kind));
    }

    let from_savings = fixture
        .movements
        .list(Some(MovementFilter {
            account_id: Some(pot.clone()),
            ..movement_filter()
        }))
        .await
        .expect("savings movements");
    assert_eq!(from_savings.len(), 4);

    let failed_edit = fixture
        .movements
        .update(
            &yield_result.entry_id,
            UpdateMovementInput {
                kind: "adjustment".to_owned(),
                primary_account_id: pot.clone(),
                counterparty_account_id: None,
                category_id: None,
                amount_cents: 10_000,
                occurred_on: "2026-11-04".to_owned(),
                occurred_time: None,
                manual_period: None,
                description: None,
                payment_method: None,
            },
        )
        .await;
    assert!(matches!(failed_edit, Err(AppError::Conflict { .. })));
    assert!(matches!(
        fixture.movements.delete(&yield_result.entry_id).await,
        Err(AppError::Conflict { .. })
    ));

    let csv = movements_csv(&movements);
    assert_eq!(csv.lines().count(), 5);
    assert!(csv.contains(";Rendimento de cofrinho;"));
    assert!(csv.contains(";Estorno de rendimento;"));
    for movement in &movements {
        assert_eq!(csv.matches(&movement.id).count(), 1, "CSV duplicou a linha");
    }
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
async fn reversal_preserves_original_and_replacement_requires_last_of_day() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 115.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;

    let first = yield_at(&fixture, &pot, "2026-11-03").await;
    let second = yield_at(&fixture, &pot, "2026-11-03").await;
    assert_eq!(second.base_cents, 100_000 + first.yield_cents);

    let rejected = fixture
        .savings
        .replace_yield(ReplaceSavingsYieldInput {
            yield_id: first.id.clone(),
        })
        .await;
    assert!(matches!(rejected, Err(AppError::Conflict { .. })));

    let reversal = fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: first.id.clone(),
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
                    [first.entry_id.clone()],
                    |row| row.get(0),
                )
                .optional()
        })
        .await
        .expect("entry")
        .flatten();
    assert!(original_entry_posted.is_some());

    let last = yield_at(&fixture, &pot, "2026-11-03").await;
    let replacement = fixture
        .savings
        .replace_yield(ReplaceSavingsYieldInput {
            yield_id: last.id.clone(),
        })
        .await
        .expect("replacement");
    assert_ne!(replacement.id, last.id);
    assert_eq!(replacement.as_of, last.as_of);
    assert_eq!(replacement.base_cents, last.base_cents);

    let history = fixture
        .savings
        .overview(&pot)
        .await
        .expect("overview")
        .yields;
    assert_eq!(history.len(), 4);
    let stored_second = history
        .iter()
        .find(|item| item.id == second.id)
        .expect("segundo snapshot");
    assert_eq!(stored_second.status, "confirmed");
    assert_eq!(stored_second.base_cents, second.base_cents);
    assert_eq!(stored_second.yield_cents, second.yield_cents);
    assert_eq!(
        history
            .iter()
            .filter(|item| item.status == "confirmed" && item.as_of == "2026-11-03")
            .count(),
        2
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
    let message = match rejected {
        Err(AppError::Validation { message }) => message,
        other => panic!("estorno deveria ser rejeitado: {other:?}"),
    };
    assert!(message.contains("já foi resgatado"), "{message}");

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
    assert_eq!(version, 8);
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
async fn business_requires_a_rate_published_on_the_exact_date() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "business", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-17").await;
    seed_rates(&fixture, &[("2026-11-19", 0.00050788)]).await;
    let service = cdi_service(&fixture);

    let missing = service
        .create_yield(CreateSavingsYieldInput {
            account_id: pot.clone(),
            as_of: "2026-11-18".to_owned(),
        })
        .await
        .expect_err("data útil sem publicação não pode render");
    assert!(
        missing
            .to_string()
            .contains("não há taxa CDI publicada para esta data"),
        "{missing}"
    );

    let created = service
        .create_yield(CreateSavingsYieldInput {
            account_id: pot,
            as_of: "2026-11-19".to_owned(),
        })
        .await
        .expect("taxa publicada na data");
    assert_eq!(created.rate_date.as_deref(), Some("2026-11-19"));
    assert_eq!(created.rate_series.as_deref(), Some("12"));
    assert_eq!(created.cdi_daily_rate, Some(0.00050788));
    assert!(created.rate_fetched_at.is_some());
}

#[tokio::test]
async fn calendar_uses_the_latest_rate_on_or_before_as_of() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-15").await;
    seed_rates(
        &fixture,
        &[("2026-11-16", 0.0004), ("2026-11-19", 0.00050788)],
    )
    .await;

    let created = cdi_service(&fixture)
        .create_yield(CreateSavingsYieldInput {
            account_id: pot,
            as_of: "2026-11-21".to_owned(),
        })
        .await
        .expect("calendar aceita data sem publicação");
    assert_eq!(created.rate_date.as_deref(), Some("2026-11-19"));
    assert_eq!(created.cdi_daily_rate, Some(0.00050788));
}

#[tokio::test]
async fn legacy_yield_snapshots_without_rate_provenance_remain_readable() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;
    let created = yield_at(&fixture, &pot, "2026-11-03").await;
    let id = created.id.clone();

    fixture
        .db
        .call(move |connection| {
            connection.execute(
                "UPDATE savings_yields
                 SET rate_date = NULL, rate_series = NULL,
                     cdi_daily_rate = NULL, rate_fetched_at = NULL
                 WHERE id = ?1",
                [id],
            )
        })
        .await
        .expect("snapshot legado");

    let history = fixture
        .savings
        .overview(&pot)
        .await
        .expect("overview")
        .yields;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].id, created.id);
    assert_eq!(history[0].rate_date, None);
    assert_eq!(history[0].rate_series, None);
    assert_eq!(history[0].cdi_daily_rate, None);
    assert_eq!(history[0].rate_fetched_at, None);
}

#[tokio::test]
async fn legacy_future_yield_can_be_reversed_on_its_original_date() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;
    let original = yield_at(&fixture, &pot, "2026-11-03").await;
    let tomorrow = (Local::now().date_naive() + Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    mark_yield_as_legacy_future(&fixture, &original.id, &tomorrow).await;

    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
    let reversal = fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: original.id.clone(),
            as_of: today,
        })
        .await
        .expect("estorno de rendimento legado");
    assert_eq!(reversal.status, "reversed");
    assert_eq!(reversal.as_of, tomorrow);

    let reversal_entry = reversal
        .reversal_entry_id
        .clone()
        .expect("entry do estorno");
    assert_eq!(entry_occurred_on(&fixture, &reversal_entry).await, tomorrow);
    assert_eq!(
        entry_occurred_on(&fixture, &original.entry_id).await,
        "2026-11-03"
    );

    let history = fixture
        .savings
        .overview(&pot)
        .await
        .expect("overview")
        .yields;
    assert_eq!(history.len(), 1);
    let stored = history
        .iter()
        .find(|item| item.id == original.id)
        .expect("snapshot original");
    assert_eq!(stored.as_of, tomorrow);
    assert_eq!(stored.status, "reversed");
    assert_eq!(
        stored.reversal_entry_id.as_deref(),
        Some(reversal_entry.as_str())
    );
    assert_eq!(stored.entry_id, original.entry_id);
    assert_eq!(stored.base_cents, original.base_cents);
    assert_eq!(stored.yield_cents, original.yield_cents);
    assert_eq!(stored.daily_rate, original.daily_rate);
    assert_eq!(stored.rate_date, None);
    assert_eq!(stored.rate_series, None);
    assert_eq!(stored.cdi_daily_rate, None);
    assert_eq!(stored.rate_fetched_at, None);
    assert_eq!(balance(&fixture, &pot).await, 100_000);
}

#[tokio::test]
async fn legacy_future_yield_replacement_is_atomic_and_preserves_history() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 115.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;
    let original = yield_at(&fixture, &pot, "2026-11-03").await;
    let balance_before = balance(&fixture, &pot).await;
    let entries_before: i64 = fixture
        .db
        .call(|connection| {
            connection.query_row("SELECT COUNT(*) FROM journal_entries", [], |row| row.get(0))
        })
        .await
        .expect("entries");
    let tomorrow = (Local::now().date_naive() + Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    mark_yield_as_legacy_future(&fixture, &original.id, &tomorrow).await;

    let replacement = fixture
        .savings
        .replace_yield(ReplaceSavingsYieldInput {
            yield_id: original.id.clone(),
        })
        .await
        .expect("substituição de rendimento legado");
    assert_ne!(replacement.id, original.id);
    assert_eq!(replacement.as_of, tomorrow);
    assert_eq!(replacement.status, "confirmed");
    assert_eq!(replacement.rate_date.as_deref(), Some("2026-11-19"));
    assert_eq!(replacement.rate_series.as_deref(), Some("12"));
    assert_eq!(replacement.cdi_daily_rate, Some(0.00050788));
    assert!(replacement.rate_fetched_at.is_some());
    assert_eq!(balance(&fixture, &pot).await, balance_before);

    let entries_after: i64 = fixture
        .db
        .call(|connection| {
            connection.query_row("SELECT COUNT(*) FROM journal_entries", [], |row| row.get(0))
        })
        .await
        .expect("entries");
    assert_eq!(entries_after, entries_before + 2);

    let history = fixture
        .savings
        .overview(&pot)
        .await
        .expect("overview")
        .yields;
    assert_eq!(history.len(), 2);
    let old = history
        .iter()
        .find(|item| item.id == original.id)
        .expect("snapshot original");
    assert_eq!(old.status, "reversed");
    assert_eq!(old.as_of, tomorrow);
    assert_eq!(old.yield_cents, original.yield_cents);
    assert_eq!(old.base_cents, original.base_cents);
    assert_eq!(old.rate_date, None);
    assert_eq!(old.rate_series, None);
    assert_eq!(old.cdi_daily_rate, None);
    assert_eq!(old.rate_fetched_at, None);
    let reversal_entry = old.reversal_entry_id.clone().expect("entry do estorno");
    assert_eq!(entry_occurred_on(&fixture, &reversal_entry).await, tomorrow);

    let new = history
        .iter()
        .find(|item| item.id == replacement.id)
        .expect("snapshot substituto");
    assert_eq!(new.status, "confirmed");
    assert_eq!(new.as_of, tomorrow);
    assert_eq!(new.rate_date.as_deref(), Some("2026-11-19"));
    assert_eq!(new.rate_series.as_deref(), Some("12"));
    assert_eq!(new.cdi_daily_rate, Some(0.00050788));
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

#[tokio::test]
async fn future_and_invalid_reversal_dates_are_rejected() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;
    let original = yield_at(&fixture, &pot, "2026-11-03").await;
    let tomorrow = (Local::now().date_naive() + Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    let future_yield = fixture
        .savings
        .create_yield(CreateSavingsYieldInput {
            account_id: pot.clone(),
            as_of: tomorrow.clone(),
        })
        .await;
    let message = match future_yield {
        Err(AppError::Validation { message }) => message,
        other => panic!("rendimento futuro deveria ser rejeitado: {other:?}"),
    };
    assert!(
        message.contains("não é possível gerar rendimento em data futura"),
        "{message}"
    );

    let future_reversal = fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: original.id.clone(),
            as_of: tomorrow,
        })
        .await;
    let message = match future_reversal {
        Err(AppError::Validation { message }) => message,
        other => panic!("estorno futuro deveria ser rejeitado: {other:?}"),
    };
    assert!(
        message.contains("não é possível estornar em data futura"),
        "{message}"
    );

    let invalid_reversal = fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: original.id.clone(),
            as_of: "24/11/2026".to_owned(),
        })
        .await;
    let message = match invalid_reversal {
        Err(AppError::Validation { message }) => message,
        other => panic!("data inválida deveria ser rejeitada: {other:?}"),
    };
    assert!(message.contains("data válida"), "{message}");

    let earlier_reversal = fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: original.id.clone(),
            as_of: "2026-11-02".to_owned(),
        })
        .await;
    let message = match earlier_reversal {
        Err(AppError::Validation { message }) => message,
        other => panic!("estorno anterior deveria ser rejeitado: {other:?}"),
    };
    assert!(
        message.contains("estorno (2026-11-02)")
            && message.contains("anterior à data do rendimento (2026-11-03)"),
        "{message}"
    );

    let history = fixture
        .savings
        .overview(&pot)
        .await
        .expect("overview")
        .yields;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].status, "confirmed");
}

#[tokio::test]
async fn deposits_and_opening_balance_count_on_the_yield_date() {
    let fixture = fixture().await;
    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
    let bank = account(&fixture, "Banco", 500_000).await;

    let empty = savings(&fixture, "Vazio", "calendar", 100.0).await;
    let no_balance = fixture
        .savings
        .create_yield(CreateSavingsYieldInput {
            account_id: empty,
            as_of: "2026-11-03".to_owned(),
        })
        .await
        .expect_err("sem saldo");
    let message = match no_balance {
        AppError::Validation { message } => message,
        other => panic!("erro inesperado: {other:?}"),
    };
    assert!(
        message.contains("não tem saldo disponível na data informada"),
        "{message}"
    );

    let transferred = savings(&fixture, "Com aplicação", "calendar", 100.0).await;
    transfer(&fixture, &bank, &transferred, 100_000, &today).await;
    let same_day = yield_at(&fixture, &transferred, &today).await;
    assert_eq!(same_day.base_cents, 100_000);

    let opened = fixture
        .accounts
        .create(CreateAccountInput {
            name: "Saldo inicial hoje".to_owned(),
            account_type: "savings".to_owned(),
            opening_balance_cents: 100_000,
            credit_limit_cents: None,
            is_savings: Some(true),
            yield_mode: Some("calendar".to_owned()),
            cdi_percent: Some(100.0),
        })
        .await
        .expect("savings")
        .id;
    let opening = yield_at(&fixture, &opened, &today).await;
    assert_eq!(opening.base_cents, 100_000);
}

#[tokio::test]
async fn replace_yield_recalculates_in_one_backend_call() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 130.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;
    let original = yield_at(&fixture, &pot, "2026-11-03").await;

    let replacement = fixture
        .savings
        .replace_yield(ReplaceSavingsYieldInput {
            yield_id: original.id.clone(),
        })
        .await
        .expect("replacement");
    assert_ne!(replacement.id, original.id);
    assert_eq!(replacement.as_of, original.as_of);
    assert_eq!(replacement.status, "confirmed");

    let history = fixture
        .savings
        .overview(&pot)
        .await
        .expect("overview")
        .yields;
    let old = history
        .iter()
        .find(|item| item.id == original.id)
        .expect("snapshot original");
    assert_eq!(old.status, "reversed");
    assert!(old.reversal_entry_id.is_some());
    assert_eq!(old.yield_cents, original.yield_cents);
    assert_eq!(history.len(), 2);
}

#[tokio::test]
async fn failed_replacement_rolls_back_the_whole_operation() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;
    let original = yield_at(&fixture, &pot, "2026-11-03").await;
    let balance_before = balance(&fixture, &pot).await;

    let entries_before: i64 = fixture
        .db
        .call(|connection| {
            connection.query_row("SELECT COUNT(*) FROM journal_entries", [], |row| row.get(0))
        })
        .await
        .expect("entries");
    fixture
        .db
        .call(|connection| {
            connection.execute_batch(
                "CREATE TRIGGER fail_replacement BEFORE INSERT ON journal_entries
                 WHEN NEW.kind = 'adjustment' AND NEW.occurred_on = '2026-11-03'
                 BEGIN SELECT RAISE(ABORT, 'falha forçada no recálculo'); END;",
            )
        })
        .await
        .expect("trigger");

    let failed = fixture
        .savings
        .replace_yield(ReplaceSavingsYieldInput {
            yield_id: original.id.clone(),
        })
        .await;
    assert!(failed.is_err());

    fixture
        .db
        .call(|connection| connection.execute_batch("DROP TRIGGER fail_replacement;"))
        .await
        .expect("drop trigger");

    let history = fixture
        .savings
        .overview(&pot)
        .await
        .expect("overview")
        .yields;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].id, original.id);
    assert_eq!(history[0].status, "confirmed");
    assert!(history[0].reversal_entry_id.is_none());
    assert_eq!(balance(&fixture, &pot).await, balance_before);

    let entries_after: i64 = fixture
        .db
        .call(|connection| {
            connection.query_row("SELECT COUNT(*) FROM journal_entries", [], |row| row.get(0))
        })
        .await
        .expect("entries");
    assert_eq!(entries_after, entries_before);
}

#[tokio::test]
async fn multiple_yields_same_day_compound_round_and_keep_snapshots() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;

    let first = yield_at(&fixture, &pot, "2026-11-03").await;
    let second = yield_at(&fixture, &pot, "2026-11-03").await;
    let third = yield_at(&fixture, &pot, "2026-11-03").await;

    let expected = |base_cents| {
        calculate_yield(base_cents, YieldMode::Calendar, rate().daily_rate, 10_000).yield_cents
    };
    assert_eq!(first.base_cents, 100_000);
    assert_eq!(second.base_cents, 100_000 + first.yield_cents);
    assert_eq!(
        third.base_cents,
        100_000 + first.yield_cents + second.yield_cents
    );
    for snapshot in [&first, &second, &third] {
        assert_eq!(snapshot.yield_cents, expected(snapshot.base_cents));
        assert_eq!(snapshot.daily_rate, rate().daily_rate);
        assert_eq!(snapshot.as_of, "2026-11-03");
        assert_eq!(snapshot.days, 1);
    }
    let final_balance = 100_000 + first.yield_cents + second.yield_cents + third.yield_cents;
    assert_eq!(balance(&fixture, &pot).await, final_balance);

    let mut filter = movement_filter();
    filter.kind = Some("savings_yield".to_owned());
    let listed = fixture
        .movements
        .list(Some(filter))
        .await
        .expect("movements");
    assert_eq!(listed.len(), 3);
    for snapshot in [&first, &second, &third] {
        assert!(listed.iter().any(|item| item.id == snapshot.entry_id));
    }
    let csv = movements_csv(&listed);
    assert_eq!(csv.lines().count(), 4);
    for item in &listed {
        assert_eq!(csv.matches(&item.id).count(), 1, "CSV duplicou a linha");
    }

    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
    let net_worth = fixture
        .reports
        .net_worth(Some(today))
        .await
        .expect("net worth");
    assert_eq!(net_worth.savings_cents, final_balance);
    assert_eq!(net_worth.net_worth_cents, 400_000);
}

#[tokio::test]
async fn reversal_of_one_among_same_day_yields_keeps_the_others() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;

    let first = yield_at(&fixture, &pot, "2026-11-03").await;
    let second = yield_at(&fixture, &pot, "2026-11-03").await;
    let third = yield_at(&fixture, &pot, "2026-11-03").await;

    fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: second.id.clone(),
            as_of: "2026-11-05".to_owned(),
        })
        .await
        .expect("reversal");

    let history = fixture
        .savings
        .overview(&pot)
        .await
        .expect("overview")
        .yields;
    for id in [&first.id, &third.id] {
        let stored = history
            .iter()
            .find(|item| item.id == *id)
            .expect("rendimento preservado");
        assert_eq!(stored.status, "confirmed");
    }
    let stored_second = history
        .iter()
        .find(|item| item.id == second.id)
        .expect("rendimento estornado");
    assert_eq!(stored_second.status, "reversed");
    assert_eq!(stored_second.yield_cents, second.yield_cents);
    assert_eq!(
        balance(&fixture, &pot).await,
        100_000 + first.yield_cents + third.yield_cents
    );
}

#[tokio::test]
async fn replacement_keeps_later_snapshots_unchanged() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;

    let first = yield_at(&fixture, &pot, "2026-11-03").await;
    let later = yield_at(&fixture, &pot, "2026-11-04").await;

    let replacement = fixture
        .savings
        .replace_yield(ReplaceSavingsYieldInput {
            yield_id: first.id.clone(),
        })
        .await
        .expect("replacement");
    assert_eq!(replacement.as_of, first.as_of);
    assert_eq!(replacement.base_cents, first.base_cents);

    let history = fixture
        .savings
        .overview(&pot)
        .await
        .expect("overview")
        .yields;
    let stored_first = history
        .iter()
        .find(|item| item.id == first.id)
        .expect("snapshot original");
    assert_eq!(stored_first.status, "reversed");
    assert_eq!(stored_first.base_cents, first.base_cents);
    assert_eq!(stored_first.yield_cents, first.yield_cents);

    let stored_later = history
        .iter()
        .find(|item| item.id == later.id)
        .expect("snapshot posterior");
    assert_eq!(stored_later.status, "confirmed");
    assert_eq!(stored_later.base_cents, later.base_cents);
    assert_eq!(stored_later.yield_cents, later.yield_cents);
    assert_eq!(stored_later.daily_rate, later.daily_rate);
}

#[tokio::test]
async fn same_day_yields_survive_reopening_the_database() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("tally.db");
    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();

    let pot = {
        let db = Database::open(path.clone())
            .await
            .expect("database should open");
        let accounts = AccountsService::new(SqliteAccountsRepository::new(db.clone()));
        let savings = SavingsService::new(SqliteSavingsRepository::new(db), FixedCdi);
        let pot = accounts
            .create(CreateAccountInput {
                name: "Cofrinho".to_owned(),
                account_type: "savings".to_owned(),
                opening_balance_cents: 100_000,
                credit_limit_cents: None,
                is_savings: Some(true),
                yield_mode: Some("calendar".to_owned()),
                cdi_percent: Some(100.0),
            })
            .await
            .expect("savings")
            .id;
        savings
            .create_yield(CreateSavingsYieldInput {
                account_id: pot.clone(),
                as_of: today.clone(),
            })
            .await
            .expect("primeiro rendimento");
        savings
            .create_yield(CreateSavingsYieldInput {
                account_id: pot.clone(),
                as_of: today.clone(),
            })
            .await
            .expect("segundo rendimento");
        pot
    };

    let db = Database::open(path).await.expect("database should reopen");
    let savings = SavingsService::new(SqliteSavingsRepository::new(db), FixedCdi);
    let yields = savings.overview(&pot).await.expect("overview").yields;
    assert_eq!(yields.len(), 2);
    assert_eq!(yields[0].as_of, today);
    assert_eq!(yields[1].as_of, today);
    // O histórico vem do mais recente para o mais antigo.
    assert_eq!(yields[0].base_cents, 100_000 + yields[1].yield_cents);
}

#[tokio::test]
async fn migration_0008_preserves_yields_and_drops_the_one_per_day_index() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("legacy-v7.db");
    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();

    let (pot, original) = {
        let db = Database::open(path.clone())
            .await
            .expect("database should open");
        let accounts = AccountsService::new(SqliteAccountsRepository::new(db.clone()));
        let savings = SavingsService::new(SqliteSavingsRepository::new(db.clone()), FixedCdi);
        let pot = accounts
            .create(CreateAccountInput {
                name: "Cofrinho legado".to_owned(),
                account_type: "savings".to_owned(),
                opening_balance_cents: 100_000,
                credit_limit_cents: None,
                is_savings: Some(true),
                yield_mode: Some("calendar".to_owned()),
                cdi_percent: Some(100.0),
            })
            .await
            .expect("savings")
            .id;
        let original = savings
            .create_yield(CreateSavingsYieldInput {
                account_id: pot.clone(),
                as_of: today.clone(),
            })
            .await
            .expect("rendimento legado");
        db.call(|connection| {
            connection.execute_batch(
                "PRAGMA user_version = 7;
                 CREATE UNIQUE INDEX ux_savings_yields_confirmed
                     ON savings_yields(account_id, as_of) WHERE status = 'confirmed';",
            )
        })
        .await
        .expect("simular banco v7");
        (pot, original)
    };

    let db = Database::open(path)
        .await
        .expect("migration 0008 should apply");
    let version: i64 = db
        .call(|connection| connection.query_row("PRAGMA user_version", [], |row| row.get(0)))
        .await
        .expect("version");
    assert_eq!(version, 8);
    let index: Option<String> = db
        .call(|connection| {
            connection
                .query_row(
                    "SELECT name FROM sqlite_master
                     WHERE type = 'index' AND name = 'ux_savings_yields_confirmed'",
                    [],
                    |row| row.get(0),
                )
                .optional()
        })
        .await
        .expect("index");
    assert!(index.is_none(), "índice antigo ainda presente");

    let savings = SavingsService::new(SqliteSavingsRepository::new(db), FixedCdi);
    let yields = savings.overview(&pot).await.expect("overview").yields;
    assert_eq!(yields.len(), 1);
    assert_eq!(yields[0].id, original.id);
    assert_eq!(yields[0].base_cents, original.base_cents);
    assert_eq!(yields[0].yield_cents, original.yield_cents);

    let second = savings
        .create_yield(CreateSavingsYieldInput {
            account_id: pot,
            as_of: today,
        })
        .await
        .expect("segundo rendimento após a migration");
    assert_eq!(
        second.base_cents,
        original.base_cents + original.yield_cents
    );
}

#[tokio::test]
async fn reversing_last_of_day_allows_replacing_previous_confirmed() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;

    let first = yield_at(&fixture, &pot, "2026-11-03").await;
    let last = yield_at(&fixture, &pot, "2026-11-03").await;
    fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: last.id.clone(),
            as_of: "2026-11-05".to_owned(),
        })
        .await
        .expect("reversal do último");

    let replacement = fixture
        .savings
        .replace_yield(ReplaceSavingsYieldInput {
            yield_id: first.id.clone(),
        })
        .await
        .expect("substituição do penúltimo após estorno do último");
    assert_eq!(replacement.as_of, first.as_of);
    assert_eq!(replacement.base_cents, first.base_cents);
    assert_ne!(replacement.id, first.id);

    let history = fixture
        .savings
        .overview(&pot)
        .await
        .expect("overview")
        .yields;
    assert_eq!(history.len(), 3);
    for id in [&first.id, &last.id] {
        let stored = history
            .iter()
            .find(|item| item.id == *id)
            .expect("snapshot preservado");
        assert_eq!(stored.status, "reversed");
        assert!(stored.reversal_entry_id.is_some());
        assert_eq!(
            stored.yield_cents,
            if *id == first.id {
                first.yield_cents
            } else {
                last.yield_cents
            }
        );
    }
}

#[tokio::test]
async fn reversed_yield_cannot_be_replaced_or_reversed_again() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;

    let first = yield_at(&fixture, &pot, "2026-11-03").await;
    let last = yield_at(&fixture, &pot, "2026-11-03").await;
    fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: last.id.clone(),
            as_of: "2026-11-05".to_owned(),
        })
        .await
        .expect("reversal");

    assert!(matches!(
        fixture
            .savings
            .replace_yield(ReplaceSavingsYieldInput {
                yield_id: last.id.clone(),
            })
            .await,
        Err(AppError::Conflict { .. })
    ));
    assert!(matches!(
        fixture
            .savings
            .reverse_yield(ReverseSavingsYieldInput {
                yield_id: last.id.clone(),
                as_of: "2026-11-06".to_owned(),
            })
            .await,
        Err(AppError::Conflict { .. })
    ));
    assert_eq!(first.status, "confirmed");
}

#[tokio::test]
async fn same_day_tie_uses_insertion_order_instead_of_uuid() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;

    let first = yield_at(&fixture, &pot, "2026-11-03").await;
    let last = yield_at(&fixture, &pot, "2026-11-03").await;
    // Força empate em `created_at` e um id menor no registro inserido por último:
    // só a ordem de inserção (rowid) identifica corretamente o último do dia.
    let renamed = "00000000-0000-7000-8000-000000000001".to_owned();
    let first_id = first.id.clone();
    let last_id = last.id.clone();
    let renamed_for_update = renamed.clone();
    fixture
        .db
        .call(move |connection| {
            connection.execute(
                "UPDATE savings_yields SET created_at = '2026-11-03T12:00:00.000Z'
                 WHERE id IN (?1, ?2)",
                params![first_id, last_id.clone()],
            )?;
            connection.execute(
                "UPDATE savings_yields SET id = ?1 WHERE id = ?2",
                params![renamed_for_update, last_id],
            )
        })
        .await
        .expect("empate forçado");

    fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: renamed.clone(),
            as_of: "2026-11-05".to_owned(),
        })
        .await
        .expect("reversal do último inserido");

    let replacement = fixture
        .savings
        .replace_yield(ReplaceSavingsYieldInput {
            yield_id: first.id.clone(),
        })
        .await
        .expect("substituição do penúltimo");
    assert_eq!(replacement.as_of, first.as_of);
    assert_eq!(replacement.base_cents, first.base_cents);
}

#[tokio::test]
async fn delete_stays_blocked_when_all_yields_are_reversed() {
    let fixture = fixture().await;
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, "2026-11-02").await;

    let first = yield_at(&fixture, &pot, "2026-11-03").await;
    let last = yield_at(&fixture, &pot, "2026-11-03").await;
    for id in [&first.id, &last.id] {
        fixture
            .savings
            .reverse_yield(ReverseSavingsYieldInput {
                yield_id: id.clone(),
                as_of: "2026-11-05".to_owned(),
            })
            .await
            .expect("todos os rendimentos estornados");
    }

    assert!(matches!(
        fixture.accounts.delete(&pot).await,
        Err(AppError::Conflict { .. })
    ));
    fixture
        .accounts
        .set_archived(&pot, true)
        .await
        .expect("arquivamento permitido");
    assert!(
        fixture
            .savings
            .create_yield(CreateSavingsYieldInput {
                account_id: pot.clone(),
                as_of: "2026-11-06".to_owned(),
            })
            .await
            .is_err()
    );
    assert!(
        fixture
            .savings
            .reverse_yield(ReverseSavingsYieldInput {
                yield_id: first.id.clone(),
                as_of: "2026-11-07".to_owned(),
            })
            .await
            .is_err()
    );
}

fn current_month_dates() -> (String, String, String) {
    let now = Local::now().date_naive();
    let first = now.with_day(1).expect("dia 1 do mês");
    (
        first.format("%Y-%m-%d").to_string(),
        now.format("%Y-%m-%d").to_string(),
        (first - Duration::days(1)).format("%Y-%m-%d").to_string(),
    )
}

#[tokio::test]
async fn month_yield_indicator_sums_every_yield_of_the_day() {
    let fixture = fixture().await;
    let (first_of_month, today, _) = current_month_dates();
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, &first_of_month).await;

    let dashboard_before = fixture.reports.dashboard().await.expect("dashboard");
    assert_eq!(dashboard_before.month_savings_yield_cents, 0);

    let first = yield_at(&fixture, &pot, &today).await;
    let second = yield_at(&fixture, &pot, &today).await;
    let third = yield_at(&fixture, &pot, &today).await;

    let dashboard = fixture.reports.dashboard().await.expect("dashboard");
    assert_eq!(
        dashboard.month_savings_yield_cents,
        first.yield_cents + second.yield_cents + third.yield_cents
    );
    assert_eq!(
        dashboard.month_income_cents,
        dashboard_before.month_income_cents
    );
    assert_eq!(
        dashboard.month_expense_cents,
        dashboard_before.month_expense_cents
    );
    assert_eq!(
        dashboard.month_result_cents,
        dashboard_before.month_result_cents
    );

    let flow = fixture
        .reports
        .cash_flow(ReportFilter {
            from: Some(dashboard.month_from.clone()),
            to: Some(dashboard.month_to.clone()),
            period: None,
            category_id: None,
            account_id: None,
            payment_method: None,
        })
        .await
        .expect("cash flow");
    assert_eq!(flow.net_cents, 0);
}

#[tokio::test]
async fn month_yield_indicator_subtracts_reversal_of_previous_month_yield() {
    let fixture = fixture().await;
    let (_, today, previous_month_end) = current_month_dates();
    let previous_month_day = (Local::now().date_naive().with_day(1).expect("dia 1")
        - Duration::days(2))
    .format("%Y-%m-%d")
    .to_string();
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, &previous_month_day).await;

    let old = yield_at(&fixture, &pot, &previous_month_end).await;
    let dashboard_before = fixture.reports.dashboard().await.expect("dashboard");
    assert_eq!(dashboard_before.month_savings_yield_cents, 0);

    fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: old.id.clone(),
            as_of: today.clone(),
        })
        .await
        .expect("reversal no mês atual");

    let dashboard = fixture.reports.dashboard().await.expect("dashboard");
    assert_eq!(dashboard.month_savings_yield_cents, -old.yield_cents);
}

#[tokio::test]
async fn month_yield_indicator_nets_same_month_reversal_and_replacement() {
    let fixture = fixture().await;
    let (first_of_month, today, _) = current_month_dates();
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, &first_of_month).await;

    let reverted = yield_at(&fixture, &pot, &today).await;
    fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: reverted.id.clone(),
            as_of: today.clone(),
        })
        .await
        .expect("reversal");
    let after_reversal = fixture.reports.dashboard().await.expect("dashboard");
    assert_eq!(after_reversal.month_savings_yield_cents, 0);

    let original = yield_at(&fixture, &pot, &today).await;
    let replacement = fixture
        .savings
        .replace_yield(ReplaceSavingsYieldInput {
            yield_id: original.id.clone(),
        })
        .await
        .expect("replacement");
    let dashboard = fixture.reports.dashboard().await.expect("dashboard");
    // O original e seu estorno se cancelam no mês; sobra o rendimento substituto.
    assert_eq!(dashboard.month_savings_yield_cents, replacement.yield_cents);
}

#[tokio::test]
async fn month_yield_indicator_matches_savings_balance_delta() {
    let fixture = fixture().await;
    let (first_of_month, today, previous_month_end) = current_month_dates();
    let bank = account(&fixture, "Banco", 500_000).await;
    let pot = savings(&fixture, "Cofrinho", "calendar", 100.0).await;
    transfer(&fixture, &bank, &pot, 100_000, &previous_month_end).await;

    let before = balance(&fixture, &pot).await;
    yield_at(&fixture, &pot, &first_of_month).await;
    let last = yield_at(&fixture, &pot, &today).await;
    fixture
        .savings
        .reverse_yield(ReverseSavingsYieldInput {
            yield_id: last.id.clone(),
            as_of: today.clone(),
        })
        .await
        .expect("reversal");

    let dashboard = fixture.reports.dashboard().await.expect("dashboard");
    assert_eq!(
        dashboard.month_savings_yield_cents,
        balance(&fixture, &pot).await - before
    );
}
