use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Local};
use rusqlite::params;
use tally_app::dto::{BackupDto, CsvExportDto, MovementDto, RestoreResultDto};
use tally_app::error::AppError;
use tally_app::ports::BootstrapRepository;

use crate::bootstrap_repo::SqliteBootstrapRepository;
use crate::db::Database;

const BACKUP_PREFIX: &str = "tally-backup-";
const PRE_MIGRATION_PREFIX: &str = "tally-pre-migration-";
const PRE_RESTORE_PREFIX: &str = "tally-pre-restore-";
const EXPORT_PREFIX: &str = "tally-movements-";

#[derive(Clone)]
pub struct DataTools {
    db: Database,
    backups: PathBuf,
    exports: PathBuf,
}

impl DataTools {
    pub fn new(db: Database, backups: impl Into<PathBuf>, exports: impl Into<PathBuf>) -> Self {
        Self {
            db,
            backups: backups.into(),
            exports: exports.into(),
        }
    }

    pub async fn create_backup(&self) -> Result<BackupDto, AppError> {
        fs::create_dir_all(&self.backups).map_err(|error| {
            AppError::io(
                "não foi possível criar o diretório de backups",
                error.to_string(),
            )
        })?;

        let file_name = format!(
            "{BACKUP_PREFIX}{}.db",
            Local::now().format("%Y%m%d-%H%M%S-%3f")
        );
        let path = self.backups.join(&file_name);
        let target = path.to_string_lossy().to_string();

        self.db
            .call(move |connection| {
                connection.execute("VACUUM INTO ?1", params![target])?;
                Ok(())
            })
            .await?;

        let created_at = Local::now().to_rfc3339();
        let setting = created_at.clone();
        self.db
            .call(move |connection| {
                connection.execute(
                    "INSERT INTO settings (key, value, updated_at)
                     VALUES ('last_backup_at', ?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                     ON CONFLICT(key) DO UPDATE SET
                         value = excluded.value,
                         updated_at = excluded.updated_at",
                    params![setting],
                )?;
                Ok(())
            })
            .await?;

        self.prune_backups().await?;

        Ok(backup_dto(&path, created_at))
    }

    pub fn list_backups(&self) -> Result<Vec<BackupDto>, AppError> {
        let files = backup_files(&self.backups)?;
        Ok(files
            .into_iter()
            .map(|path| {
                let modified = fs::metadata(&path)
                    .and_then(|metadata| metadata.modified())
                    .unwrap_or_else(|_| SystemTime::now());
                backup_dto(&path, format_timestamp(modified))
            })
            .collect())
    }

    pub async fn restore_backup(&self, file_name: &str) -> Result<RestoreResultDto, AppError> {
        validate_backup_name(file_name)?;

        let path = self.backups.join(file_name);
        if !path.is_file() {
            return Err(AppError::not_found("backup não encontrado"));
        }

        // A restauração substitui o banco inteiro. Antes de tocar nos dados
        // atuais, guarda uma cópia que sobrevive à retenção normal.
        self.create_pre_restore_snapshot().await?;

        self.db.restore_from(&path).await?;

        let repository = SqliteBootstrapRepository::new(self.db.clone());
        let integrity = repository.integrity_check().await?;

        let modified = fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .unwrap_or_else(|_| SystemTime::now());

        Ok(RestoreResultDto {
            backup: backup_dto(&path, format_timestamp(modified)),
            integrity,
        })
    }

    async fn create_pre_restore_snapshot(&self) -> Result<PathBuf, AppError> {
        fs::create_dir_all(&self.backups).map_err(|error| {
            AppError::io(
                "não foi possível criar o diretório de backups",
                error.to_string(),
            )
        })?;

        let file_name = format!(
            "{PRE_RESTORE_PREFIX}{}.db",
            Local::now().format("%Y%m%d-%H%M%S-%3f")
        );
        let path = self.backups.join(&file_name);
        let target = path.to_string_lossy().to_string();

        self.db
            .call(move |connection| {
                connection.execute("VACUUM INTO ?1", params![target])?;
                Ok(())
            })
            .await?;

        Ok(path)
    }

    pub fn export_movements(&self, movements: &[MovementDto]) -> Result<CsvExportDto, AppError> {
        fs::create_dir_all(&self.exports).map_err(|error| {
            AppError::io(
                "não foi possível criar o diretório de exportação",
                error.to_string(),
            )
        })?;

        let file_name = format!(
            "{EXPORT_PREFIX}{}.csv",
            Local::now().format("%Y%m%d-%H%M%S-%3f")
        );
        let path = self.exports.join(&file_name);
        let content = tally_app::movements_csv(movements);

        fs::write(&path, content)
            .map_err(|error| AppError::io("não foi possível gravar o CSV", error.to_string()))?;

        Ok(CsvExportDto {
            file_name,
            path: path.display().to_string(),
            row_count: movements.len() as i64,
        })
    }

    async fn prune_backups(&self) -> Result<(), AppError> {
        let retention = self
            .db
            .call(|connection| {
                connection.query_row(
                    "SELECT value FROM settings WHERE key = 'backup_retention'",
                    [],
                    |row| row.get::<_, String>(0),
                )
            })
            .await
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(10)
            .max(1);

        let files = backup_files(&self.backups)?;
        for path in files
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(BACKUP_PREFIX))
            })
            .skip(retention)
        {
            fs::remove_file(&path).map_err(|error| {
                AppError::io(
                    "não foi possível remover backups antigos",
                    error.to_string(),
                )
            })?;
        }

        Ok(())
    }
}

