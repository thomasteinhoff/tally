use tally_app::{
    AccountDto, AccountStatementDto, AccountsService, AppError, BackupDto, BootstrapDto,
    BootstrapService, CardPaymentDto, CardSummaryDto, CardsService, CashFlowReportDto,
    CategoriesService, CategoryDto, CategoryReportDto, CommitmentsReportDto, CreateAccountInput,
    CreateAdjustmentInput, CreateCardPaymentInput, CreateCardPurchaseInput, CreateCategoryInput,
    CreateExpenseInput, CreateIncomeInput, CreateSavingsYieldInput, CreateTransferInput,
    CsvExportDto, DashboardDto, ExpenseTrendDto, ExportCsvInput, InstallmentPlanDto,
    IntegrityReportDto, MovementDto, MovementFilter, MovementsService, NetWorthHistoryDto,
    NetWorthReportDto, PeriodSpendingReportDto, ReplaceSavingsYieldInput, ReportFilter,
    ReportsService, RestoreResultDto, ReverseSavingsYieldInput, SavingsOverviewDto, SavingsService,
    SavingsYieldDto, SettingsDto, UpdateInstallmentPlanInput, UpdateMovementInput,
    UpdateSettingsInput,
};
use tally_infra::{
    DataTools, Database, SqliteAccountsRepository, SqliteBootstrapRepository,
    SqliteCardsRepository, SqliteCategoriesRepository, SqliteMovementsRepository,
    SqliteReportsRepository, SqliteSavingsRepository,
};
use tauri::State;

use crate::state::AppState;

fn database(state: &AppState) -> Result<Database, AppError> {
    match &state.db {
        Some(db) => Ok(db.clone()),
        None => Err(AppError::database(
            "o banco de dados não está disponível",
            state
                .boot_error
                .clone()
                .unwrap_or_else(|| "erro de inicialização desconhecido".to_owned()),
        )),
    }
}

fn accounts_service(
    state: &AppState,
) -> Result<AccountsService<SqliteAccountsRepository>, AppError> {
    let db = database(state)?;
    Ok(AccountsService::new(SqliteAccountsRepository::new(db)))
}

fn categories_service(
    state: &AppState,
) -> Result<CategoriesService<SqliteCategoriesRepository>, AppError> {
    let db = database(state)?;
    Ok(CategoriesService::new(SqliteCategoriesRepository::new(db)))
}

fn movements_service(
    state: &AppState,
) -> Result<MovementsService<SqliteMovementsRepository>, AppError> {
    let db = database(state)?;
    Ok(MovementsService::new(SqliteMovementsRepository::new(db)))
}

fn cards_service(state: &AppState) -> Result<CardsService<SqliteCardsRepository>, AppError> {
    let db = database(state)?;
    Ok(CardsService::new(SqliteCardsRepository::new(db)))
}

fn reports_service(state: &AppState) -> Result<ReportsService<SqliteReportsRepository>, AppError> {
    let db = database(state)?;
    Ok(ReportsService::new(SqliteReportsRepository::new(db)))
}

fn savings_service(state: &AppState) -> Result<SavingsService<SqliteSavingsRepository>, AppError> {
    let db = database(state)?;
    Ok(SavingsService::new(SqliteSavingsRepository::new(db)))
}

fn data_tools(state: &AppState) -> Result<DataTools, AppError> {
    let db = database(state)?;
    Ok(DataTools::new(
        db,
        state.paths.backups.clone(),
        state.paths.exports.clone(),
    ))
}

fn bootstrap_service(
    state: &AppState,
) -> Result<BootstrapService<SqliteBootstrapRepository>, AppError> {
    let db = database(state)?;
    Ok(BootstrapService::new(
        SqliteBootstrapRepository::new(db),
        state.app_version.clone(),
        state.paths.db_path.display().to_string(),
        state.paths.root.display().to_string(),
    ))
}

#[tauri::command]
pub async fn get_bootstrap(state: State<'_, AppState>) -> Result<BootstrapDto, AppError> {
    bootstrap_service(state.inner())?.bootstrap().await
}

