use chrono::{Datelike, Duration, Local, NaiveDate};

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
    use super::{earliest_sgs_date, parse_daily_rate};
    use chrono::NaiveDate;

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
}
