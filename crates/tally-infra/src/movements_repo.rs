use async_trait::async_trait;
use rusqlite::{OptionalExtension, Row, params};
use tally_app::dto::{
    AccountDto, AccountStatementDto, MovementDto, MovementFilter, StatementLineDto,
};
use tally_app::error::AppError;
use tally_app::movements::{MovementsRepository, NewMovement};
use tally_domain::movement::MovementKind;

use crate::db::{Database, map_sqlite_error};

const SELECT_ACCOUNT: &str = "
    SELECT a.id, a.name, a.type, a.credit_limit_cents, a.archived_at, a.created_at,
           COALESCE((
               SELECT SUM(p.amount_cents)
               FROM postings p
               JOIN journal_entries e ON e.id = p.entry_id
               WHERE p.account_id = a.id AND e.posted_at IS NOT NULL
           ), 0) AS balance_cents
    FROM accounts a
    WHERE a.is_system = 0
";

const SELECT_MOVEMENT: &str = "
    SELECT e.id, e.kind, e.occurred_on, e.occurred_time, e.manual_period, e.period,
           e.description, e.primary_account_id, pa.name,
           e.counterparty_account_id, ca.name, e.category_id, c.name,
           e.payment_method,
           COALESCE((
               SELECT SUM(p.amount_cents)
               FROM postings p
               WHERE p.entry_id = e.id AND p.account_id = e.primary_account_id
           ), 0) AS amount_cents,
           e.created_at,
           ip.id, ip.installment_count,
           (SELECT i.amount_cents
            FROM installments i
            WHERE i.plan_id = ip.id
            ORDER BY i.seq
            LIMIT 1) AS first_installment_cents
    FROM journal_entries e
    JOIN accounts pa ON pa.id = e.primary_account_id
    LEFT JOIN accounts ca ON ca.id = e.counterparty_account_id
    LEFT JOIN categories c ON c.id = e.category_id
    LEFT JOIN installment_plans ip ON ip.purchase_entry_id = e.id
    WHERE e.posted_at IS NOT NULL AND e.kind <> 'opening_balance'
      AND NOT EXISTS (
          SELECT 1 FROM installments i WHERE i.recognition_entry_id = e.id
      )
";

#[derive(Clone)]
pub struct SqliteMovementsRepository {
    db: Database,
}

