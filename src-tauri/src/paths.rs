use std::path::PathBuf;

use tally_app::AppError;
use tauri::{AppHandle, Manager, Runtime};

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub root: PathBuf,
    pub db_path: PathBuf,
    pub backups: PathBuf,
    pub exports: PathBuf,
    pub logs: PathBuf,
}

impl AppPaths {
    pub fn resolve<R: Runtime>(app: &AppHandle<R>) -> Result<Self, AppError> {
        let root = app
            .path()
            .local_data_dir()
            .map_err(|error| {
                AppError::io(
                    "não foi possível resolver o diretório de dados do aplicativo",
                    error.to_string(),
                )
            })?
            .join("Tally");

        Ok(Self {
            db_path: root.join("tally.db"),
            backups: root.join("backups"),
            exports: root.join("exports"),
            logs: root.join("logs"),
            root,
        })
    }

    pub fn ensure(&self) -> Result<(), AppError> {
        for path in [&self.root, &self.backups, &self.exports, &self.logs] {
            std::fs::create_dir_all(path).map_err(|error| {
                AppError::io(
                    "não foi possível criar o diretório de dados do aplicativo",
                    format!("{}: {error}", path.display()),
                )
            })?;
        }

        Ok(())
    }
}
