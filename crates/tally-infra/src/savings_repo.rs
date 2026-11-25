use async_trait::async_trait;
use rusqlite::{OptionalExtension, Row, params};
use tally_app::dto::{AccountDto, SavingsYieldDto};
use tally_app::error::AppError;
use tally_app::savings::{CDI_SERIES, NewSavingsReversal, NewSavingsYield, SavingsRepository};
use uuid::Uuid;

use crate::db::{Database, map_sqlite_error};

/// Conta interna "Receitas": o rendimento entra como ajuste, nunca como receita
/// de categoria, e por isso não aparece em receitas/despesas nem no fluxo de caixa.
const INCOME_ACCOUNT_ID: &str = "00000000-0000-7000-8000-000000000002";

const SELECT_ACCOUNT: &str = "
    SELECT a.id, a.name, a.type, a.credit_limit_cents, a.is_savings, a.yield_mode,
           a.cdi_percent_bps, a.archived_at, a.created_at,
           COALESCE((
               SELECT SUM(p.amount_cents)
               FROM postings p
               JOIN journal_entries e ON e.id = p.entry_id
               WHERE p.account_id = a.id AND e.posted_at IS NOT NULL
           ), 0) AS balance_cents
    FROM accounts a
    WHERE a.is_system = 0
";

const SELECT_YIELD: &str = "
    SELECT sy.id, sy.account_id, sy.as_of, sy.mode, sy.cdi_annual_bps,
           sy.contracted_percent_bps, sy.daily_rate, sy.days, sy.base_cents,
           sy.yield_cents, sy.status, sy.entry_id, sy.reversal_entry_id, sy.created_at,
           sy.rate_date, sy.rate_series, sy.cdi_daily_rate, sy.rate_fetched_at
    FROM savings_yields sy
";

#[derive(Clone)]
pub struct SqliteSavingsRepository {
    db: Database,
}

impl SqliteSavingsRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SavingsRepository for SqliteSavingsRepository {
    async fn savings_account(&self, id: &str) -> Result<AccountDto, AppError> {
        let id = id.to_owned();
        self.db
            .call(move |connection| {
                let sql = format!("{SELECT_ACCOUNT} AND a.id = ?1");
                connection.query_row(&sql, [id], map_account_row).optional()
            })
            .await?
            .ok_or_else(|| AppError::not_found("cofrinho não encontrado"))
    }

    async fn balance_through(&self, account_id: &str, as_of: &str) -> Result<i64, AppError> {
        let account_id = account_id.to_owned();
        let as_of = as_of.to_owned();
        self.db
            .call(move |connection| {
                connection.query_row(
                    "SELECT COALESCE(SUM(p.amount_cents), 0)
                         FROM postings p
                         JOIN journal_entries e ON e.id = p.entry_id
                         WHERE p.account_id = ?1
                           AND e.posted_at IS NOT NULL
                           AND e.occurred_on <= ?2",
                    params![account_id, as_of],
                    |row| row.get::<_, i64>(0),
                )
            })
            .await
    }

    async fn current_balance(&self, account_id: &str) -> Result<i64, AppError> {
        let account_id = account_id.to_owned();
        self.db
            .call(move |connection| {
                connection.query_row(
                    "SELECT COALESCE(SUM(p.amount_cents), 0)
                         FROM postings p
                         JOIN journal_entries e ON e.id = p.entry_id
                         WHERE p.account_id = ?1 AND e.posted_at IS NOT NULL",
                    [account_id],
                    |row| row.get::<_, i64>(0),
                )
            })
            .await
    }

    async fn create_yield(&self, snapshot: NewSavingsYield) -> Result<SavingsYieldDto, AppError> {
        let id = snapshot.id.clone();
        self.db
            .transaction(move |transaction| {
                insert_yield(transaction, snapshot)?;
                require_yield(transaction, &id)
            })
            .await
    }

    async fn reverse_yield(
        &self,
        yield_id: &str,
        reversal: NewSavingsReversal,
    ) -> Result<SavingsYieldDto, AppError> {
        let yield_id = yield_id.to_owned();
        self.db
            .transaction(move |transaction| {
                insert_reversal(transaction, &yield_id, reversal)?;
                require_yield(transaction, &yield_id)
            })
            .await
    }