fn validate_backup_name(file_name: &str) -> Result<(), AppError> {
    let has_safe_characters = file_name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'));

    let has_known_prefix = [BACKUP_PREFIX, PRE_MIGRATION_PREFIX, PRE_RESTORE_PREFIX]
        .iter()
        .any(|prefix| file_name.starts_with(prefix));

    if !has_safe_characters
        || !has_known_prefix
        || file_name.contains("..")
        || !file_name.ends_with(".db")
    {
        return Err(AppError::validation("nome de backup inválido"));
    }

    Ok(())
}

fn backup_files(directory: &Path) -> Result<Vec<PathBuf>, AppError> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(AppError::io(
                "não foi possível listar os backups",
                error.to_string(),
            ));
        }
    };

    let mut files = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("db"))
        .collect::<Vec<_>>();

    // Ordena pela data do arquivo: backups de migração/restauração têm
    // prefixos diferentes, então o nome sozinho não reflete a cronologia.
    files.sort_by(|left, right| {
        let left_time = fs::metadata(left)
            .and_then(|metadata| metadata.modified())
            .ok();
        let right_time = fs::metadata(right)
            .and_then(|metadata| metadata.modified())
            .ok();
        right_time
            .cmp(&left_time)
            .then_with(|| right.file_name().cmp(&left.file_name()))
    });
    Ok(files)
}

fn backup_dto(path: &Path, created_at: String) -> BackupDto {
    let size_bytes = fs::metadata(path)
        .map(|metadata| metadata.len() as i64)
        .unwrap_or(0);

    BackupDto {
        file_name: path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default(),
        path: path.display().to_string(),
        size_bytes,
        created_at,
    }
}

fn format_timestamp(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    datetime.to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::backup_files;

    #[test]
    fn backup_dto_uses_file_name_and_path() {
        let temp = tempfile::tempdir().expect("temp dir");
        let path = temp.path().join("tally-backup-20260101-120000-000.db");
        std::fs::write(&path, b"conteudo").expect("write");
        let dto = super::backup_dto(&path, "2026-01-01T12:00:00-03:00".to_owned());
        assert_eq!(dto.file_name, "tally-backup-20260101-120000-000.db");
        assert_eq!(dto.size_bytes, 8);
        assert!(dto.path.ends_with(&dto.file_name));
        assert_eq!(
            backup_files(temp.path()).expect("files"),
            vec![path.clone()]
        );
    }
}
