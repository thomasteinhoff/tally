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
        self.connection.call(function).await.map_err(|error| {
            AppError::database(
                "falha ao executar operação no banco de dados",
                error.to_string(),
            )
        })
    }
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
