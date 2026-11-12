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
        let local = app.path().local_data_dir().map_err(|error| {
            AppError::io(
                "não foi possível resolver o diretório de dados do aplicativo",
                error.to_string(),
            )
        })?;

        // O instalador NSIS per-user usa %LOCALAPPDATA%\Tally, o mesmo caminho
        // usado por versões anteriores para os dados. Quem já tem banco lá
        // continua nesse diretório; instalações novas usam TallyData para não
        // misturar dados financeiros com os binários do aplicativo.
        let legacy_root = local.join("Tally");
        let root = if legacy_root.join("tally.db").exists() {
            legacy_root
        } else {
            local.join("TallyData")
        };

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
