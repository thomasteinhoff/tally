use std::path::{Path, PathBuf};

use rusqlite::Connection as SyncConnection;
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

        let mut sync_connection = SyncConnection::open(&path).map_err(|error| {
            AppError::database("não foi possível abrir o banco de dados", error.to_string())
        })?;

        configure_sync(&sync_connection)?;
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
    Migrations::new(vec![M::up(include_str!("../migrations/0001_init.sql"))])
}
