use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Local;
use rusqlite::Connection as SyncConnection;
use rusqlite::OpenFlags;
use rusqlite_migration::{M, Migrations};
use tally_app::AppError;
use tokio_rusqlite::Connection;

#[derive(Clone)]
pub struct Database {
    connection: Connection,
    path: PathBuf,
}

impl Database {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let path = path.as_ref().to_path_buf();
        let existed = path.exists();

        let mut sync_connection = SyncConnection::open(&path).map_err(|error| {
            AppError::database("não foi possível abrir o banco de dados", error.to_string())
        })?;

        configure_sync(&sync_connection)?;

        // Regra de persistência: qualquer migração pendente em um banco já
        // existente ganha uma cópia antes de ser aplicada.
        if existed {
            let pending = migrations()
                .pending_migrations(&sync_connection)
                .map_err(|error| {
                    AppError::database(
                        "não foi possível verificar a versão do banco de dados",
                        error.to_string(),
                    )
                })?;

            if pending > 0 {
                backup_before_migration(&sync_connection, &path)?;
            }
        }

        migrations()
            .to_latest(&mut sync_connection)
            .map_err(|error| {
                AppError::database(
                    "não foi possível aplicar as migrações do banco de dados",
                    error.to_string(),
                )
            })?;
        drop(sync_connection);

        let connection = Connection::open(&path).await.map_err(|error| {
            AppError::database(
                "não foi possível abrir a conexão de trabalho do banco de dados",
                error.to_string(),
            )
        })?;

        configure_async(&connection).await?;

        Ok(Self { connection, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Substitui o conteúdo do banco atual por um backup, validando a cópia e
    /// aplicando migrações pendentes depois da restauração.
    pub async fn restore_from(&self, source: &Path) -> Result<(), AppError> {
        if source == self.path {
            return Err(AppError::validation(
                "o arquivo de backup é o próprio banco em uso",
            ));
        }

        let source_connection = SyncConnection::open_with_flags(
            source,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| {
            AppError::io(
                "não foi possível abrir o backup para restauração",
                error.to_string(),
            )
        })?;

        validate_backup(&source_connection)?;
        drop(source_connection);

        let source = source.to_path_buf();

        self.connection
            .call(move |connection| {
                let source_connection = SyncConnection::open_with_flags(
                    &source,
                    OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
                )?;

                {
                    let backup = rusqlite::backup::Backup::new(&source_connection, connection)?;
                    backup.run_to_completion(100, Duration::from_millis(0), None)?;
                }

                migrations().to_latest(connection).map_err(|error| {
                    rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
                        Some(error.to_string()),
                    )
                })?;
                connection.execute_batch(
                    "PRAGMA foreign_keys=ON;
                     PRAGMA busy_timeout=5000;",
                )?;

                Ok(())
            })
            .await
            .map_err(map_async_error)
    }

    pub async fn call<F, R>(&self, function: F) -> Result<R, AppError>
    where
        F: FnOnce(&mut rusqlite::Connection) -> rusqlite::Result<R> + Send + 'static,
        R: Send + 'static,
    {
        self.connection
            .call(function)
            .await
            .map_err(map_async_error)
    }

    /// Executa uma mutation em uma única transação.
    ///
    /// O rollback acontece automaticamente quando o closure retorna erro.
    pub async fn transaction<F, R>(&self, function: F) -> Result<R, AppError>
    where
        F: FnOnce(&mut rusqlite::Transaction<'_>) -> Result<R, AppError> + Send + 'static,
        R: Send + 'static,
    {
        self.connection
            .call(move |connection| {
                let mut transaction = connection.transaction()?;
                let result = function(&mut transaction);

                match result {
                    Ok(value) => {
                        transaction.commit()?;
                        Ok(Ok(value))
                    }
                    Err(error) => Ok(Err(error)),
                }
            })
            .await
            .map_err(map_async_error)?
    }
}

fn map_async_error(error: tokio_rusqlite::Error) -> AppError {
    match error {
        tokio_rusqlite::Error::Error(error) => map_sqlite_error(error),
        other => AppError::database(
            "a conexão do banco de dados não está disponível",
            other.to_string(),
        ),
    }
}

pub(crate) fn map_sqlite_error(error: rusqlite::Error) -> AppError {
    if let rusqlite::Error::SqliteFailure(code, message) = &error
        && code.code == rusqlite::ErrorCode::ConstraintViolation
    {
        let message = message.as_deref().unwrap_or_default();

        if message.contains("accounts.name") {
            return AppError::conflict("já existe uma conta com esse nome");
        }

        if message.contains("categories.name") {
            return AppError::conflict("já existe uma categoria com esse nome e tipo");
        }

        if message.contains("FOREIGN KEY constraint failed") {
            return AppError::conflict("o registro está em uso e não pode ser excluído");
        }

        tracing::warn!(error = %error, "violação de constraint no SQLite");
        return AppError::validation("os dados informados violam uma regra do banco");
    }

    AppError::database(
        "falha ao executar operação no banco de dados",
        error.to_string(),
    )
}

fn configure_sync(connection: &SyncConnection) -> Result<(), AppError> {
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=5000;
             PRAGMA wal_autocheckpoint=1000;",
        )
        .map_err(|error| {
            AppError::database("não foi possível configurar o SQLite", error.to_string())
        })
}

async fn configure_async(connection: &Connection) -> Result<(), AppError> {
    connection
        .call(|connection| {
            connection.execute_batch(
                "PRAGMA synchronous=FULL;
                 PRAGMA foreign_keys=ON;
                 PRAGMA busy_timeout=5000;",
            )
        })
        .await
        .map_err(|error| {
            AppError::database(
                "não foi possível configurar a conexão de trabalho do SQLite",
                error.to_string(),
            )
        })
}

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(include_str!("../migrations/0001_init.sql")),
        M::up(include_str!("../migrations/0002_cards.sql")),
        M::up(include_str!("../migrations/0003_category_icons.sql")),
        M::up(include_str!("../migrations/0004_savings.sql")),
        M::up(include_str!(
            "../migrations/0005_savings_opening_balance.sql"
        )),
        M::up(include_str!("../migrations/0006_savings_adjustment.sql")),
        M::up(include_str!("../migrations/0007_cdi_rates.sql")),
    ])
}