    async fn replace_yield(
        &self,
        yield_id: &str,
        reversal: NewSavingsReversal,
        snapshot: NewSavingsYield,
    ) -> Result<SavingsYieldDto, AppError> {
        let yield_id = yield_id.to_owned();
        let replacement_id = snapshot.id.clone();
        self.db
            .transaction(move |transaction| {
                insert_reversal(transaction, &yield_id, reversal)?;
                insert_yield(transaction, snapshot)?;
                require_yield(transaction, &replacement_id)
            })
            .await
    }

    async fn find_yield(&self, yield_id: &str) -> Result<SavingsYieldDto, AppError> {
        let yield_id = yield_id.to_owned();
        self.db
            .call(move |connection| fetch_yield(connection, &yield_id))
            .await?
            .ok_or_else(|| AppError::not_found("rendimento não encontrado"))
    }

    async fn list_yields(&self, account_id: &str) -> Result<Vec<SavingsYieldDto>, AppError> {
        let account_id = account_id.to_owned();
        self.db
            .call(move |connection| {
                let sql = format!(
                    "{SELECT_YIELD} WHERE sy.account_id = ?1 \
                     ORDER BY sy.as_of DESC, sy.created_at DESC, sy.rowid DESC"
                );
                let mut statement = connection.prepare(&sql)?;
                let rows = statement.query_map([account_id], map_yield_row)?;
                rows.collect::<rusqlite::Result<Vec<_>>>()
            })
            .await
    }
}

/// A coluna legada `cdi_annual_bps` guarda a taxa anual equivalente à taxa
/// diária publicada, capitalizada na base de 252 dias úteis.
fn cdi_annual_bps(daily_rate: f64) -> i64 {
    ((1.0 + daily_rate).powf(252.0) - 1.0).mul_add(10_000.0, 0.5) as i64
}

fn insert_yield(
    transaction: &rusqlite::Transaction<'_>,
    snapshot: NewSavingsYield,
) -> Result<(), AppError> {
    // FK deferida: o snapshot entra antes do entry para que os triggers de
    // cofrinho reconheçam o lançamento como rendimento.
    transaction
        .execute(
            "INSERT INTO savings_yields (
                id, account_id, as_of, mode, cdi_annual_bps,
                contracted_percent_bps, daily_rate, days, base_cents,
                yield_cents, status, entry_id, created_at, updated_at,
                rate_date, rate_series, cdi_daily_rate, rate_fetched_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                ?10, 'confirmed', ?11,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                ?12, ?13, ?14, ?15
             )",
            params![
                snapshot.id,
                snapshot.account_id,
                snapshot.as_of,
                snapshot.mode.as_str(),
                cdi_annual_bps(snapshot.daily_rate),
                snapshot.contracted_percent_bps,
                snapshot.daily_rate,
                snapshot.days,
                snapshot.base_cents,
                snapshot.yield_cents,
                snapshot.entry_id,
                snapshot.rate.as_ref().map(|rate| rate.rate_date.as_str()),
                snapshot.rate.as_ref().map(|_| CDI_SERIES),
                snapshot.rate.as_ref().map(|rate| rate.daily_rate),
                snapshot.rate.as_ref().map(|rate| rate.fetched_at.as_str()),
            ],
        )
        .map_err(map_sqlite_error)?;

    insert_entry(
        transaction,
        &snapshot.entry_id,
        &snapshot.as_of,
        &snapshot.description,
        &snapshot.account_id,
    )?;
    insert_postings(
        transaction,
        &snapshot.entry_id,
        &snapshot.account_id,
        snapshot.yield_cents,
    )?;
    post_entry(transaction, &snapshot.entry_id)
}

