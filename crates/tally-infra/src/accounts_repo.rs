use async_trait::async_trait;
use rusqlite::{OptionalExtension, Row, params};
use tally_app::accounts::{AccountsRepository, NewAccount, OpeningBalance};
use tally_app::dto::AccountDto;
use tally_app::error::AppError;

use crate::db::{Database, map_sqlite_error};

const SELECT_ACCOUNT: &str = "
    SELECT a.id, a.name, a.type, a.archived_at, a.created_at,
           COALESCE(b.balance_cents, 0) AS balance_cents
    FROM accounts a
    LEFT JOIN (
        SELECT p.account_id, SUM(p.amount_cents) AS balance_cents
        FROM postings p
        JOIN journal_entries e ON e.id = p.entry_id
        WHERE e.posted_at IS NOT NULL
        GROUP BY p.account_id
    ) b ON b.account_id = a.id
    WHERE a.is_system = 0
";

#[derive(Clone)]
pub struct SqliteAccountsRepository {
    db: Database,
}

impl SqliteAccountsRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AccountsRepository for SqliteAccountsRepository {
    async fn list(&self, include_archived: bool) -> Result<Vec<AccountDto>, AppError> {
        let include_archived = i64::from(include_archived);
        let sql = format!(
            "{SELECT_ACCOUNT} AND (?1 = 1 OR a.archived_at IS NULL) ORDER BY a.name COLLATE NOCASE"
        );

        self.db
            .call(move |connection| {
                let mut statement = connection.prepare(&sql)?;
                let rows = statement.query_map([include_archived], map_account_row)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await
    }

    async fn create(&self, account: NewAccount) -> Result<AccountDto, AppError> {
        self.db
            .transaction(move |transaction| {
                transaction
                    .execute(
                        "INSERT INTO accounts (
                            id, name, class, type, is_system, credit_limit_cents,
                            archived_at, created_at, updated_at
                         ) VALUES (
                            ?1, ?2, 'asset', ?3, 0, NULL, NULL,
                            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                            strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                         )",
                        params![account.id, account.name, account.account_type.as_str()],
                    )
                    .map_err(map_sqlite_error)?;

                if let Some(entry) = &account.opening_entry {
                    insert_opening_balance(transaction, &account.id, entry)?;
                }

                fetch_account(transaction, &account.id)
            })
            .await
    }

    async fn rename(&self, id: &str, name: &str) -> Result<AccountDto, AppError> {
        let id = id.to_owned();
        let name = name.to_owned();

        self.db
            .transaction(move |transaction| {
                let changed = transaction
                    .execute(
                        "UPDATE accounts
                         SET name = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                         WHERE id = ?2 AND is_system = 0",
                        params![name, id],
                    )
                    .map_err(map_sqlite_error)?;

                if changed == 0 {
                    return Err(AppError::not_found("conta não encontrada"));
                }

                fetch_account(transaction, &id)
            })
            .await
    }

    async fn set_archived(&self, id: &str, archived: bool) -> Result<AccountDto, AppError> {
        let id = id.to_owned();

        self.db
            .transaction(move |transaction| {
                let changed = transaction
                    .execute(
                        "UPDATE accounts
                         SET archived_at = CASE
                                 WHEN ?1 = 1 THEN strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                                 ELSE NULL
                             END,
                             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                         WHERE id = ?2 AND is_system = 0",
                        params![i64::from(archived), id],
                    )
                    .map_err(map_sqlite_error)?;

                if changed == 0 {
                    return Err(AppError::not_found("conta não encontrada"));
                }

                fetch_account(transaction, &id)
            })
            .await
    }

    async fn delete(&self, id: &str) -> Result<(), AppError> {
        let id = id.to_owned();

        self.db
            .transaction(move |transaction| {
                let exists = transaction
                    .query_row(
                        "SELECT 1 FROM accounts WHERE id = ?1 AND is_system = 0",
                        [&id],
                        |row| row.get::<_, i64>(0),
                    )
                    .optional()
                    .map_err(map_sqlite_error)?;

                if exists.is_none() {
                    return Err(AppError::not_found("conta não encontrada"));
                }

                let history = transaction
                    .query_row(
                        "SELECT COUNT(*)
                         FROM postings p
                         JOIN journal_entries e ON e.id = p.entry_id
                         WHERE p.account_id = ?1 AND e.kind <> 'opening_balance'",
                        [&id],
                        |row| row.get::<_, i64>(0),
                    )
                    .map_err(map_sqlite_error)?;

                if history > 0 {
                    return Err(AppError::conflict(
                        "a conta possui movimentações; arquive em vez de excluir",
                    ));
                }

                transaction
                    .execute(
                        "UPDATE journal_entries
                         SET posted_at = NULL
                         WHERE kind = 'opening_balance'
                           AND (primary_account_id = ?1 OR counterparty_account_id = ?1)
                           AND posted_at IS NOT NULL",
                        [&id],
                    )
                    .map_err(map_sqlite_error)?;

                transaction
                    .execute(
                        "DELETE FROM journal_entries
                         WHERE kind = 'opening_balance'
                           AND (primary_account_id = ?1 OR counterparty_account_id = ?1)",
                        [&id],
                    )
                    .map_err(map_sqlite_error)?;

                transaction
                    .execute(
                        "DELETE FROM accounts WHERE id = ?1 AND is_system = 0",
                        [&id],
                    )
                    .map_err(map_sqlite_error)?;

                Ok(())
            })
            .await
    }
}

fn insert_opening_balance(
    transaction: &mut rusqlite::Transaction<'_>,
    account_id: &str,
    entry: &OpeningBalance,
) -> Result<(), AppError> {
    transaction
        .execute(
            "INSERT INTO journal_entries (
                id, kind, occurred_on, occurred_time, manual_period, description,
                primary_account_id, counterparty_account_id, category_id, payment_method,
                posted_at, created_at, updated_at
             ) VALUES (
                ?1, 'opening_balance', ?2, NULL, NULL, 'Saldo inicial',
                ?3, NULL, NULL, NULL, NULL,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![entry.entry_id, entry.occurred_on, account_id],
        )
        .map_err(map_sqlite_error)?;

    for posting in &entry.postings {
        transaction
            .execute(
                "INSERT INTO postings (id, entry_id, account_id, amount_cents, created_at)
                 VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                params![
                    posting.id,
                    entry.entry_id,
                    posting.account_id,
                    posting.amount.cents()
                ],
            )
            .map_err(map_sqlite_error)?;
    }

    transaction
        .execute(
            "UPDATE journal_entries
             SET posted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1",
            [&entry.entry_id],
        )
        .map_err(map_sqlite_error)?;

    Ok(())
}

fn fetch_account(connection: &rusqlite::Connection, id: &str) -> Result<AccountDto, AppError> {
    let sql = format!("{SELECT_ACCOUNT} AND a.id = ?1");
    connection
        .query_row(&sql, [id], map_account_row)
        .optional()
        .map_err(map_sqlite_error)?
        .ok_or_else(|| AppError::not_found("conta não encontrada"))
}

fn map_account_row(row: &Row<'_>) -> rusqlite::Result<AccountDto> {
    Ok(AccountDto {
        id: row.get(0)?,
        name: row.get(1)?,
        account_type: row.get(2)?,
        archived_at: row.get(3)?,
        created_at: row.get(4)?,
        balance_cents: row.get(5)?,
    })
}
