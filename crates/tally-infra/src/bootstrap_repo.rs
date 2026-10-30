use std::collections::HashMap;

use async_trait::async_trait;
use tally_app::dto::{IntegrityReportDto, SettingsDto};
use tally_app::error::AppError;
use tally_app::ports::BootstrapRepository;

use crate::db::Database;

#[derive(Clone)]
pub struct SqliteBootstrapRepository {
    db: Database,
}

impl SqliteBootstrapRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl BootstrapRepository for SqliteBootstrapRepository {
    async fn schema_version(&self) -> Result<i64, AppError> {
        self.db
            .call(|connection| {
                connection.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            })
            .await
    }

    async fn settings(&self) -> Result<SettingsDto, AppError> {
        let rows = self
            .db
            .call(|connection| {
                let mut statement = connection.prepare("SELECT key, value FROM settings")?;
                let rows = statement.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })?;

                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?;

        let values: HashMap<String, String> = rows.into_iter().collect();

        let backup_retention = match values.get("backup_retention") {
            Some(raw) => raw.parse::<u32>().unwrap_or_else(|_| {
                tracing::warn!(value = %raw, "backup_retention inválido; usando 10");
                10
            }),
            None => 10,
        };

        let first_run_completed = values
            .get("first_run_completed")
            .is_some_and(|value| value == "true");

        Ok(SettingsDto {
            backup_retention,
            first_run_completed,
            last_backup_at: values.get("last_backup_at").cloned(),
        })
    }

    async fn system_account_count(&self) -> Result<i64, AppError> {
        self.db
            .call(|connection| {
                connection.query_row(
                    "SELECT COUNT(*) FROM accounts WHERE is_system = 1",
                    [],
                    |row| row.get::<_, i64>(0),
                )
            })
            .await
    }

    async fn integrity_check(&self) -> Result<IntegrityReportDto, AppError> {
        let issues = self
            .db
            .call(|connection| {
                let mut statement = connection.prepare("PRAGMA integrity_check")?;
                let rows = statement.query_map([], |row| row.get::<_, String>(0))?;

                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await?;

        let ok = issues.len() == 1 && issues[0] == "ok";

        Ok(IntegrityReportDto {
            ok,
            issues: if ok { Vec::new() } else { issues },
        })
    }
}
