use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Utc, Weekday};

use crate::dto::CdiStatusDto;
use crate::error::AppError;
use crate::ports::{CdiFetchRange, CdiRateSource, CdiRatesRepository};

/// SGS 12 — CDI diário, `% p.d.` (6 casas na publicação).
pub const CDI_SERIES: &str = "12";
const SGS_MAX_RANGE_YEARS: i32 = 10;

/// Primeira data aceita pelo SGS para uma consulta que termina em `to`.
pub fn earliest_sgs_date(to: NaiveDate) -> NaiveDate {
    let year = to.year() - SGS_MAX_RANGE_YEARS;
    to.with_year(year)
        .or_else(|| NaiveDate::from_ymd_opt(year, 3, 1))
        .expect("data limite do SGS é válida")
}

/// Converte `0.050788` (percentual ao dia) em `0.00050788` (fração decimal).
pub fn parse_daily_rate(raw: &str) -> Result<f64, AppError> {
    let value = raw.trim();
    let parsed = value
        .parse::<f64>()
        .map_err(|_| AppError::validation("taxa CDI inválida na resposta do SGS"))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(AppError::validation("taxa CDI inválida na resposta do SGS"));
    }
    Ok(parsed / 100.0)
}

fn parse_rate_date(value: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AppError::validation("data da taxa CDI inválida"))
}

/// Decisão pura da atualização automática na abertura. Falha recente bloqueia
/// novas tentativas por 1h; sem falha, um sucesso recente bloqueia por 24h.
pub fn startup_refresh_needed(status: &CdiStatusDto, now: DateTime<Utc>) -> bool {
    if matches!(now.weekday(), Weekday::Sat | Weekday::Sun) {
        return false;
    }
    if status.last_error.is_some() {
        return match parse_timestamp(status.last_attempt_at.as_deref()) {
            Some(attempted_at) => elapsed_hours_at_least(&now, &attempted_at, 1),
            None => true,
        };
    }
    if status.rate_count == 0 {
        return true;
    }
    match parse_timestamp(status.last_success_at.as_deref()) {
        Some(success_at) => elapsed_hours_at_least(&now, &success_at, 24),
        None => true,
    }
}

fn parse_timestamp(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
}

fn elapsed_hours_at_least(now: &DateTime<Utc>, then: &DateTime<Utc>, hours: i64) -> bool {
    now.signed_duration_since(*then).num_hours() >= hours
}

pub struct CdiService<R, S> {
    repo: R,
    source: S,
}

impl<R: CdiRatesRepository, S: CdiRateSource> CdiService<R, S> {
    pub fn new(repo: R, source: S) -> Self {
        Self { repo, source }
    }

    pub async fn status(&self) -> Result<CdiStatusDto, AppError> {
        self.repo.status().await
    }

    /// Avalia o cache local sem abrir conexão com o SGS.
    pub async fn should_refresh_on_startup(&self) -> Result<bool, AppError> {
        let status = self.status().await?;
        Ok(startup_refresh_needed(&status, Utc::now()))
    }

    /// Atualização manual: busca incrementos de 7 dias e substitui o cache
    /// somente após sucesso. Nenhuma requisição ultrapassa 10 anos.
    pub async fn refresh(&self, from: Option<&str>) -> Result<CdiStatusDto, AppError> {
        let info = self.repo.info().await?;
        let today = Local::now().date_naive();
        let earliest = earliest_sgs_date(today);
        let requested = match from {
            Some(value) => {
                let date = parse_rate_date(value)?;
                if date < earliest {
                    return Err(AppError::validation(format!(
                        "o SGS aceita no máximo 10 anos de dados; informe uma data igual ou posterior a {}",
                        earliest.format("%d/%m/%Y")
                    )));
                }
                date
            }
            None => earliest,
        };
        let cached = info
            .last_rate_date
            .as_deref()
            .map(parse_rate_date)
            .transpose()?;
        let start = cached
            .map(|date| date - Duration::days(7))
            .unwrap_or(requested)
            .max(requested)
            .max(earliest);
        if start > today {
            return Err(AppError::validation(
                "a data inicial do CDI não pode ser futura",
            ));
        }

        let fetched_at = Local::now().to_rfc3339();
        let range = CdiFetchRange {
            from: start.format("%Y-%m-%d").to_string(),
            to: today.format("%Y-%m-%d").to_string(),
        };
        match self.source.fetch(range).await {
            Ok(rates) => {
                self.repo.replace(CDI_SERIES, rates, &fetched_at).await?;
            }
            Err(error) => {
                self.repo
                    .record_failure(&fetched_at, &error.message_with_details())
                    .await?;
                return Err(error);
            }
        }
        self.status().await
    }
}

