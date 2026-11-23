use async_trait::async_trait;
use chrono::{Datelike, Months, NaiveDate};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use tally_app::cards::{
    CardsRepository, EXPENSE_ACCOUNT_ID, NewCardPayment, NewCardPurchase, NewInstallment,
    PlanUpdate, RECOGNITION_ACCOUNT_ID,
};
use tally_app::dto::{
    CardPaymentDto, CardSummaryDto, InstallmentAllocationDto, InstallmentDto, InstallmentPlanDto,
};
use tally_app::error::AppError;
use tally_domain::installment::build_schedule;
use tally_domain::ledger::{Posting, validate_postings};
use tally_domain::money::Money;
use uuid::Uuid;

use crate::db::{Database, map_sqlite_error};

const SELECT_PLAN: &str = "
    SELECT pl.id, pl.card_account_id, ca.name, pl.category_id, c.name, pl.description,
           pl.total_cents, pl.installment_count, pl.first_due_on, pl.period, pl.payment_method,
           e.occurred_on, pl.created_at
    FROM installment_plans pl
    JOIN accounts ca ON ca.id = pl.card_account_id
    JOIN categories c ON c.id = pl.category_id
    JOIN journal_entries e ON e.id = pl.purchase_entry_id
";

#[derive(Clone)]
pub struct SqliteCardsRepository {
    db: Database,
}

impl SqliteCardsRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl CardsRepository for SqliteCardsRepository {
    async fn list_cards(&self) -> Result<Vec<CardSummaryDto>, AppError> {
        self.db
            .call(|connection| {
                let ids = {
                    let mut statement = connection.prepare(
                        "SELECT id FROM accounts
                         WHERE is_system = 0 AND type = 'credit_card'
                         ORDER BY name COLLATE NOCASE",
                    )?;
                    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
                    rows.collect::<rusqlite::Result<Vec<_>>>()?
                };
                let mut cards = Vec::with_capacity(ids.len());
                for id in ids {
                    if let Some(summary) = fetch_card_summary(connection, &id)? {
                        cards.push(summary);
                    }
                }
                Ok(cards)
            })
            .await
    }

    async fn card_summary(&self, card_account_id: &str) -> Result<CardSummaryDto, AppError> {
        let card_account_id = card_account_id.to_owned();
        self.db
            .call(move |connection| fetch_card_summary(connection, &card_account_id))
            .await?
            .ok_or_else(|| AppError::not_found("cartão não encontrado"))
    }

    async fn create_purchase(
        &self,
        purchase: NewCardPurchase,
    ) -> Result<InstallmentPlanDto, AppError> {
        self.db
            .transaction(move |transaction| {
                write_purchase(transaction, &purchase)?;
                fetch_plan(transaction, &purchase.plan_id)
                    .map_err(map_sqlite_error)?
                    .ok_or_else(|| AppError::not_found("parcelamento não encontrado"))
            })
            .await
    }

    async fn list_plans(
        &self,
        card_account_id: Option<&str>,
    ) -> Result<Vec<InstallmentPlanDto>, AppError> {
        let card_account_id = card_account_id.map(str::to_owned);
        self.db
            .call(move |connection| {
                let ids = {
                    let mut statement = connection.prepare(
                        "SELECT pl.id
                         FROM installment_plans pl
                         JOIN journal_entries e ON e.id = pl.purchase_entry_id
                         WHERE (?1 IS NULL OR pl.card_account_id = ?1)
                         ORDER BY e.occurred_on DESC, pl.created_at DESC, pl.id DESC",
                    )?;
                    let rows = statement.query_map(params![card_account_id.as_deref()], |row| {
                        row.get::<_, String>(0)
                    })?;
                    rows.collect::<rusqlite::Result<Vec<_>>>()?
                };
                let mut plans = Vec::with_capacity(ids.len());
                for id in ids {
                    if let Some(plan) = fetch_plan(connection, &id)? {
                        plans.push(plan);
                    }
                }
                Ok(plans)
            })
            .await
    }

    async fn get_plan(&self, plan_id: &str) -> Result<InstallmentPlanDto, AppError> {
        let plan_id = plan_id.to_owned();
        self.db
            .call(move |connection| fetch_plan(connection, &plan_id))
            .await?
            .ok_or_else(|| AppError::not_found("parcelamento não encontrado"))
    }

