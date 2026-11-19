use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use rusqlite::{Connection, params};
use tally_app::dto::{
    CashFlowGroupDto, CashFlowMonthDto, CashFlowReportDto, CategoryReportDto, CategoryTotalDto,
    CommitmentDto, CommitmentMonthDto, CommitmentsReportDto, ExpenseTrendDto, MonthlyExpenseDto,
    NetWorthAccountDto, NetWorthHistoryDto, NetWorthPointDto, NetWorthReportDto,
    PeriodSpendingBucketDto, PeriodSpendingReportDto,
};
use tally_app::error::AppError;
use tally_app::reports::{ReportQuery, ReportsRepository};

use crate::db::Database;

const PERIOD_BANDS: [&str; 4] = ["madrugada", "manha", "tarde", "noite"];
const UNCLASSIFIED: &str = "unclassified";

#[derive(Clone)]
pub struct SqliteReportsRepository {
    db: Database,
}

impl SqliteReportsRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ReportsRepository for SqliteReportsRepository {
    async fn category_report(
        &self,
        kind: &str,
        query: &ReportQuery,
    ) -> Result<CategoryReportDto, AppError> {
        let kind = kind.to_owned();
        let query = query.clone();

        self.db
            .call(move |connection| {
                let sign = if kind == "expense" { 1_i64 } else { -1_i64 };
                let mut statement = connection.prepare(
                    "SELECT e.category_id, c.name, SUM(p.amount_cents * ?1) AS total,
                            COUNT(DISTINCT e.id) AS entries
                     FROM journal_entries e
                     JOIN postings p ON p.entry_id = e.id
                     JOIN accounts a ON a.id = p.account_id
                                    AND a.is_system = 1 AND a.class = ?2
                     LEFT JOIN categories c ON c.id = e.category_id
                     WHERE e.posted_at IS NOT NULL
                       AND e.kind = ?2
                       AND e.occurred_on BETWEEN ?3 AND ?4
                       AND (?5 IS NULL OR e.period = ?5)
                       AND (?6 IS NULL OR e.category_id = ?6)
                       AND (
                           ?7 IS NULL
                           OR e.primary_account_id = ?7
                           OR EXISTS (
                               SELECT 1
                               FROM installments i
                               JOIN installment_plans pl ON pl.id = i.plan_id
                               WHERE i.recognition_entry_id = e.id
                                 AND pl.card_account_id = ?7
                           )
                       )
                       AND (?8 IS NULL OR e.payment_method = ?8)
                     GROUP BY e.category_id, c.name
                     ORDER BY total DESC, c.name COLLATE NOCASE",
                )?;

                let rows = statement.query_map(
                    params![
                        sign,
                        kind.as_str(),
                        query.from.as_str(),
                        query.to.as_str(),
                        query.period.as_deref(),
                        query.category_id.as_deref(),
                        query.account_id.as_deref(),
                        query.payment_method.as_deref()
                    ],
                    |row| {
                        Ok(CategoryTotalDto {
                            category_id: row.get(0)?,
                            category_name: row
                                .get::<_, Option<String>>(1)?
                                .unwrap_or_else(|| "Sem categoria".to_owned()),
                            total_cents: row.get(2)?,
                            entry_count: row.get(3)?,
                        })
                    },
                )?;

                let categories = rows.collect::<rusqlite::Result<Vec<_>>>()?;
                let total_cents = categories.iter().map(|item| item.total_cents).sum();
                let entry_count = categories.iter().map(|item| item.entry_count).sum();

                Ok(CategoryReportDto {
                    kind,
                    from: query.from,
                    to: query.to,
                    period: query.period,
                    total_cents,
                    entry_count,
                    categories,
                })
            })
            .await
    }