#[cfg(test)]
mod tests {
    use super::{CDI_SERIES, earliest_sgs_date, parse_daily_rate, startup_refresh_needed};
    use crate::dto::CdiStatusDto;
    use chrono::{DateTime, NaiveDate, Utc};

    fn now(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .expect("data")
            .with_timezone(&Utc)
    }

    fn status(
        rate_count: i64,
        last_success_at: Option<&str>,
        last_attempt_at: Option<&str>,
        last_error: Option<&str>,
    ) -> CdiStatusDto {
        CdiStatusDto {
            series: CDI_SERIES.to_owned(),
            rate_count,
            first_rate_date: None,
            last_rate_date: None,
            last_success_at: last_success_at.map(str::to_owned),
            last_attempt_at: last_attempt_at.map(str::to_owned),
            last_error: last_error.map(str::to_owned),
        }
    }

    #[test]
    fn converts_published_percent_to_decimal_fraction() {
        assert!((parse_daily_rate("0.050788").expect("taxa") - 0.00050788).abs() < 1e-12);
        assert!((parse_daily_rate(" 0.050788 ").expect("taxa") - 0.00050788).abs() < 1e-12);
    }

    #[test]
    fn rejects_invalid_rates() {
        for raw in ["", "abc", "0", "-0.05", "NaN"] {
            assert!(parse_daily_rate(raw).is_err(), "valor {raw} deveria falhar");
        }
    }

    #[test]
    fn limits_sgs_window_to_ten_years() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 23).expect("data");
        assert_eq!(
            earliest_sgs_date(today),
            NaiveDate::from_ymd_opt(2016, 11, 23).expect("data")
        );
    }

    #[test]
    fn avoids_invalid_leap_day_when_subtracting_ten_years() {
        let today = NaiveDate::from_ymd_opt(2024, 2, 29).expect("data");
        assert_eq!(
            earliest_sgs_date(today),
            NaiveDate::from_ymd_opt(2014, 3, 1).expect("data")
        );
    }

    #[test]
    fn empty_cache_refreshes_on_a_weekday() {
        let current = now("2026-11-24T12:00:00Z");
        assert!(startup_refresh_needed(
            &status(0, None, None, None),
            current
        ));
    }

    #[test]
    fn recent_success_skips_refresh() {
        let current = now("2026-11-24T12:00:00Z");
        let cached = status(1, Some("2026-11-23T13:00:00Z"), None, None);
        assert!(!startup_refresh_needed(&cached, current));
        let stale = status(1, Some("2026-11-23T12:00:00Z"), None, None);
        assert!(startup_refresh_needed(&stale, current));
    }

    #[test]
    fn old_failure_allows_refresh_even_with_recent_success() {
        let current = now("2026-11-24T12:00:00Z");
        let cached = status(
            1,
            Some("2026-11-23T13:00:00Z"),
            Some("2026-11-24T11:00:00Z"),
            Some("sem rede"),
        );
        assert!(startup_refresh_needed(&cached, current));
    }

    #[test]
    fn recent_failure_waits_one_hour() {
        let current = now("2026-11-24T12:00:00Z");
        let cached = status(
            1,
            Some("2026-11-20T12:00:00Z"),
            Some("2026-11-24T11:30:00Z"),
            Some("sem rede"),
        );
        assert!(!startup_refresh_needed(&cached, current));
    }

    #[test]
    fn weekends_skip_refresh_even_with_empty_cache() {
        let current = now("2026-11-28T12:00:00Z");
        assert!(!startup_refresh_needed(
            &status(0, None, None, None),
            current
        ));
    }
}