    async fn delete_plan(&self, plan_id: &str) -> Result<(), AppError> {
        let plan_id = plan_id.to_owned();
        self.db
            .transaction(move |transaction| remove_plan(transaction, &plan_id))
            .await
    }
    async fn update_plan(&self, update: PlanUpdate) -> Result<InstallmentPlanDto, AppError> {
        self.db
            .transaction(move |transaction| replace_plan(transaction, &update))
            .await
    }
    async fn create_payment(&self, payment: NewCardPayment) -> Result<CardPaymentDto, AppError> {
        self.db
            .transaction(move |transaction| {
                write_payment(transaction, &payment)?;
                fetch_payment(transaction, &payment.id)
                    .map_err(map_sqlite_error)?
                    .ok_or_else(|| AppError::not_found("pagamento não encontrado"))
            })
            .await
    }

    async fn list_payments(
        &self,
        card_account_id: Option<&str>,
    ) -> Result<Vec<CardPaymentDto>, AppError> {
        let card_account_id = card_account_id.map(str::to_owned);
        self.db
            .call(move |connection| {
                let ids = {
                    let mut statement = connection.prepare(
                        "SELECT p.id
                         FROM card_payments p
                         JOIN journal_entries e ON e.id = p.entry_id
                         WHERE (?1 IS NULL OR p.card_account_id = ?1)
                         ORDER BY e.occurred_on DESC, p.created_at DESC, p.id DESC",
                    )?;
                    let rows = statement.query_map(params![card_account_id.as_deref()], |row| {
                        row.get::<_, String>(0)
                    })?;
                    rows.collect::<rusqlite::Result<Vec<_>>>()?
                };
                let mut payments = Vec::with_capacity(ids.len());
                for id in ids {
                    if let Some(payment) = fetch_payment(connection, &id)? {
                        payments.push(payment);
                    }
                }
                Ok(payments)
            })
            .await
    }

    async fn delete_payment(&self, payment_id: &str) -> Result<(), AppError> {
        let payment_id = payment_id.to_owned();
        self.db
            .transaction(move |transaction| delete_payment_rows(transaction, &payment_id))
            .await
    }
}

fn write_purchase(connection: &Connection, purchase: &NewCardPurchase) -> Result<(), AppError> {
    ensure_card(connection, &purchase.card_account_id)?;
    ensure_expense_category(connection, &purchase.category_id)?;

    insert_entry(
        connection,
        &purchase.purchase_entry_id,
        "card_purchase",
        &purchase.purchased_on,
        None,
        None,
        purchase.description.as_deref(),
        &purchase.card_account_id,
        None,
        None,
        Some(purchase.payment_method.as_str()),
    )?;
    insert_postings(
        connection,
        &purchase.purchase_entry_id,
        &purchase.purchase_postings,
    )?;
    post_entry(connection, &purchase.purchase_entry_id)?;

    connection
        .execute(
            "INSERT INTO installment_plans (
                id, card_account_id, purchase_entry_id, category_id, description,
                total_cents, installment_count, first_due_on, period, payment_method,
                created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![
                purchase.plan_id,
                purchase.card_account_id,
                purchase.purchase_entry_id,
                purchase.category_id,
                purchase.description,
                purchase.total_cents,
                purchase.installment_count,
                purchase.first_due_on,
                purchase.period,
                purchase.payment_method.as_str(),
            ],
        )
        .map_err(map_sqlite_error)?;

    for installment in &purchase.installments {
        write_installment(connection, purchase, installment)?;
    }

    Ok(())
}

fn write_installment(
    connection: &Connection,
    purchase: &NewCardPurchase,
    installment: &NewInstallment,
) -> Result<(), AppError> {
    // Cada parcela reconhece a despesa na própria data de vencimento.
    insert_entry(
        connection,
        &installment.recognition_entry.id,
        "expense",
        &installment.due_on,
        None,
        Some(&purchase.period),
        purchase.description.as_deref(),
        EXPENSE_ACCOUNT_ID,
        None,
        Some(&purchase.category_id),
        Some(purchase.payment_method.as_str()),
    )?;
    insert_postings(
        connection,
        &installment.recognition_entry.id,
        &installment.recognition_entry.postings,
    )?;
    post_entry(connection, &installment.recognition_entry.id)?;

    connection
        .execute(
            "INSERT INTO installments (
                id, plan_id, seq, due_on, amount_cents, recognition_entry_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                installment.id,
                purchase.plan_id,
                installment.seq,
                installment.due_on,
                installment.amount_cents,
                installment.recognition_entry.id,
            ],
        )
        .map_err(map_sqlite_error)?;

    Ok(())
}

