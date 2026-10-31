use async_trait::async_trait;
use rusqlite::{OptionalExtension, Row, params};
use tally_app::categories::{CategoriesRepository, NewCategory};
use tally_app::dto::CategoryDto;
use tally_app::error::AppError;

use crate::db::{Database, map_sqlite_error};

const SELECT_CATEGORY: &str = "
    SELECT id, name, kind, archived_at, created_at
    FROM categories
    WHERE 1 = 1
";

#[derive(Clone)]
pub struct SqliteCategoriesRepository {
    db: Database,
}

impl SqliteCategoriesRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl CategoriesRepository for SqliteCategoriesRepository {
    async fn list(&self, include_archived: bool) -> Result<Vec<CategoryDto>, AppError> {
        let include_archived = i64::from(include_archived);
        let sql = format!(
            "{SELECT_CATEGORY} AND (?1 = 1 OR archived_at IS NULL) ORDER BY kind, name COLLATE NOCASE"
        );

        self.db
            .call(move |connection| {
                let mut statement = connection.prepare(&sql)?;
                let rows = statement.query_map([include_archived], map_category_row)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await
    }

    async fn create(&self, category: NewCategory) -> Result<CategoryDto, AppError> {
        self.db
            .transaction(move |transaction| {
                transaction
                    .execute(
                        "INSERT INTO categories (
                            id, name, kind, archived_at, created_at, updated_at
                         ) VALUES (
                            ?1, ?2, ?3, NULL,
                            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                         )",
                        params![category.id, category.name, category.kind.as_str()],
                    )
                    .map_err(map_sqlite_error)?;

                fetch_category(transaction, &category.id)
            })
            .await
    }

    async fn rename(&self, id: &str, name: &str) -> Result<CategoryDto, AppError> {
        let id = id.to_owned();
        let name = name.to_owned();

        self.db
            .transaction(move |transaction| {
                let changed = transaction
                    .execute(
                        "UPDATE categories
                         SET name = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                         WHERE id = ?2",
                        params![name, id],
                    )
                    .map_err(map_sqlite_error)?;

                if changed == 0 {
                    return Err(AppError::not_found("categoria não encontrada"));
                }

                fetch_category(transaction, &id)
            })
            .await
    }

    async fn set_archived(&self, id: &str, archived: bool) -> Result<CategoryDto, AppError> {
        let id = id.to_owned();

        self.db
            .transaction(move |transaction| {
                let changed = transaction
                    .execute(
                        "UPDATE categories
                         SET archived_at = CASE
                                 WHEN ?1 = 1 THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                                 ELSE NULL
                             END,
                             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                         WHERE id = ?2",
                        params![i64::from(archived), id],
                    )
                    .map_err(map_sqlite_error)?;

                if changed == 0 {
                    return Err(AppError::not_found("categoria não encontrada"));
                }

                fetch_category(transaction, &id)
            })
            .await
    }

    async fn delete(&self, id: &str) -> Result<(), AppError> {
        let id = id.to_owned();

        self.db
            .transaction(move |transaction| {
                let exists = transaction
                    .query_row("SELECT 1 FROM categories WHERE id = ?1", [&id], |row| {
                        row.get::<_, i64>(0)
                    })
                    .optional()
                    .map_err(map_sqlite_error)?;

                if exists.is_none() {
                    return Err(AppError::not_found("categoria não encontrada"));
                }

                let history = transaction
                    .query_row(
                        "SELECT COUNT(*) FROM journal_entries WHERE category_id = ?1",
                        [&id],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(map_sqlite_error)?;

                if history > 0 {
                    return Err(AppError::conflict(
                        "a categoria possui histórico; arquive em vez de excluir",
                    ));
                }

                transaction
                    .execute("DELETE FROM categories WHERE id = ?1", [&id])
                    .map_err(map_sqlite_error)?;

                Ok(())
            })
            .await
    }
}

fn fetch_category(connection: &rusqlite::Connection, id: &str) -> Result<CategoryDto, AppError> {
    let sql = format!("{SELECT_CATEGORY} AND id = ?1");
    connection
        .query_row(&sql, [id], map_category_row)
        .optional()
        .map_err(map_sqlite_error)?
        .ok_or_else(|| AppError::not_found("categoria não encontrada"))
}

fn map_category_row(row: &Row<'_>) -> rusqlite::Result<CategoryDto> {
    Ok(CategoryDto {
        id: row.get(0)?,
        name: row.get(1)?,
        kind: row.get(2)?,
        archived_at: row.get(3)?,
        created_at: row.get(4)?,
    })
}
