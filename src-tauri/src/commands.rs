use tally_app::{AppError, BootstrapDto, BootstrapService, IntegrityReportDto};
use tally_infra::{Database, SqliteBootstrapRepository};
use tauri::State;

use crate::state::AppState;

fn database(state: &AppState) -> Result<Database, AppError> {
    match &state.db {
        Some(db) => Ok(db.clone()),
        None => Err(AppError::bootstrap_with_details(
            "o banco de dados não está disponível",
            state
                .boot_error
                .clone()
                .unwrap_or_else(|| "erro de inicialização desconhecido".to_owned()),
        )),
    }
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
