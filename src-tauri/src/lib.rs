mod commands;
mod logging;
mod paths;
mod state;

use state::AppState;
use tally_infra::Database;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let paths = paths::AppPaths::resolve(app.handle())?;
            paths.ensure()?;

            match logging::init(&paths) {
                Ok(guard) => {
                    let _ = app.manage(guard);
                }
                Err(error) => {
                    eprintln!("Tally: falha ao iniciar logging: {error}");
                }
            }

            let (db, boot_error) =
                match tauri::async_runtime::block_on(Database::open(paths.db_path.clone())) {
                    Ok(db) => (Some(db), None),
                    Err(error) => {
                        tracing::error!(
                            error = %error,
                            db_path = %paths.db_path.display(),
                            "falha ao inicializar o banco de dados"
                        );
                        (None, Some(error.to_string()))
                    }
                };

            let _ = app.manage(AppState {
                db,
                paths,
                app_version: env!("CARGO_PKG_VERSION").to_owned(),
                boot_error,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::run_integrity_check,
            commands::list_accounts,
            commands::create_account,
            commands::update_account,
            commands::set_account_archived,
            commands::delete_account,
            commands::list_categories,
            commands::create_category,
            commands::update_category,
            commands::set_category_archived,
            commands::delete_category,
            commands::list_movements,
            commands::create_expense,
            commands::create_income,
            commands::create_transfer,
            commands::create_adjustment,
            commands::update_movement,
            commands::delete_movement,
            commands::get_account_statement,
            commands::list_cards,
            commands::get_card,
            commands::create_card_purchase,
            commands::list_installment_plans,
            commands::get_installment_plan,
            commands::delete_installment_plan,
            commands::create_card_payment,
            commands::list_card_payments,
            commands::delete_card_payment
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar o Tally");
}