    async fn cash_flow(&self, query: &ReportQuery) -> Result<CashFlowReportDto, AppError> {
        let query = query.clone();

        self.db
            .call(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT e.id, e.kind, e.occurred_on, c.name,
                            SUM(p.amount_cents) AS delta
                     FROM journal_entries e
                     JOIN postings p ON p.entry_id = e.id
                     JOIN accounts a ON a.id = p.account_id
                                    AND a.is_system = 0 AND a.class = 'asset'
                     LEFT JOIN categories c ON c.id = e.category_id
                     WHERE e.posted_at IS NOT NULL
                       AND e.kind IN ('expense', 'income', 'card_payment', 'adjustment')
                       AND e.occurred_on BETWEEN ?1 AND ?2
                       AND (?3 IS NULL OR e.period = ?3)
                       AND (?4 IS NULL OR e.category_id = ?4)
                       AND (?5 IS NULL OR p.account_id = ?5)
                       AND (?6 IS NULL OR e.payment_method = ?6)
                       AND NOT EXISTS (
                           SELECT 1 FROM installments i
                           WHERE i.recognition_entry_id = e.id
                       )
                     GROUP BY e.id
                     ORDER BY e.occurred_on, e.created_at, e.id",
                )?;

                let rows = statement.query_map(
                    params![
                        query.from.as_str(),
                        query.to.as_str(),
                        query.period.as_deref(),
                        query.category_id.as_deref(),
                        query.account_id.as_deref(),
                        query.payment_method.as_deref()
                    ],
                    |row| {
                        Ok((
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, i64>(4)?,
                        ))
                    },
                )?;

                let mut inflow_cents = 0_i64;
                let mut outflow_cents = 0_i64;
                let mut groups: BTreeMap<(String, String), (i64, i64)> = BTreeMap::new();
                let mut months: BTreeMap<String, (i64, i64)> = BTreeMap::new();

                for row in rows {
                    let (kind, occurred_on, category_name, delta) = row?;
                    let label = cash_flow_label(&kind, category_name.as_deref());
                    let (inflow, outflow) = split_delta(delta);
                    inflow_cents += inflow;
                    outflow_cents += outflow;

                    let group = groups.entry((label, kind)).or_default();
                    group.0 += inflow;
                    group.1 += outflow;

                    let month = occurred_on.get(..7).unwrap_or(&occurred_on).to_owned();
                    let month_totals = months.entry(month).or_default();
                    month_totals.0 += inflow;
                    month_totals.1 += outflow;
                }

                let mut groups = groups
                    .into_iter()
                    .map(|((label, kind), (inflow, outflow))| CashFlowGroupDto {
                        label,
                        kind,
                        inflow_cents: inflow,
                        outflow_cents: outflow,
                        net_cents: inflow - outflow,
                    })
                    .collect::<Vec<_>>();
                groups.sort_by(|left, right| {
                    right
                        .net_cents
                        .cmp(&left.net_cents)
                        .then_with(|| left.label.cmp(&right.label))
                });

                let months = months
                    .into_iter()
                    .map(|(month, (inflow, outflow))| CashFlowMonthDto {
                        month,
                        inflow_cents: inflow,
                        outflow_cents: outflow,
                        net_cents: inflow - outflow,
                    })
                    .collect();

                Ok(CashFlowReportDto {
                    from: query.from,
                    to: query.to,
                    period: query.period,
                    inflow_cents,
                    outflow_cents,
                    net_cents: inflow_cents - outflow_cents,
                    groups,
                    months,
                })
            })
            .await
    }

    async fn period_spending(
        &self,
        query: &ReportQuery,
    ) -> Result<PeriodSpendingReportDto, AppError> {
        let query = query.clone();

        self.db
            .call(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT e.period, e.occurred_on, SUM(p.amount_cents) AS total,
                            COUNT(DISTINCT e.id) AS entries
                     FROM journal_entries e
                     JOIN postings p ON p.entry_id = e.id
                     JOIN accounts a ON a.id = p.account_id
                                    AND a.is_system = 1 AND a.class = 'expense'
                     WHERE e.posted_at IS NOT NULL
                       AND e.kind = 'expense'
                       AND e.occurred_on BETWEEN ?1 AND ?2
                     GROUP BY e.period, e.occurred_on",
                )?;

                let rows = statement.query_map(
                    params![query.from.as_str(), query.to.as_str()],
                    |row| {
                        Ok((
                            row.get::<_, Option<String>>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, i64>(3)?,
                        ))
                    },
                )?;

                let mut buckets: BTreeMap<String, BucketAccumulator> = BTreeMap::new();
                let mut days_with_expense = BTreeSet::new();

                for row in rows {
                    let (period, occurred_on, total_cents, entry_count) = row?;
                    let key = match period.as_deref() {
                        Some(value) if PERIOD_BANDS.contains(&value) => value.to_owned(),
                        // Período manual (`custom`) e ausente ficam fora das faixas.
                        _ => UNCLASSIFIED.to_owned(),
                    };

                    let bucket = buckets.entry(key).or_default();
                    bucket.total_cents += total_cents;
                    bucket.entry_count += entry_count;
                    bucket.days.insert(occurred_on.clone());
                    days_with_expense.insert(occurred_on);
                }

                let days_with_expense = days_with_expense.len() as i64;
                let make = |key: &str| {
                    let bucket = buckets.get(key);
                    let total_cents = bucket.map_or(0, |item| item.total_cents);
                    let entry_count = bucket.map_or(0, |item| item.entry_count);
                    let active_days = bucket.map_or(0, |item| item.days.len() as i64);

                    PeriodSpendingBucketDto {
                        period: key.to_owned(),
                        total_cents,
                        entry_count,
                        active_days,
                        average_cents: if active_days > 0 {
                            total_cents / active_days
                        } else {
                            0
                        },
                        frequency_percent: if days_with_expense > 0 {
                            active_days as f64 / days_with_expense as f64 * 100.0
                        } else {
                            0.0
                        },
                    }
                };

                let total_cents = buckets.values().map(|item| item.total_cents).sum();
                let entry_count = buckets.values().map(|item| item.entry_count).sum();
                let bands = PERIOD_BANDS.iter().map(|band| make(band)).collect();
                let unclassified = make(UNCLASSIFIED);

                Ok(PeriodSpendingReportDto {
                    from: query.from,
                    to: query.to,
                    total_cents,
                    entry_count,
                    days_with_expense,
                    buckets: bands,
                    unclassified,
                })
            })
            .await
    }

    async fn expense_trend(
        &self,
        from: &str,
        to: &str,
        months: &[String],
    ) -> Result<ExpenseTrendDto, AppError> {
        let from = from.to_owned();
        let to = to.to_owned();
        let months = months.to_vec();

        self.db
            .call(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT substr(e.occurred_on, 1, 7) AS month,
                            SUM(p.amount_cents) AS total,
                            COUNT(DISTINCT e.id) AS entries
                     FROM journal_entries e
                     JOIN postings p ON p.entry_id = e.id
                     JOIN accounts a ON a.id = p.account_id
                                    AND a.is_system = 1 AND a.class = 'expense'
                     WHERE e.posted_at IS NOT NULL
                       AND e.kind = 'expense'
                       AND e.occurred_on BETWEEN ?1 AND ?2
                     GROUP BY month",
                )?;

                let rows = statement.query_map(params![from.as_str(), to.as_str()], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                })?;

                let mut totals = BTreeMap::new();
                for row in rows {
                    let (month, total_cents, entry_count) = row?;
                    totals.insert(month, (total_cents, entry_count));
                }

                let months = months
                    .into_iter()
                    .map(|month| {
                        let (total_cents, entry_count) =
                            totals.get(&month).copied().unwrap_or((0, 0));
                        MonthlyExpenseDto {
                            month,
                            total_cents,
                            entry_count,
                        }
                    })
                    .collect::<Vec<_>>();

                Ok(ExpenseTrendDto {
                    from_month: months
                        .first()
                        .map_or_else(String::new, |item| item.month.clone()),
                    to_month: months
                        .last()
                        .map_or_else(String::new, |item| item.month.clone()),
                    months,
                })
            })
            .await
    }

    async fn net_worth_history(&self, dates: &[String]) -> Result<NetWorthHistoryDto, AppError> {
        let dates = dates.to_vec();

        self.db
            .call(move |connection| {
                let values = (1..=dates.len())
                    .map(|index| format!("SELECT ?{index} AS as_of"))
                    .collect::<Vec<_>>()
                    .join(" UNION ALL ");
                let sql = format!(
                    "WITH dates AS ({values})
                     SELECT d.as_of, a.class,
                            COALESCE((
                                SELECT SUM(p.amount_cents)
                                FROM postings p
                                JOIN journal_entries e ON e.id = p.entry_id
                                WHERE p.account_id = a.id
                                  AND e.posted_at IS NOT NULL
                                  AND e.occurred_on <= d.as_of
                            ), 0) AS balance_cents
                     FROM dates d
                     CROSS JOIN accounts a
                     WHERE a.class IN ('asset', 'liability')
                     ORDER BY d.as_of, a.class"
                );

                let mut statement = connection.prepare(&sql)?;
                let rows = statement.query_map(
                    rusqlite::params_from_iter(dates.iter().map(String::as_str)),
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                        ))
                    },
                )?;

                let mut totals: BTreeMap<String, (i64, i64)> = BTreeMap::new();
                for row in rows {
                    let (as_of, class, balance_cents) = row?;
                    let entry = totals.entry(as_of).or_default();
                    if class == "asset" {
                        entry.0 += balance_cents;
                    } else {
                        entry.1 += -balance_cents;
                    }
                }

                let points = dates
                    .into_iter()
                    .map(|as_of| {
                        let (assets_cents, liabilities_cents) =
                            totals.get(&as_of).copied().unwrap_or((0, 0));
                        NetWorthPointDto {
                            month: as_of.get(..7).unwrap_or(&as_of).to_owned(),
                            as_of,
                            net_worth_cents: assets_cents - liabilities_cents,
                            assets_cents,
                            liabilities_cents,
                        }
                    })
                    .collect::<Vec<_>>();

                Ok(NetWorthHistoryDto {
                    from_month: points
                        .first()
                        .map_or_else(String::new, |point| point.month.clone()),
                    to_month: points
                        .last()
                        .map_or_else(String::new, |point| point.month.clone()),
                    points,
                })
            })
            .await
    }

    async fn net_worth(&self, as_of: &str) -> Result<NetWorthReportDto, AppError> {
        let as_of = as_of.to_owned();

        self.db
            .call(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT a.id, a.name, a.type, a.is_system, a.class,
                            COALESCE((
                                SELECT SUM(p.amount_cents)
                                FROM postings p
                                JOIN journal_entries e ON e.id = p.entry_id
                                WHERE p.account_id = a.id
                                  AND e.posted_at IS NOT NULL
                                  AND e.occurred_on <= ?1
                            ), 0) AS balance_cents
                     FROM accounts a
                     WHERE a.class IN ('asset', 'liability')
                     ORDER BY a.is_system, a.class, a.name COLLATE NOCASE",
                )?;

                let rows = statement.query_map([&as_of], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                })?;

                let mut assets_cents = 0_i64;
                let mut liabilities_cents = 0_i64;
                let mut deferred_expense_cents = 0_i64;
                let mut accounts = Vec::new();

                for row in rows {
                    let (id, name, account_type, is_system, class, balance_cents) = row?;

                    match class.as_str() {
                        "asset" => {
                            assets_cents += balance_cents;
                            if is_system == 1 {
                                deferred_expense_cents += balance_cents;
                                continue;
                            }
                        }
                        "liability" => liabilities_cents += -balance_cents,
                        _ => continue,
                    }

                    if balance_cents != 0 {
                        accounts.push(NetWorthAccountDto {
                            account_id: id,
                            name,
                            account_type,
                            balance_cents,
                        });
                    }
                }

                Ok(NetWorthReportDto {
                    as_of,
                    assets_cents,
                    liabilities_cents,
                    net_worth_cents: assets_cents - liabilities_cents,
                    deferred_expense_cents,
                    accounts,
                })
            })
            .await
    }

    async fn commitments(
        &self,
        as_of: &str,
        start_after: &str,
        horizon_end: &str,
    ) -> Result<CommitmentsReportDto, AppError> {
        let as_of = as_of.to_owned();
        let start_after = start_after.to_owned();
        let horizon_end = horizon_end.to_owned();

        self.db
            .call(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT i.id, i.plan_id, pl.card_account_id, ca.name, c.name,
                            pl.description, i.seq, pl.installment_count, i.due_on,
                            i.amount_cents, COALESCE(SUM(a.amount_cents), 0) AS paid_cents
                     FROM installments i
                     JOIN installment_plans pl ON pl.id = i.plan_id
                     JOIN accounts ca ON ca.id = pl.card_account_id
                     JOIN categories c ON c.id = pl.category_id
                     LEFT JOIN installment_allocations a ON a.installment_id = i.id
                     WHERE i.due_on > ?1 AND i.due_on <= ?2
                     GROUP BY i.id
                     HAVING i.amount_cents - COALESCE(SUM(a.amount_cents), 0) > 0
                     ORDER BY i.due_on, ca.name COLLATE NOCASE, i.seq",
                )?;

                let rows = statement.query_map(
                    params![start_after.as_str(), horizon_end.as_str()],
                    |row| {
                        let amount_cents: i64 = row.get(9)?;
                        let paid_cents: i64 = row.get(10)?;
                        Ok(CommitmentDto {
                            installment_id: row.get(0)?,
                            plan_id: row.get(1)?,
                            card_account_id: row.get(2)?,
                            card_account_name: row.get(3)?,
                            category_name: row.get(4)?,
                            description: row.get(5)?,
                            seq: row.get(6)?,
                            installment_count: row.get(7)?,
                            due_on: row.get(8)?,
                            amount_cents,
                            paid_cents,
                            outstanding_cents: amount_cents - paid_cents,
                        })
                    },
                )?;

                let mut months: BTreeMap<String, Vec<CommitmentDto>> = BTreeMap::new();
                for row in rows {
                    let commitment = row?;
                    let month = commitment
                        .due_on
                        .get(..7)
                        .unwrap_or(&commitment.due_on)
                        .to_owned();
                    months.entry(month).or_default().push(commitment);
                }

                let months = months
                    .into_iter()
                    .map(|(month, items)| CommitmentMonthDto {
                        month,
                        total_cents: items.iter().map(|item| item.outstanding_cents).sum(),
                        items,
                    })
                    .collect::<Vec<_>>();
                let total_cents = months.iter().map(|month| month.total_cents).sum();
                let overdue_cents = overdue_installments(connection, &as_of)?;

                Ok(CommitmentsReportDto {
                    as_of,
                    start_after,
                    horizon_end,
                    total_cents,
                    overdue_cents,
                    months,
                })
            })
            .await
    }

    async fn overdue_installments(&self, as_of: &str) -> Result<i64, AppError> {
        let as_of = as_of.to_owned();
        self.db
            .call(move |connection| overdue_installments(connection, &as_of))
            .await
    }

    async fn card_totals(&self) -> Result<(i64, i64), AppError> {
        self.db
            .call(|connection| {
                connection.query_row(
                    "SELECT COALESCE(SUM(CASE WHEN balance < 0 THEN -balance ELSE 0 END), 0),
                            COALESCE(SUM(CASE WHEN balance > 0 THEN balance ELSE 0 END), 0)
                     FROM (
                         SELECT COALESCE((
                             SELECT SUM(p.amount_cents)
                             FROM postings p
                             JOIN journal_entries e ON e.id = p.entry_id
                             WHERE p.account_id = a.id AND e.posted_at IS NOT NULL
                         ), 0) AS balance
                         FROM accounts a
                         WHERE a.is_system = 0 AND a.type = 'credit_card'
                     )",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
            })
            .await
    }
}

#[derive(Default)]
struct BucketAccumulator {
    total_cents: i64,
    entry_count: i64,
    days: BTreeSet<String>,
}

fn cash_flow_label(kind: &str, category_name: Option<&str>) -> String {
    match kind {
        "card_payment" => "Pagamento de cartão".to_owned(),
        "adjustment" => "Ajuste".to_owned(),
        _ => category_name.unwrap_or("Sem categoria").to_owned(),
    }
}

fn split_delta(delta: i64) -> (i64, i64) {
    if delta >= 0 { (delta, 0) } else { (0, -delta) }
}

fn overdue_installments(connection: &Connection, as_of: &str) -> rusqlite::Result<i64> {
    connection.query_row(
        "SELECT COALESCE(SUM(outstanding), 0)
         FROM (
             SELECT i.amount_cents - COALESCE(SUM(a.amount_cents), 0) AS outstanding
             FROM installments i
             LEFT JOIN installment_allocations a ON a.installment_id = i.id
             WHERE i.due_on <= ?1
             GROUP BY i.id
             HAVING outstanding > 0
         )",
        [as_of],
        |row| row.get(0),
    )
}
