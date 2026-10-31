use tally_app::{
    AccountDto, AccountStatementDto, AccountsService, AppError, BootstrapDto, BootstrapService,
    CategoriesService, CategoryDto, CreateAccountInput, CreateAdjustmentInput, CreateCategoryInput,
    CreateExpenseInput, CreateIncomeInput, CreateTransferInput, IntegrityReportDto, MovementDto,
    MovementFilter, MovementsService, UpdateMovementInput,
};
use tally_infra::{
    Database, SqliteAccountsRepository, SqliteBootstrapRepository, SqliteCategoriesRepository,
    SqliteMovementsRepository,
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

#[tauri::command]
pub async fn get_bootstrap(state: State<'_, AppState>) -> Result<BootstrapDto, AppError> {
    let db = database(state.inner())?;
    let repository = SqliteBootstrapRepository::new(db);
    let service = BootstrapService::new(
        repository,
        state.app_version.clone(),
        state.paths.db_path.display().to_string(),
        state.paths.root.display().to_string(),
    );

    service.bootstrap().await
}

#[tauri::command]
pub async fn run_integrity_check(
    state: State<'_, AppState>,
) -> Result<IntegrityReportDto, AppError> {
    let db = database(state.inner())?;
    let repository = SqliteBootstrapRepository::new(db);
    let service = BootstrapService::new(
        repository,
        state.app_version.clone(),
        state.paths.db_path.display().to_string(),
        state.paths.root.display().to_string(),
    );

    service.integrity_check().await
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
) -> Result<AccountDto, AppError> {
    accounts_service(state.inner())?.rename(&id, &name).await
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
) -> Result<CategoryDto, AppError> {
    categories_service(state.inner())?.rename(&id, &name).await
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