#[tauri::command]
pub async fn run_integrity_check(
    state: State<'_, AppState>,
) -> Result<IntegrityReportDto, AppError> {
    bootstrap_service(state.inner())?.integrity_check().await
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<SettingsDto, AppError> {
    bootstrap_service(state.inner())?.settings().await
}

#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    input: UpdateSettingsInput,
) -> Result<SettingsDto, AppError> {
    bootstrap_service(state.inner())?
        .update_settings(input)
        .await
}

#[tauri::command]
pub async fn list_accounts(
    state: State<'_, AppState>,
    include_archived: Option<bool>,
) -> Result<Vec<AccountDto>, AppError> {
    accounts_service(state.inner())?
        .list(include_archived.unwrap_or(false))
        .await
}

#[tauri::command]
pub async fn create_account(
    state: State<'_, AppState>,
    input: CreateAccountInput,
) -> Result<AccountDto, AppError> {
    accounts_service(state.inner())?.create(input).await
}

#[tauri::command]
pub async fn update_account(
    state: State<'_, AppState>,
    id: String,
    name: String,
    credit_limit_cents: Option<i64>,
) -> Result<AccountDto, AppError> {
    accounts_service(state.inner())?
        .update(&id, &name, credit_limit_cents)
        .await
}

#[tauri::command]
pub async fn set_account_archived(
    state: State<'_, AppState>,
    id: String,
    archived: bool,
) -> Result<AccountDto, AppError> {
    accounts_service(state.inner())?
        .set_archived(&id, archived)
        .await
}

#[tauri::command]
pub async fn delete_account(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    accounts_service(state.inner())?.delete(&id).await
}

#[tauri::command]
pub async fn get_savings_overview(
    state: State<'_, AppState>,
    account_id: String,
) -> Result<SavingsOverviewDto, AppError> {
    savings_service(state.inner())?.overview(&account_id).await
}

#[tauri::command]
pub async fn create_savings_yield(
    state: State<'_, AppState>,
    input: CreateSavingsYieldInput,
) -> Result<SavingsYieldDto, AppError> {
    savings_service(state.inner())?.create_yield(input).await
}

#[tauri::command]
pub async fn reverse_savings_yield(
    state: State<'_, AppState>,
    input: ReverseSavingsYieldInput,
) -> Result<SavingsYieldDto, AppError> {
    savings_service(state.inner())?.reverse_yield(input).await
}

#[tauri::command]
pub async fn replace_savings_yield(
    state: State<'_, AppState>,
    input: ReplaceSavingsYieldInput,
) -> Result<SavingsYieldDto, AppError> {
    savings_service(state.inner())?.replace_yield(input).await
}

#[tauri::command]
pub async fn list_categories(
    state: State<'_, AppState>,
    include_archived: Option<bool>,
) -> Result<Vec<CategoryDto>, AppError> {
    categories_service(state.inner())?
        .list(include_archived.unwrap_or(false))
        .await
}

#[tauri::command]
pub async fn create_category(
    state: State<'_, AppState>,
    input: CreateCategoryInput,
) -> Result<CategoryDto, AppError> {
    categories_service(state.inner())?.create(input).await
}

#[tauri::command]
pub async fn update_category(
    state: State<'_, AppState>,
    id: String,
    name: String,
    icon: Option<String>,
) -> Result<CategoryDto, AppError> {
    categories_service(state.inner())?
        .update(&id, &name, icon.as_deref())
        .await
}

#[tauri::command]
pub async fn set_category_archived(
    state: State<'_, AppState>,
    id: String,
    archived: bool,
) -> Result<CategoryDto, AppError> {
    categories_service(state.inner())?
        .set_archived(&id, archived)
        .await
}

#[tauri::command]
pub async fn delete_category(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    categories_service(state.inner())?.delete(&id).await
}

#[tauri::command]
pub async fn list_movements(
    state: State<'_, AppState>,
    filter: Option<MovementFilter>,
) -> Result<Vec<MovementDto>, AppError> {
    movements_service(state.inner())?.list(filter).await
}

