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
            commands::run_integrity_check
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar o Tally");
}