/// Remove o plano e seus lançamentos. Um plano já pago não pode ser excluído:
/// as alocações ficariam órfãs e a dívida do cartão mudaria sem histórico.
fn remove_plan(transaction: &Transaction<'_>, plan_id: &str) -> Result<(), AppError> {
    let purchase_entry_id: String = transaction
        .query_row(
            "SELECT purchase_entry_id FROM installment_plans WHERE id = ?1",
            [plan_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(map_sqlite_error)?
        .ok_or_else(|| AppError::not_found("parcelamento não encontrado"))?;

    let payment_count: i64 = transaction
        .query_row(
            "SELECT COUNT(*)
             FROM installment_allocations a
             JOIN installments i ON i.id = a.installment_id
             WHERE i.plan_id = ?1",
            [plan_id],
            |row| row.get(0),
        )
        .map_err(map_sqlite_error)?;

    if payment_count > 0 {
        return Err(AppError::conflict(
            "o parcelamento possui pagamentos; remova-os primeiro",
        ));
    }

    let recognition_ids = recognition_entry_ids(transaction, plan_id)?;
    for entry_id in &recognition_ids {
        unpost_entry(transaction, entry_id)?;
    }

    transaction
        .execute("DELETE FROM installments WHERE plan_id = ?1", [plan_id])
        .map_err(map_sqlite_error)?;
    transaction
        .execute("DELETE FROM installment_plans WHERE id = ?1", [plan_id])
        .map_err(map_sqlite_error)?;

    unpost_entry(transaction, &purchase_entry_id)?;
    transaction
        .execute(
            "DELETE FROM journal_entries WHERE id = ?1",
            [&purchase_entry_id],
        )
        .map_err(map_sqlite_error)?;

    Ok(())
}

fn recognition_entry_ids(connection: &Connection, plan_id: &str) -> Result<Vec<String>, AppError> {
    let mut statement = connection
        .prepare("SELECT recognition_entry_id FROM installments WHERE plan_id = ?1")
        .map_err(map_sqlite_error)?;
    let rows = statement
        .query_map([plan_id], |row| row.get::<_, String>(0))
        .map_err(map_sqlite_error)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(map_sqlite_error)
}

/// Atualiza um parcelamento. Com pagamentos alocados, apenas a descrição muda;
/// sem pagamentos, o plano é reconstruído (unpost → alterar → repost) para
/// preservar o razão e recalcular as parcelas.
fn replace_plan(
    transaction: &Transaction<'_>,
    update: &PlanUpdate,
) -> Result<InstallmentPlanDto, AppError> {
    let current = fetch_plan(transaction, &update.plan_id)
        .map_err(map_sqlite_error)?
        .ok_or_else(|| AppError::not_found("parcelamento não encontrado"))?;

    let purchase_entry_id: String = transaction
        .query_row(
            "SELECT purchase_entry_id FROM installment_plans WHERE id = ?1",
            [&update.plan_id],
            |row| row.get(0),
        )
        .map_err(map_sqlite_error)?;

    let payment_count: i64 = transaction
        .query_row(
            "SELECT COUNT(*)
             FROM installment_allocations a
             JOIN installments i ON i.id = a.installment_id
             WHERE i.plan_id = ?1",
            [&update.plan_id],
            |row| row.get(0),
        )
        .map_err(map_sqlite_error)?;

    if payment_count > 0 {
        if current.total_cents != update.total_cents
            || current.installment_count != update.installment_count
            || current.first_due_on != update.first_due_on
            || current.category_id != update.category_id
            || current.period != update.period
        {
            return Err(AppError::conflict(
                "o parcelamento possui pagamentos; apenas a descrição pode ser alterada",
            ));
        }

        transaction
            .execute(
                "UPDATE installment_plans
                 SET description = ?1,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                 WHERE id = ?2",
                params![update.description, update.plan_id],
            )
            .map_err(map_sqlite_error)?;

        let mut entry_ids = recognition_entry_ids(transaction, &update.plan_id)?;
        entry_ids.push(purchase_entry_id);
        for entry_id in &entry_ids {
            unpost_entry(transaction, entry_id)?;
        }
        for entry_id in &entry_ids {
            transaction
                .execute(
                    "UPDATE journal_entries
                     SET description = ?1,
                         updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = ?2",
                    params![update.description, entry_id],
                )
                .map_err(map_sqlite_error)?;
        }
        for entry_id in &entry_ids {
            post_entry(transaction, entry_id)?;
        }

        return fetch_plan(transaction, &update.plan_id)
            .map_err(map_sqlite_error)?
            .ok_or_else(|| AppError::not_found("parcelamento não encontrado"));
    }

    ensure_card(transaction, &current.card_account_id)?;
    ensure_expense_category(transaction, &update.category_id)?;

    let recognition_ids = recognition_entry_ids(transaction, &update.plan_id)?;
    transaction
        .execute(
            "DELETE FROM installments WHERE plan_id = ?1",
            [&update.plan_id],
        )
        .map_err(map_sqlite_error)?;
    for entry_id in &recognition_ids {
        unpost_entry(transaction, entry_id)?;
        transaction
            .execute("DELETE FROM journal_entries WHERE id = ?1", [entry_id])
            .map_err(map_sqlite_error)?;
    }

    unpost_entry(transaction, &purchase_entry_id)?;
    transaction
        .execute(
            "UPDATE journal_entries
             SET description = ?1,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?2",
            params![update.description, purchase_entry_id],
        )
        .map_err(map_sqlite_error)?;
    transaction
        .execute(
            "DELETE FROM postings WHERE entry_id = ?1",
            [&purchase_entry_id],
        )
        .map_err(map_sqlite_error)?;

    let purchase_postings = [
        Posting::new(
            Uuid::now_v7().to_string(),
            RECOGNITION_ACCOUNT_ID,
            Money::from_cents(update.total_cents),
        ),
        Posting::new(
            Uuid::now_v7().to_string(),
            current.card_account_id.clone(),
            Money::from_cents(-update.total_cents),
        ),
    ];
    validate_postings(&purchase_postings).map_err(AppError::validation)?;
    insert_postings(transaction, &purchase_entry_id, &purchase_postings)?;
    post_entry(transaction, &purchase_entry_id)?;

    let first_due = NaiveDate::parse_from_str(&update.first_due_on, "%Y-%m-%d")
        .map_err(|_| AppError::validation("informe uma data de vencimento válida"))?;
    let schedule = build_schedule(
        update.total_cents,
        update.installment_count as u32,
        first_due,
    )
    .map_err(AppError::validation)?;

    for (index, (amount, due_on)) in schedule
        .amounts
        .iter()
        .zip(schedule.due_dates.iter())
        .enumerate()
    {
        let amount_cents = amount.cents();
        let entry_id = Uuid::now_v7().to_string();
        let postings = [
            Posting::new(
                Uuid::now_v7().to_string(),
                EXPENSE_ACCOUNT_ID,
                Money::from_cents(amount_cents),
            ),
            Posting::new(
                Uuid::now_v7().to_string(),
                RECOGNITION_ACCOUNT_ID,
                Money::from_cents(-amount_cents),
            ),
        ];
        validate_postings(&postings).map_err(AppError::validation)?;

        insert_entry(
            transaction,
            &entry_id,
            "expense",
            &due_on.format("%Y-%m-%d").to_string(),
            None,
            Some(&update.period),
            update.description.as_deref(),
            EXPENSE_ACCOUNT_ID,
            None,
            Some(&update.category_id),
            Some(&current.payment_method),
        )?;
        insert_postings(transaction, &entry_id, &postings)?;
        post_entry(transaction, &entry_id)?;

        transaction
            .execute(
                "INSERT INTO installments (
                    id, plan_id, seq, due_on, amount_cents, recognition_entry_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    Uuid::now_v7().to_string(),
                    update.plan_id,
                    index as i64 + 1,
                    due_on.format("%Y-%m-%d").to_string(),
                    amount_cents,
                    entry_id,
                ],
            )
            .map_err(map_sqlite_error)?;
    }

    transaction
        .execute(
            "UPDATE installment_plans
             SET category_id = ?1,
                 description = ?2,
                 total_cents = ?3,
                 installment_count = ?4,
                 first_due_on = ?5,
                 period = ?6,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?7",
            params![
                update.category_id,
                update.description,
                update.total_cents,
                update.installment_count,
                update.first_due_on,
                update.period,
                update.plan_id,
            ],
        )
        .map_err(map_sqlite_error)?;

    fetch_plan(transaction, &update.plan_id)
        .map_err(map_sqlite_error)?
        .ok_or_else(|| AppError::not_found("parcelamento não encontrado"))
}

fn unpost_entry(connection: &Connection, entry_id: &str) -> Result<(), AppError> {
    connection
        .execute(
            "UPDATE journal_entries SET posted_at = NULL WHERE id = ?1",
            [entry_id],
        )
        .map_err(map_sqlite_error)?;
    Ok(())
}

fn write_payment(connection: &Connection, payment: &NewCardPayment) -> Result<(), AppError> {
    ensure_card(connection, &payment.card_account_id)?;
    ensure_bank(connection, &payment.bank_account_id)?;

    insert_entry(
        connection,
        &payment.entry_id,
        "card_payment",
        &payment.occurred_on,
        payment.occurred_time.as_deref(),
        payment.manual_period.as_deref(),
        payment.description.as_deref(),
        &payment.bank_account_id,
        Some(&payment.card_account_id),
        None,
        None,
    )?;
    insert_postings(connection, &payment.entry_id, &payment.postings)?;
    post_entry(connection, &payment.entry_id)?;

    connection
        .execute(
            "INSERT INTO card_payments (
                id, card_account_id, bank_account_id, entry_id, amount_cents,
                initial_balance_cents, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, 0,
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             )",
            params![
                payment.id,
                payment.card_account_id,
                payment.bank_account_id,
                payment.entry_id,
                payment.amount_cents,
            ],
        )
        .map_err(map_sqlite_error)?;

    allocate_payment(
        connection,
        &payment.id,
        &payment.card_account_id,
        payment.amount_cents,
    )
}