#[tauri::command]
pub async fn create_expense(
    state: State<'_, AppState>,
    input: CreateExpenseInput,
) -> Result<MovementDto, AppError> {
    movements_service(state.inner())?
        .create_expense(input)
        .await
}

#[tauri::command]
pub async fn create_income(
    state: State<'_, AppState>,
    input: CreateIncomeInput,
) -> Result<MovementDto, AppError> {
    movements_service(state.inner())?.create_income(input).await
}

#[tauri::command]
pub async fn create_transfer(
    state: State<'_, AppState>,
    input: CreateTransferInput,
) -> Result<MovementDto, AppError> {
    movements_service(state.inner())?
        .create_transfer(input)
        .await
}

#[tauri::command]
pub async fn create_adjustment(
    state: State<'_, AppState>,
    input: CreateAdjustmentInput,
) -> Result<MovementDto, AppError> {
    movements_service(state.inner())?
        .create_adjustment(input)
        .await
}

#[tauri::command]
pub async fn update_movement(
    state: State<'_, AppState>,
    id: String,
    input: UpdateMovementInput,
) -> Result<MovementDto, AppError> {
    movements_service(state.inner())?.update(&id, input).await
}

#[tauri::command]
pub async fn delete_movement(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    movements_service(state.inner())?.delete(&id).await
}

#[tauri::command]
pub async fn get_account_statement(
    state: State<'_, AppState>,
    account_id: String,
    from: Option<String>,
    to: Option<String>,
) -> Result<AccountStatementDto, AppError> {
    movements_service(state.inner())?
        .statement(&account_id, from.as_deref(), to.as_deref())
        .await
}

#[tauri::command]
pub async fn list_cards(state: State<'_, AppState>) -> Result<Vec<CardSummaryDto>, AppError> {
    cards_service(state.inner())?.list_cards().await
}

#[tauri::command]
pub async fn get_card(
    state: State<'_, AppState>,
    card_account_id: String,
) -> Result<CardSummaryDto, AppError> {
    cards_service(state.inner())?
        .card_summary(&card_account_id)
        .await
}

#[tauri::command]
pub async fn create_card_purchase(
    state: State<'_, AppState>,
    input: CreateCardPurchaseInput,
) -> Result<InstallmentPlanDto, AppError> {
    cards_service(state.inner())?.create_purchase(input).await
}

#[tauri::command]
pub async fn list_installment_plans(
    state: State<'_, AppState>,
    card_account_id: Option<String>,
) -> Result<Vec<InstallmentPlanDto>, AppError> {
    cards_service(state.inner())?
        .list_plans(card_account_id)
        .await
}

#[tauri::command]
pub async fn get_installment_plan(
    state: State<'_, AppState>,
    plan_id: String,
) -> Result<InstallmentPlanDto, AppError> {
    cards_service(state.inner())?.get_plan(&plan_id).await
}

#[tauri::command]
pub async fn delete_installment_plan(
    state: State<'_, AppState>,
    plan_id: String,
) -> Result<(), AppError> {
    cards_service(state.inner())?.delete_plan(&plan_id).await
}

#[tauri::command]
pub async fn update_installment_plan(
    state: State<'_, AppState>,
    plan_id: String,
    input: UpdateInstallmentPlanInput,
) -> Result<InstallmentPlanDto, AppError> {
    cards_service(state.inner())?
        .update_plan(&plan_id, input)
        .await
}

#[tauri::command]
pub async fn create_card_payment(
    state: State<'_, AppState>,
    input: CreateCardPaymentInput,
) -> Result<CardPaymentDto, AppError> {
    cards_service(state.inner())?.create_payment(input).await
}

#[tauri::command]
pub async fn list_card_payments(
    state: State<'_, AppState>,
    card_account_id: Option<String>,
) -> Result<Vec<CardPaymentDto>, AppError> {
    cards_service(state.inner())?
        .list_payments(card_account_id)
        .await
}