fn insert_reversal(
    transaction: &rusqlite::Transaction<'_>,
    yield_id: &str,
    reversal: NewSavingsReversal,
) -> Result<(), AppError> {
    let (account_id, yield_cents): (String, i64) = transaction
        .query_row(
            "SELECT account_id, yield_cents FROM savings_yields
             WHERE id = ?1 AND status = 'confirmed'",
            [yield_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(map_sqlite_error)?
        .ok_or_else(|| AppError::conflict("rendimento não encontrado ou já estornado"))?;

    // A FK deferida permite apontar para o entry que será gravado a seguir;
    // o trigger de tipo de entry já reconhece o estorno.
    let changed = transaction
        .execute(
            "UPDATE savings_yields
             SET status = 'reversed', reversal_entry_id = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?2 AND status = 'confirmed'",
            params![reversal.entry_id, yield_id],
        )
        .map_err(map_sqlite_error)?;
    if changed == 0 {
        return Err(AppError::conflict("rendimento já estornado"));
    }

    insert_entry(
        transaction,
        &reversal.entry_id,
        &reversal.as_of,
        &reversal.description,
        &account_id,
    )?;
    insert_postings(transaction, &reversal.entry_id, &account_id, -yield_cents)?;
    post_entry(transaction, &reversal.entry_id)
}

fn insert_entry(
    transaction: &rusqlite::Transaction<'_>,
    id: &str,
    occurred_on: &str,
    description: &str,
    account_id: &str,
) -> Result<(), AppError> {
    transaction
        .execute(
            "INSERT INTO journal_entries (
                id, kind, occurred_on, description, primary_account_id,
                posted_at, created_at, updated_at
             ) VALUES (
                ?1, 'adjustment', ?2, ?3, ?4, NULL,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![id, occurred_on, description, account_id],
        )
        .map_err(map_sqlite_error)?;
    Ok(())
}

fn insert_postings(
    transaction: &rusqlite::Transaction<'_>,
    entry_id: &str,
    account_id: &str,
    savings_cents: i64,
) -> Result<(), AppError> {
    let internal_cents = savings_cents
        .checked_neg()
        .ok_or_else(|| AppError::validation("valor de rendimento fora do limite"))?;
    for (account, cents) in [
        (account_id, savings_cents),
        (INCOME_ACCOUNT_ID, internal_cents),
    ] {
        transaction
            .execute(
                "INSERT INTO postings (id, entry_id, account_id, amount_cents, created_at)
                 VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                params![Uuid::now_v7().to_string(), entry_id, account, cents],
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
        return Err(AppError::conflict("não foi possível publicar o rendimento"));
    }
    Ok(())
}

fn fetch_yield(
    connection: &rusqlite::Connection,
    id: &str,
) -> rusqlite::Result<Option<SavingsYieldDto>> {
    let sql = format!("{SELECT_YIELD} WHERE sy.id = ?1");
    connection.query_row(&sql, [id], map_yield_row).optional()
}

fn require_yield(connection: &rusqlite::Connection, id: &str) -> Result<SavingsYieldDto, AppError> {
    fetch_yield(connection, id)
        .map_err(map_sqlite_error)?
        .ok_or_else(|| AppError::not_found("rendimento não encontrado"))
}

fn map_yield_row(row: &Row<'_>) -> rusqlite::Result<SavingsYieldDto> {
    Ok(SavingsYieldDto {
        id: row.get(0)?,
        account_id: row.get(1)?,
        as_of: row.get(2)?,
        mode: row.get(3)?,
        cdi_annual_bps: row.get(4)?,
        contracted_percent_bps: row.get(5)?,
        daily_rate: row.get(6)?,
        days: row.get(7)?,
        base_cents: row.get(8)?,
        yield_cents: row.get(9)?,
        status: row.get(10)?,
        entry_id: row.get(11)?,
        reversal_entry_id: row.get(12)?,
        created_at: row.get(13)?,
        rate_date: row.get(14)?,
        rate_series: row.get(15)?,
        cdi_daily_rate: row.get(16)?,
        rate_fetched_at: row.get(17)?,
    })
}

fn map_account_row(row: &Row<'_>) -> rusqlite::Result<AccountDto> {
    Ok(AccountDto {
        id: row.get(0)?,
        name: row.get(1)?,
        account_type: row.get(2)?,
        credit_limit_cents: row.get(3)?,
        is_savings: row.get::<_, i64>(4)? == 1,
        yield_mode: row.get(5)?,
        cdi_percent_bps: row.get(6)?,
        archived_at: row.get(7)?,
        created_at: row.get(8)?,
        balance_cents: row.get(9)?,
    })
}