/// Aplica o pagamento em FIFO: dívida inicial e depois parcelas por vencimento.
/// O que sobrar é permitido e fica como crédito no cartão, sem virar despesa.
fn allocate_payment(
    connection: &Connection,
    payment_id: &str,
    card_account_id: &str,
    amount_cents: i64,
) -> Result<(), AppError> {
    let initial_total: i64 = connection
        .query_row(
            "SELECT COALESCE(-SUM(p.amount_cents), 0)
             FROM postings p
             JOIN journal_entries e ON e.id = p.entry_id
             WHERE e.kind = 'opening_balance' AND p.account_id = ?1",
            [card_account_id],
            |row| row.get(0),
        )
        .map_err(map_sqlite_error)?;
    let initial_applied: i64 = connection
        .query_row(
            "SELECT COALESCE(SUM(initial_balance_cents), 0)
             FROM card_payments WHERE card_account_id = ?1 AND id <> ?2",
            params![card_account_id, payment_id],
            |row| row.get(0),
        )
        .map_err(map_sqlite_error)?;
    let initial_outstanding = (initial_total - initial_applied).max(0);

    let mut remaining = amount_cents;
    let initial_allocated = remaining.min(initial_outstanding);
    remaining -= initial_allocated;

    connection
        .execute(
            "UPDATE card_payments SET initial_balance_cents = ?1 WHERE id = ?2",
            params![initial_allocated, payment_id],
        )
        .map_err(map_sqlite_error)?;

    if remaining == 0 {
        return Ok(());
    }

    let outstanding = fetch_outstanding_installments(connection, card_account_id)?;

    for (installment_id, installment_outstanding) in outstanding {
        if remaining == 0 {
            break;
        }

        let allocated = remaining.min(installment_outstanding);
        if allocated <= 0 {
            continue;
        }

        connection
            .execute(
                "INSERT INTO installment_allocations (payment_id, installment_id, amount_cents)
                 VALUES (?1, ?2, ?3)",
                params![payment_id, installment_id, allocated],
            )
            .map_err(map_sqlite_error)?;
        remaining -= allocated;
    }

    Ok(())
}