#[tauri::command]
pub async fn delete_card_payment(
    state: State<'_, AppState>,
    payment_id: String,
) -> Result<(), AppError> {
    cards_service(state.inner())?
        .delete_payment(&payment_id)
        .await
}

#[tauri::command]
pub async fn get_dashboard(state: State<'_, AppState>) -> Result<DashboardDto, AppError> {
    reports_service(state.inner())?.dashboard().await
}

#[tauri::command]
pub async fn get_spending_report(
    state: State<'_, AppState>,
    filter: ReportFilter,
) -> Result<CategoryReportDto, AppError> {
    reports_service(state.inner())?.spending(filter).await
}

#[tauri::command]
pub async fn get_income_report(
    state: State<'_, AppState>,
    filter: ReportFilter,
) -> Result<CategoryReportDto, AppError> {
    reports_service(state.inner())?.income(filter).await
}

#[tauri::command]
pub async fn get_cash_flow_report(
    state: State<'_, AppState>,
    filter: ReportFilter,
) -> Result<CashFlowReportDto, AppError> {
    reports_service(state.inner())?.cash_flow(filter).await
}

#[tauri::command]
pub async fn get_period_spending_report(
    state: State<'_, AppState>,
    as_of: Option<String>,
) -> Result<PeriodSpendingReportDto, AppError> {
    reports_service(state.inner())?.period_spending(as_of).await
}

#[tauri::command]
pub async fn get_expense_trend_report(
    state: State<'_, AppState>,
    as_of: Option<String>,
) -> Result<ExpenseTrendDto, AppError> {
    reports_service(state.inner())?.expense_trend(as_of).await
}

#[tauri::command]
pub async fn get_net_worth_history_report(
    state: State<'_, AppState>,
    as_of: Option<String>,
) -> Result<NetWorthHistoryDto, AppError> {
    reports_service(state.inner())?
        .net_worth_history(as_of)
        .await
}

#[tauri::command]
pub async fn get_net_worth_report(
    state: State<'_, AppState>,
    as_of: Option<String>,
) -> Result<NetWorthReportDto, AppError> {
    reports_service(state.inner())?.net_worth(as_of).await
}

#[tauri::command]
pub async fn get_commitments_report(
    state: State<'_, AppState>,
    as_of: Option<String>,
    months: Option<u32>,
) -> Result<CommitmentsReportDto, AppError> {
    reports_service(state.inner())?
        .commitments(as_of, months)
        .await
}

#[tauri::command]
pub async fn create_backup(state: State<'_, AppState>) -> Result<BackupDto, AppError> {
    data_tools(state.inner())?.create_backup().await
}

#[tauri::command]
pub async fn list_backups(state: State<'_, AppState>) -> Result<Vec<BackupDto>, AppError> {
    data_tools(state.inner())?.list_backups()
}

#[tauri::command]
pub async fn restore_backup(
    state: State<'_, AppState>,
    file_name: String,
) -> Result<RestoreResultDto, AppError> {
    data_tools(state.inner())?.restore_backup(&file_name).await
}

#[tauri::command]
pub async fn open_app_folder(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    folder: String,
) -> Result<(), AppError> {
    use tauri_plugin_opener::OpenerExt;

    let path = match folder.as_str() {
        "data" => state.paths.root.clone(),
        "logs" => state.paths.logs.clone(),
        _ => return Err(AppError::validation("pasta solicitada não é reconhecida")),
    };

    app.opener()
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|error| {
            AppError::io(
                "não foi possível abrir a pasta no Explorer",
                error.to_string(),
            )
        })
}

#[tauri::command]
pub async fn export_csv(
    state: State<'_, AppState>,
    input: ExportCsvInput,
) -> Result<CsvExportDto, AppError> {
    let movements = movements_service(state.inner())?
        .list(Some(MovementFilter {
            account_id: input.account_id,
            category_id: input.category_id,
            kind: input.kind,
            period: input.period,
            payment_method: input.payment_method,
            from: input.from,
            to: input.to,
        }))
        .await?;

    data_tools(state.inner())?.export_movements(&movements)
}
