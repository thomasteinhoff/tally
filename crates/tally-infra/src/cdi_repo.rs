use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, params};
use tally_app::dto::CdiStatusDto;
use tally_app::error::AppError;
use tally_app::ports::{CdiCacheInfo, CdiCacheState, CdiRate, CdiRateSnapshot, CdiRatesRepository};

use crate::db::{Database, map_sqlite_error};

#[derive(Clone)]
pub struct SqliteCdiRepository {
    db: Database,
}

impl SqliteCdiRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

fn timestamp(value: &str) -> Result<String, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|date| date.with_timezone(&Utc).to_rfc3339())
        .map_err(|_| AppError::validation("data/hora inválida"))
}

#[async_trait]
impl CdiRatesRepository for SqliteCdiRepository {
    async fn info(&self) -> Result<CdiCacheInfo, AppError> {
        let info = self
            .db
            .call(|connection| {
                connection.query_row(
                    "SELECT s.last_rate_date, s.last_success_at, s.last_error,
                            (SELECT COUNT(*) FROM cdi_rates)
                     FROM cdi_sync_status s WHERE s.singleton = 1",
                    [],
                    |row| {
                        Ok((
                            row.get::<_, Option<String>>(0)?,
                            row.get::<_, Option<String>>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, i64>(3)?,
                        ))
                    },
                )
            })
            .await?;

        let (last_rate_date, last_success_at, last_error, count) = info;
        let state = if count == 0 {
            CdiCacheState::Empty
        } else if last_error.is_some() {
            CdiCacheState::RefreshFailed
        } else if last_success_at
            .as_deref()
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .is_some_and(|value| (Utc::now() - value.with_timezone(&Utc)).num_hours() >= 24)
        {
            CdiCacheState::Stale
        } else {
            CdiCacheState::Fresh
        };
        Ok(CdiCacheInfo {
            state,
            last_rate_date,
            last_success_at,
        })
    }

    async fn status(&self) -> Result<CdiStatusDto, AppError> {
        self.db
            .call(|connection| {
                connection.query_row(
                    "SELECT s.series,
                            (SELECT COUNT(*) FROM cdi_rates r WHERE r.series = s.series),
                            s.first_rate_date, s.last_rate_date,
                            s.last_success_at, s.last_attempt_at, s.last_error
                     FROM cdi_sync_status s WHERE s.singleton = 1",
                    [],
                    |row| {
                        Ok(CdiStatusDto {
                            series: row.get(0)?,
                            rate_count: row.get(1)?,
                            first_rate_date: row.get(2)?,
                            last_rate_date: row.get(3)?,
                            last_success_at: row.get(4)?,
                            last_attempt_at: row.get(5)?,
                            last_error: row.get(6)?,
                        })
                    },
                )
            })
            .await
    }

    async fn replace(
        &self,
        series: &str,
        rates: Vec<CdiRate>,
        fetched_at: &str,
    ) -> Result<(), AppError> {
        let series = series.to_owned();
        let fetched_at = timestamp(fetched_at)?;
        self.db
            .transaction(move |transaction| {
                for rate in &rates {
                    transaction
                        .execute(
                            "INSERT INTO cdi_rates (series, rate_date, raw_value, daily_rate, fetched_at)
                             VALUES (?1, ?2, ?3, ?4, ?5)
                             ON CONFLICT(series, rate_date) DO UPDATE SET
                                 raw_value = excluded.raw_value,
                                 daily_rate = excluded.daily_rate,
                                 fetched_at = excluded.fetched_at",
                            params![
                                series,
                                rate.rate_date,
                                rate.raw_value,
                                rate.daily_rate,
                                fetched_at
                            ],
                        )
                        .map_err(map_sqlite_error)?;
                }
                let (min_date, max_date) = transaction
                    .query_row(
                        "SELECT MIN(rate_date), MAX(rate_date) FROM cdi_rates WHERE series = ?1",
                        [&series],
                        |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?)),
                    )
                    .map_err(map_sqlite_error)?;
                transaction
                    .execute(
                        "UPDATE cdi_sync_status
                         SET series = ?1, last_success_at = ?2, last_attempt_at = ?2,
                             last_error = NULL, first_rate_date = ?3, last_rate_date = ?4
                         WHERE singleton = 1",
                        params![series, fetched_at, min_date, max_date],
                    )
                    .map_err(map_sqlite_error)?;
                Ok(())
            })
            .await
    }

    async fn rate_on_or_before(
        &self,
        on_or_before: &str,
    ) -> Result<Option<CdiRateSnapshot>, AppError> {
        let date = on_or_before.to_owned();
        self.db
            .call(move |connection| {
                connection
                    .query_row(
                        "SELECT rate_date, daily_rate, fetched_at FROM cdi_rates
                     WHERE series = '12' AND rate_date <= ?1
                     ORDER BY rate_date DESC LIMIT 1",
                        [date],
                        |row| {
                            Ok(CdiRateSnapshot {
                                rate_date: row.get(0)?,
                                daily_rate: row.get(1)?,
                                fetched_at: row.get(2)?,
                            })
                        },
                    )
                    .optional()
            })
            .await
    }

    async fn rate_on(&self, date: &str) -> Result<Option<CdiRateSnapshot>, AppError> {
        let date = date.to_owned();
        self.db
            .call(move |connection| {
                connection
                    .query_row(
                        "SELECT rate_date, daily_rate, fetched_at FROM cdi_rates
                     WHERE series = '12' AND rate_date = ?1",
                        [date],
                        |row| {
                            Ok(CdiRateSnapshot {
                                rate_date: row.get(0)?,
                                daily_rate: row.get(1)?,
                                fetched_at: row.get(2)?,
                            })
                        },
                    )
                    .optional()
            })
            .await
    }

    async fn record_failure(&self, attempted_at: &str, error: &str) -> Result<(), AppError> {
        let attempted_at = timestamp(attempted_at)?;
        let error = error.to_owned();
        self.db
            .call(move |connection| {
                connection.execute(
                    "UPDATE cdi_sync_status
                         SET last_attempt_at = ?1, last_error = ?2
                         WHERE singleton = 1",
                    params![attempted_at, error],
                )?;
                Ok(())
            })
            .await
    }
}