fn fetch_outstanding_installments(
    connection: &Connection,
    card_account_id: &str,
) -> Result<Vec<(String, i64)>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT i.id, i.amount_cents - COALESCE(SUM(a.amount_cents), 0) AS outstanding
             FROM installments i
             JOIN installment_plans pl ON pl.id = i.plan_id
             LEFT JOIN installment_allocations a ON a.installment_id = i.id
             WHERE pl.card_account_id = ?1
             GROUP BY i.id
             HAVING outstanding > 0
             ORDER BY i.due_on, i.seq, i.id",
        )
        .map_err(map_sqlite_error)?;
    let rows = statement
        .query_map([card_account_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(map_sqlite_error)?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(map_sqlite_error)
}

fn delete_payment_rows(transaction: &Transaction<'_>, payment_id: &str) -> Result<(), AppError> {
    let entry_id: String = transaction
        .query_row(
            "SELECT entry_id FROM card_payments WHERE id = ?1",
            [payment_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(map_sqlite_error)?
        .ok_or_else(|| AppError::not_found("pagamento não encontrado"))?;

    unpost_entry(transaction, &entry_id)?;
    // As alocações caem por ON DELETE CASCADE, reabrindo as parcelas.
    transaction
        .execute("DELETE FROM card_payments WHERE id = ?1", [payment_id])
        .map_err(map_sqlite_error)?;
    transaction
        .execute("DELETE FROM journal_entries WHERE id = ?1", [&entry_id])
        .map_err(map_sqlite_error)?;

    Ok(())
}

type AccountRow = (String, Option<String>, i64, Option<String>, i64);

fn fetch_account(connection: &Connection, id: &str) -> Result<AccountRow, AppError> {
    connection
        .query_row(
            "SELECT class, type, is_system, archived_at, is_savings FROM accounts WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .optional()
        .map_err(map_sqlite_error)?
        .ok_or_else(|| AppError::not_found("conta não encontrada"))
}

fn ensure_card(connection: &Connection, id: &str) -> Result<(), AppError> {
    let (class, account_type, is_system, archived_at, _) = fetch_account(connection, id)?;

    if is_system == 1 {
        return Err(AppError::validation(
            "contas internas não podem ser usadas diretamente",
        ));
    }
    if class != "liability" || account_type.as_deref() != Some("credit_card") {
        return Err(AppError::validation("a conta informada não é um cartão"));
    }
    if archived_at.is_some() {
        return Err(AppError::validation("o cartão está arquivado"));
    }

    Ok(())
}

fn ensure_bank(connection: &Connection, id: &str) -> Result<(), AppError> {
    let (class, account_type, is_system, archived_at, is_savings) = fetch_account(connection, id)?;

    if is_system == 1 {
        return Err(AppError::validation(
            "contas internas não podem ser usadas diretamente",
        ));
    }
    if is_savings == 1 {
        return Err(AppError::validation(
            "cofrinhos não podem pagar faturas de cartão",
        ));
    }
    if class != "asset" || account_type.is_none() {
        return Err(AppError::validation(
            "a conta de pagamento deve ser uma conta de ativo",
        ));
    }
    if archived_at.is_some() {
        return Err(AppError::validation("a conta de origem está arquivada"));
    }

    Ok(())
}

fn ensure_expense_category(connection: &Connection, id: &str) -> Result<(), AppError> {
    let (kind, archived_at): (String, Option<String>) = connection
        .query_row(
            "SELECT kind, archived_at FROM categories WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(map_sqlite_error)?
        .ok_or_else(|| AppError::not_found("categoria não encontrada"))?;

    if kind != "expense" {
        return Err(AppError::validation(
            "a categoria da compra deve ser de despesa",
        ));
    }
    if archived_at.is_some() {
        return Err(AppError::validation("a categoria está arquivada"));
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_entry(
    connection: &Connection,
    id: &str,
    kind: &str,
    occurred_on: &str,
    occurred_time: Option<&str>,
    manual_period: Option<&str>,
    description: Option<&str>,
    primary_account_id: &str,
    counterparty_account_id: Option<&str>,
    category_id: Option<&str>,
    payment_method: Option<&str>,
) -> Result<(), AppError> {
    connection
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
                id,
                kind,
                occurred_on,
                occurred_time,
                manual_period,
                description,
                primary_account_id,
                counterparty_account_id,
                category_id,
                payment_method,
            ],
        )
        .map_err(map_sqlite_error)?;

    Ok(())
}

fn insert_postings(
    connection: &Connection,
    entry_id: &str,
    postings: &[tally_domain::ledger::Posting],
) -> Result<(), AppError> {
    for posting in postings {
        connection
            .execute(
                "INSERT INTO postings (id, entry_id, account_id, amount_cents, created_at)
                 VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                params![
                    posting.id,
                    entry_id,
                    posting.account_id,
                    posting.amount.cents()
                ],
            )
            .map_err(map_sqlite_error)?;
    }

    Ok(())
}

fn post_entry(connection: &Connection, id: &str) -> Result<(), AppError> {
    let changed = connection
        .execute(
            "UPDATE journal_entries
             SET posted_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE id = ?1 AND posted_at IS NULL",
            [id],
        )
        .map_err(map_sqlite_error)?;

    if changed == 0 {
        return Err(AppError::conflict("não foi possível publicar o lançamento"));
    }

    Ok(())
}

fn fetch_plan(
    connection: &Connection,
    plan_id: &str,
) -> rusqlite::Result<Option<InstallmentPlanDto>> {
    let mut statement = connection.prepare(&format!("{SELECT_PLAN} WHERE pl.id = ?1"))?;
    let plan = statement
        .query_row([plan_id], |row| {
            Ok(InstallmentPlanDto {
                id: row.get(0)?,
                card_account_id: row.get(1)?,
                card_account_name: row.get(2)?,
                category_id: row.get(3)?,
                category_name: row.get(4)?,
                description: row.get(5)?,
                total_cents: row.get(6)?,
                installment_count: row.get(7)?,
                first_due_on: row.get(8)?,
                period: row.get(9)?,
                payment_method: row.get(10)?,
                purchased_on: row.get(11)?,
                created_at: row.get(12)?,
                paid_cents: 0,
                outstanding_cents: 0,
                installments: Vec::new(),
            })
        })
        .optional()?;

    let Some(plan) = plan else {
        return Ok(None);
    };

    let installments = fetch_installments(connection, plan_id)?;
    let paid_cents = installments.iter().map(|item| item.paid_cents).sum();
    let outstanding_cents = installments
        .iter()
        .map(|item| (item.amount_cents - item.paid_cents).max(0))
        .sum();

    Ok(Some(InstallmentPlanDto {
        installments,
        paid_cents,
        outstanding_cents,
        ..plan
    }))
}

fn fetch_installments(
    connection: &Connection,
    plan_id: &str,
) -> rusqlite::Result<Vec<InstallmentDto>> {
    let mut statement = connection.prepare(
        "SELECT i.id, i.plan_id, i.seq, i.due_on, i.amount_cents,
                COALESCE(SUM(a.amount_cents), 0) AS paid_cents
         FROM installments i
         LEFT JOIN installment_allocations a ON a.installment_id = i.id
         WHERE i.plan_id = ?1
         GROUP BY i.id
         ORDER BY i.seq",
    )?;
    let rows = statement.query_map([plan_id], |row| {
        Ok(InstallmentDto {
            id: row.get(0)?,
            plan_id: row.get(1)?,
            seq: row.get(2)?,
            due_on: row.get(3)?,
            amount_cents: row.get(4)?,
            paid_cents: row.get(5)?,
            status: String::new(),
        })
    })?;

    let today = chrono::Local::now()
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let mut installments = Vec::new();

    for row in rows {
        let mut installment = row?;
        installment.status = if installment.paid_cents >= installment.amount_cents {
            "paid"
        } else if installment.due_on.as_str() < today.as_str() {
            "overdue"
        } else if installment.paid_cents > 0 {
            "partial"
        } else {
            "open"
        }
        .to_owned();
        installments.push(installment);
    }

    Ok(installments)
}

fn fetch_card_summary(
    connection: &Connection,
    card_id: &str,
) -> rusqlite::Result<Option<CardSummaryDto>> {
    let account = connection
        .query_row(
            "SELECT name, credit_limit_cents, archived_at
             FROM accounts
             WHERE id = ?1 AND is_system = 0 AND type = 'credit_card'",
            [card_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .optional()?;

    let Some((name, credit_limit_cents, archived_at)) = account else {
        return Ok(None);
    };

    let balance_cents: i64 = connection.query_row(
        "SELECT COALESCE(SUM(p.amount_cents), 0)
         FROM postings p
         JOIN journal_entries e ON e.id = p.entry_id
         WHERE p.account_id = ?1 AND e.posted_at IS NOT NULL",
        [card_id],
        |row| row.get(0),
    )?;
    let debt_cents = (-balance_cents).max(0);
    let credit_cents = balance_cents.max(0);

    let initial_total: i64 = connection.query_row(
        "SELECT COALESCE(-SUM(p.amount_cents), 0)
         FROM postings p
         JOIN journal_entries e ON e.id = p.entry_id
         WHERE e.kind = 'opening_balance' AND p.account_id = ?1",
        [card_id],
        |row| row.get(0),
    )?;
    let initial_paid: i64 = connection.query_row(
        "SELECT COALESCE(SUM(initial_balance_cents), 0)
         FROM card_payments WHERE card_account_id = ?1",
        [card_id],
        |row| row.get(0),
    )?;
    let initial_debt_cents = (initial_total - initial_paid).max(0);

    let today = chrono::Local::now().date_naive();
    let month_end = today
        .with_day(1)
        .and_then(|date| date.checked_add_months(Months::new(1)))
        .and_then(|date| date.pred_opt())
        .expect("o fim do mês é sempre válido")
        .format("%Y-%m-%d")
        .to_string();
    let today = today.format("%Y-%m-%d").to_string();

    let mut overdue_cents = 0;
    let mut current_month_cents = 0;
    let mut future_cents = 0;

    let outstanding = {
        let mut statement = connection.prepare(
            "SELECT i.due_on, i.amount_cents - COALESCE(SUM(a.amount_cents), 0) AS outstanding
             FROM installments i
             JOIN installment_plans pl ON pl.id = i.plan_id
             LEFT JOIN installment_allocations a ON a.installment_id = i.id
             WHERE pl.card_account_id = ?1
             GROUP BY i.id
             HAVING outstanding > 0",
        )?;
        let rows = statement.query_map([card_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    for (due_on, cents) in outstanding {
        if due_on.as_str() < today.as_str() {
            overdue_cents += cents;
        } else if due_on.as_str() <= month_end.as_str() {
            current_month_cents += cents;
        } else {
            future_cents += cents;
        }
    }

    let available_limit_cents = credit_limit_cents.map(|limit| limit - debt_cents);
    let over_limit = credit_limit_cents.is_some_and(|limit| debt_cents > limit);

    Ok(Some(CardSummaryDto {
        id: card_id.to_owned(),
        name,
        archived_at,
        credit_limit_cents,
        debt_cents,
        credit_cents,
        available_limit_cents,
        initial_debt_cents,
        overdue_cents,
        current_month_cents,
        future_cents,
        over_limit,
    }))
}

fn fetch_payment(
    connection: &Connection,
    payment_id: &str,
) -> rusqlite::Result<Option<CardPaymentDto>> {
    let payment = connection
        .query_row(
            "SELECT p.id, p.card_account_id, ca.name, p.bank_account_id, ba.name, p.entry_id,
                    p.amount_cents, p.initial_balance_cents, e.occurred_on, e.occurred_time,
                    e.period, e.description, p.created_at
             FROM card_payments p
             JOIN accounts ca ON ca.id = p.card_account_id
             JOIN accounts ba ON ba.id = p.bank_account_id
             JOIN journal_entries e ON e.id = p.entry_id
             WHERE p.id = ?1",
            [payment_id],
            |row| {
                Ok(CardPaymentDto {
                    id: row.get(0)?,
                    card_account_id: row.get(1)?,
                    card_account_name: row.get(2)?,
                    bank_account_id: row.get(3)?,
                    bank_account_name: row.get(4)?,
                    entry_id: row.get(5)?,
                    amount_cents: row.get(6)?,
                    unallocated_cents: 0,
                    initial_balance_cents: row.get(7)?,
                    occurred_on: row.get(8)?,
                    occurred_time: row.get(9)?,
                    period: row.get(10)?,
                    description: row.get(11)?,
                    created_at: row.get(12)?,
                    allocations: Vec::new(),
                })
            },
        )
        .optional()?;

    let Some(payment) = payment else {
        return Ok(None);
    };

    let allocations = {
        let mut statement = connection.prepare(
            "SELECT a.installment_id, i.plan_id, i.seq, i.due_on, a.amount_cents
             FROM installment_allocations a
             JOIN installments i ON i.id = a.installment_id
             WHERE a.payment_id = ?1
             ORDER BY i.due_on, i.seq",
        )?;
        let rows = statement.query_map([payment_id], |row| {
            Ok(InstallmentAllocationDto {
                installment_id: row.get(0)?,
                plan_id: row.get(1)?,
                seq: row.get(2)?,
                due_on: row.get(3)?,
                amount_cents: row.get(4)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    Ok(Some(CardPaymentDto {
        unallocated_cents: (payment.amount_cents
            - payment.initial_balance_cents
            - allocations
                .iter()
                .map(|allocation| allocation.amount_cents)
                .sum::<i64>())
        .max(0),
        allocations,
        ..payment
    }))
}