fn backup_before_migration(connection: &SyncConnection, db_path: &Path) -> Result<(), AppError> {
    let directory = db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups");
    std::fs::create_dir_all(&directory).map_err(|error| {
        AppError::io(
            "não foi possível criar o diretório de backups",
            error.to_string(),
        )
    })?;

    let file_name = format!(
        "tally-pre-migration-{}.db",
        Local::now().format("%Y%m%d-%H%M%S-%3f")
    );
    let target = directory.join(file_name);
    let target_path = target.to_string_lossy().to_string();

    connection
        .execute("VACUUM INTO ?1", rusqlite::params![target_path])
        .map_err(|error| {
            AppError::io(
                "não foi possível criar o backup antes da migração",
                error.to_string(),
            )
        })?;

    Ok(())
}

fn validate_backup(connection: &SyncConnection) -> Result<(), AppError> {
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| {
            AppError::integrity(
                "não foi possível verificar a integridade do backup",
                error.to_string(),
            )
        })?;

    if integrity != "ok" {
        return Err(AppError::integrity(
            "o backup não passou na verificação de integridade",
            integrity,
        ));
    }

    let core_tables: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type = 'table'
               AND name IN ('accounts', 'journal_entries', 'postings')",
            [],
            |row| row.get(0),
        )
        .map_err(|error| {
            AppError::integrity("não foi possível ler o schema do backup", error.to_string())
        })?;

    if core_tables < 3 {
        return Err(AppError::validation(
            "o arquivo não parece ser um backup do Tally",
        ));
    }

    let mut foreign_keys = connection
        .prepare("PRAGMA foreign_key_check")
        .map_err(|error| {
            AppError::integrity(
                "não foi possível verificar as referências do backup",
                error.to_string(),
            )
        })?;

    if foreign_keys.exists([]).map_err(|error| {
        AppError::integrity(
            "não foi possível verificar as referências do backup",
            error.to_string(),
        )
    })? {
        return Err(AppError::integrity(
            "o backup tem referências quebradas",
            "foreign_key_check encontrou violações",
        ));
    }
    drop(foreign_keys);

    let pending = migrations()
        .pending_migrations(connection)
        .map_err(|error| {
            AppError::database("não foi possível ler a versão do backup", error.to_string())
        })?;

    if pending < 0 {
        return Err(AppError::validation(
            "o backup foi criado por uma versão mais nova do Tally",
        ));
    }

    Ok(())
}