impl SqliteMovementsRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl MovementsRepository for SqliteMovementsRepository {
    async fn entry_kind(&self, id: &str) -> Result<String, AppError> {
        let id = id.to_owned();
        self.db
            .call(move |connection| {
                connection
                    .query_row(
                        "SELECT kind FROM journal_entries WHERE id = ?1",
                        [id],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
            })
            .await?
            .ok_or_else(|| AppError::not_found("movimentação não encontrada"))
    }

    async fn create(&self, movement: NewMovement) -> Result<MovementDto, AppError> {
        self.db
            .transaction(move |transaction| {
                validate_references(transaction, &movement, false, None, None)?;
                insert_entry(transaction, &movement)?;
                post_entry(transaction, &movement.id)?;
                fetch_movement(transaction, &movement.id)
            })
            .await
    }

    async fn update(&self, id: &str, mut movement: NewMovement) -> Result<MovementDto, AppError> {
        let id = id.to_owned();
        movement.id.clone_from(&id);

        self.db
            .transaction(move |transaction| {
                let existing = transaction
                    .query_row(
                        "SELECT kind, primary_account_id, counterparty_account_id, category_id
                         FROM journal_entries WHERE id = ?1",
                        [&id],
                        |row| {
                            Ok((
                                row.get::<_, String>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, Option<String>>(2)?,
                                row.get::<_, Option<String>>(3)?,
                            ))
                        },
                    )
                    .optional()
                    .map_err(map_sqlite_error)?
                    .ok_or_else(|| AppError::not_found("movimentação não encontrada"))?;

                let existing_kind = MovementKind::parse(&existing.0).ok_or_else(|| {
                    AppError::conflict("esta entrada interna não pode ser editada")
                })?;
                if existing_kind != movement.kind {
                    return Err(AppError::validation(
                        "o tipo de uma movimentação não pode ser alterado",
                    ));
                }

                validate_references(
                    transaction,
                    &movement,
                    true,
                    Some((&existing.1, existing.2.as_deref())),
                    existing.3.as_deref(),
                )?;

                transaction
                    .execute(
                        "UPDATE journal_entries SET posted_at = NULL WHERE id = ?1",
                        [&id],
                    )
                    .map_err(map_sqlite_error)?;

                transaction
                    .execute("DELETE FROM postings WHERE entry_id = ?1", [&id])
                    .map_err(map_sqlite_error)?;

                update_entry(transaction, &id, &movement)?;
                insert_postings(transaction, &movement)?;
                post_entry(transaction, &id)?;
                fetch_movement(transaction, &id)
            })
            .await
    }

    async fn delete(&self, id: &str) -> Result<(), AppError> {
        let id = id.to_owned();

        self.db
            .transaction(move |transaction| {
                let kind = transaction
                    .query_row(
                        "SELECT kind FROM journal_entries WHERE id = ?1",
                        [&id],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(map_sqlite_error)?
                    .ok_or_else(|| AppError::not_found("movimentação não encontrada"))?;

                MovementKind::parse(&kind).ok_or_else(|| {
                    AppError::conflict("esta entrada interna não pode ser excluída")
                })?;

                transaction
                    .execute(
                        "UPDATE journal_entries SET posted_at = NULL WHERE id = ?1",
                        [&id],
                    )
                    .map_err(map_sqlite_error)?;
                transaction
                    .execute("DELETE FROM journal_entries WHERE id = ?1", [&id])
                    .map_err(map_sqlite_error)?;

                Ok(())
            })
            .await
    }

    async fn list(&self, filter: MovementFilter) -> Result<Vec<MovementDto>, AppError> {
        let sql = format!(
            "{SELECT_MOVEMENT}
             AND (?1 IS NULL OR e.primary_account_id = ?1 OR e.counterparty_account_id = ?1)
             AND (?2 IS NULL OR e.category_id = ?2)
             AND (?3 IS NULL OR e.kind = ?3)
             AND (?4 IS NULL OR e.period = ?4)
             AND (?5 IS NULL OR e.payment_method = ?5)
             AND (?6 IS NULL OR e.occurred_on >= ?6)
             AND (?7 IS NULL OR e.occurred_on <= ?7)
             ORDER BY e.occurred_on DESC, e.created_at DESC, e.id DESC"
        );
        let account_id = filter.account_id;
        let category_id = filter.category_id;
        let kind = filter.kind;
        let period = filter.period;
        let payment_method = filter.payment_method;
        let from = filter.from;
        let to = filter.to;

        self.db
            .call(move |connection| {
                let mut statement = connection.prepare(&sql)?;
                let rows = statement.query_map(
                    params![
                        account_id.as_deref(),
                        category_id.as_deref(),
                        kind.as_deref(),
                        period.as_deref(),
                        payment_method.as_deref(),
                        from.as_deref(),
                        to.as_deref()
                    ],
                    map_movement_row,
                )?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await
    }

    async fn statement(
        &self,
        account_id: &str,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<AccountStatementDto, AppError> {
        let account_id = account_id.to_owned();
        let from = from.map(str::to_owned);
        let to = to.map(str::to_owned);

        let statement = self
            .db
            .call(move |connection| {
                let account_sql = format!("{SELECT_ACCOUNT} AND a.id = ?1");
                let Some(account) = connection
                    .query_row(&account_sql, [&account_id], map_account_row)
                    .optional()?
                else {
                    return Ok(None);
                };

                let opening_balance_cents = match from.as_deref() {
                    Some(from) => connection.query_row(
                        "SELECT COALESCE(SUM(p.amount_cents), 0)
                         FROM postings p
                         JOIN journal_entries e ON e.id = p.entry_id
                         WHERE p.account_id = ?1
                           AND e.posted_at IS NOT NULL
                           AND e.occurred_on < ?2",
                        params![account_id, from],
                        |row| row.get::<_, i64>(0),
                    )?,
                    None => 0,
                };

                let mut statement = connection.prepare(
                    "SELECT e.id, e.kind, e.occurred_on, e.occurred_time, e.period,
                            e.description, e.category_id, c.name, ca.name,
                            SUM(p.amount_cents)
                     FROM journal_entries e
                     JOIN postings p ON p.entry_id = e.id AND p.account_id = ?1
                     LEFT JOIN categories c ON c.id = e.category_id
                     LEFT JOIN accounts ca ON ca.id = e.counterparty_account_id
                     WHERE e.posted_at IS NOT NULL
                       AND (?2 IS NULL OR e.occurred_on >= ?2)
                       AND (?3 IS NULL OR e.occurred_on <= ?3)
                     GROUP BY e.id
                     ORDER BY e.occurred_on, e.created_at, e.id",
                )?;

                let rows = statement.query_map(
                    params![account_id, from.as_deref(), to.as_deref()],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?,
                            row.get::<_, Option<String>>(5)?,
                            row.get::<_, Option<String>>(6)?,
                            row.get::<_, Option<String>>(7)?,
                            row.get::<_, Option<String>>(8)?,
                            row.get::<_, i64>(9)?,
                        ))
                    },
                )?;

                let mut running_balance_cents = opening_balance_cents;
                let mut lines = Vec::new();

                for row in rows {
                    let (
                        entry_id,
                        kind,
                        occurred_on,
                        occurred_time,
                        period,
                        description,
                        category_id,
                        category_name,
                        counterparty_account_name,
                        amount_cents,
                    ) = row?;
                    let Some(next_balance) = running_balance_cents.checked_add(amount_cents) else {
                        return Err(rusqlite::Error::InvalidQuery);
                    };
                    running_balance_cents = next_balance;

                    lines.push(StatementLineDto {
                        entry_id,
                        kind,
                        occurred_on,
                        occurred_time,
                        period,
                        description,
                        category_id,
                        category_name,
                        counterparty_account_name,
                        amount_cents,
                        running_balance_cents,
                    });
                }

                Ok(Some(AccountStatementDto {
                    account,
                    from,
                    to,
                    opening_balance_cents,
                    closing_balance_cents: running_balance_cents,
                    lines,
                }))
            })
            .await?;

        statement.ok_or_else(|| AppError::not_found("conta não encontrada"))
    }
}

fn validate_references(
    transaction: &rusqlite::Transaction<'_>,
    movement: &NewMovement,
    update: bool,
    existing_accounts: Option<(&str, Option<&str>)>,
    existing_category: Option<&str>,
) -> Result<(), AppError> {
    let allow_primary = update
        && existing_accounts.is_some_and(|(primary, _)| primary == movement.primary_account_id);
    ensure_account(transaction, &movement.primary_account_id, allow_primary)?;

    if let Some(counterparty) = &movement.counterparty_account_id {
        let allow = update
            && existing_accounts
                .is_some_and(|(_, existing)| existing == Some(counterparty.as_str()));
        ensure_account(transaction, counterparty, allow)?;
    }

    if let Some(category_id) = &movement.category_id {
        let allow = update && existing_category.is_some_and(|existing| existing == category_id);
        ensure_category(transaction, category_id, movement.kind, allow)?;
    }

    Ok(())
}

fn ensure_account(
    transaction: &rusqlite::Transaction<'_>,
    id: &str,
    allow_archived: bool,
) -> Result<(), AppError> {
    let account = transaction
        .query_row(
            "SELECT is_system, archived_at FROM accounts WHERE id = ?1",
            [id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(map_sqlite_error)?
        .ok_or_else(|| AppError::not_found("conta não encontrada"))?;

    if account.0 == 1 {
        return Err(AppError::validation(
            "contas internas não podem ser usadas diretamente",
        ));
    }

    if account.1.is_some() && !allow_archived {
        return Err(AppError::validation("a conta está arquivada"));
    }

    Ok(())
}

fn ensure_category(
    transaction: &rusqlite::Transaction<'_>,
    id: &str,
    movement_kind: MovementKind,
    allow_archived: bool,
) -> Result<(), AppError> {
    let category = transaction
        .query_row(
            "SELECT kind, archived_at FROM categories WHERE id = ?1",
            [id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(map_sqlite_error)?
        .ok_or_else(|| AppError::not_found("categoria não encontrada"))?;

    let expected = match movement_kind {
        MovementKind::Expense => "expense",
        MovementKind::Income => "income",
        MovementKind::Transfer | MovementKind::Adjustment => {
            return Err(AppError::validation(
                "este tipo de movimentação não aceita categoria",
            ));
        }
    };

    if category.0 != expected {
        return Err(AppError::validation(
            "a categoria não corresponde ao tipo da movimentação",
        ));
    }

    if category.1.is_some() && !allow_archived {
        return Err(AppError::validation("a categoria está arquivada"));
    }

    Ok(())
}

fn insert_entry(
    transaction: &rusqlite::Transaction<'_>,
    movement: &NewMovement,
) -> Result<(), AppError> {
    transaction
        .execute(
            "INSERT INTO journal_entries (
                id, kind, occurred_on, occurred_time, manual_period, description,
                primary_account_id, counterparty_account_id, category_id, payment_method,
                posted_at, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![
                movement.id,
                movement.kind.as_str(),
                movement.occurred_on,
                movement.occurred_time,
                movement.manual_period,
                movement.description,
                movement.primary_account_id,
                movement.counterparty_account_id,
                movement.category_id,
                movement.payment_method.map(|method| method.as_str()),
            ],
        )
        .map_err(map_sqlite_error)?;

    insert_postings(transaction, movement)
}

fn update_entry(
    transaction: &rusqlite::Transaction<'_>,
    id: &str,
    movement: &NewMovement,
) -> Result<(), AppError> {
    let changed = transaction
        .execute(
            "UPDATE journal_entries
             SET kind = ?1, occurred_on = ?2, occurred_time = ?3, manual_period = ?4,
                 description = ?5, primary_account_id = ?6, counterparty_account_id = ?7,
                 category_id = ?8, payment_method = ?9,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?10",
            params![
                movement.kind.as_str(),
                movement.occurred_on,
                movement.occurred_time,
                movement.manual_period,
                movement.description,
                movement.primary_account_id,
                movement.counterparty_account_id,
                movement.category_id,
                movement.payment_method.map(|method| method.as_str()),
                id,
            ],
        )
        .map_err(map_sqlite_error)?;

    if changed == 0 {
        return Err(AppError::not_found("movimentação não encontrada"));
    }

    Ok(())
}

fn insert_postings(
    transaction: &rusqlite::Transaction<'_>,
    movement: &NewMovement,
) -> Result<(), AppError> {
    for posting in &movement.postings {
        transaction
            .execute(
                "INSERT INTO postings (id, entry_id, account_id, amount_cents, created_at)
                 VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                params![
                    posting.id,
                    movement.id,
                    posting.account_id,
                    posting.amount.cents()
                ],
            )
            .map_err(map_sqlite_error)?;
    }

    Ok(())
}

fn post_entry(transaction: &rusqlite::Transaction<'_>, id: &str) -> Result<(), AppError> {
    let changed = transaction
        .execute(
            "UPDATE journal_entries
             SET posted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND posted_at IS NULL",
            [id],
        )
        .map_err(map_sqlite_error)?;

    if changed == 0 {
        return Err(AppError::conflict(
            "não foi possível publicar a movimentação",
        ));
    }

    Ok(())
}

fn fetch_movement(connection: &rusqlite::Connection, id: &str) -> Result<MovementDto, AppError> {
    let sql = format!("{SELECT_MOVEMENT} AND e.id = ?1");
    connection
        .query_row(&sql, [id], map_movement_row)
        .optional()
        .map_err(map_sqlite_error)?
        .ok_or_else(|| AppError::not_found("movimentação não encontrada"))
}

fn map_movement_row(row: &Row<'_>) -> rusqlite::Result<MovementDto> {
    Ok(MovementDto {
        id: row.get(0)?,
        kind: row.get(1)?,
        occurred_on: row.get(2)?,
        occurred_time: row.get(3)?,
        manual_period: row.get(4)?,
        period: row.get(5)?,
        description: row.get(6)?,
        primary_account_id: row.get(7)?,
        primary_account_name: row.get(8)?,
        counterparty_account_id: row.get(9)?,
        counterparty_account_name: row.get(10)?,
        category_id: row.get(11)?,
        category_name: row.get(12)?,
        payment_method: row.get(13)?,
        amount_cents: row.get(14)?,
        created_at: row.get(15)?,
        installment_plan_id: row.get(16)?,
        installment_count: row.get(17)?,
        first_installment_cents: row.get(18)?,
    })
}

fn map_account_row(row: &Row<'_>) -> rusqlite::Result<AccountDto> {
    Ok(AccountDto {
        id: row.get(0)?,
        name: row.get(1)?,
        account_type: row.get(2)?,
        credit_limit_cents: row.get(3)?,
        archived_at: row.get(4)?,
        created_at: row.get(5)?,
        balance_cents: row.get(6)?,
    })
}
