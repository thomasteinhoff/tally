use async_trait::async_trait;
use chrono::NaiveDate;
use serde::Deserialize;
use tally_app::cdi::parse_daily_rate;
use tally_app::error::AppError;
use tally_app::ports::{CdiFetchRange, CdiRate, CdiRateSource};

const BASE_URL: &str = "https://api.bcb.gov.br/dados/serie/bcdata.sgs.12/dados";

#[derive(Debug, Deserialize)]
struct SgsObservation {
    #[serde(rename = "data")]
    date: String,
    #[serde(rename = "valor")]
    value: String,
}

pub struct HttpCdiRateSource {
    client: reqwest::Client,
}

impl Default for HttpCdiRateSource {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpCdiRateSource {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("cliente HTTP do CDI"),
        }
    }
}

#[async_trait]
impl CdiRateSource for HttpCdiRateSource {
    async fn fetch(&self, range: CdiFetchRange) -> Result<Vec<CdiRate>, AppError> {
        let from = convert_date(&range.from)?;
        let to = convert_date(&range.to)?;
        let response = self
            .client
            .get(BASE_URL)
            .query(&[
                ("formato", "json"),
                ("dataInicial", from.as_str()),
                ("dataFinal", to.as_str()),
            ])
            .send()
            .await
            .map_err(|error| {
                AppError::io("não foi possível consultar as taxas CDI", error.to_string())
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(AppError::io(
                "o SGS não respondeu às taxas CDI",
                status.to_string(),
            ));
        }
        let observations = response
            .json::<Vec<SgsObservation>>()
            .await
            .map_err(|error| AppError::io("resposta inválida do SGS", error.to_string()))?;

        observations
            .into_iter()
            .map(|item| {
                let rate_date = NaiveDate::parse_from_str(&item.date, "%d/%m/%Y")
                    .map_err(|_| AppError::validation("data inválida na resposta do SGS"))?
                    .format("%Y-%m-%d")
                    .to_string();
                Ok(CdiRate {
                    rate_date,
                    daily_rate: parse_daily_rate(&item.value)?,
                    raw_value: item.value.trim().to_owned(),
                })
            })
            .collect()
    }
}

fn convert_date(value: &str) -> Result<String, AppError> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AppError::validation("data de consulta do CDI inválida"))?;
    Ok(date.format("%d/%m/%Y").to_string())
}

#[cfg(test)]
mod tests {
    use super::{SgsObservation, convert_date};

    #[test]
    fn converts_dates_to_sgs_format() {
        assert_eq!(convert_date("2026-11-19").expect("data"), "19/11/2026");
    }

    #[test]
    fn accepts_sgs_field_names() {
        let item: SgsObservation =
            serde_json::from_str(r#"{"data":"19/11/2026","valor":"0.050788"}"#).expect("json");
        assert_eq!(item.date, "19/11/2026");
        assert_eq!(item.value, "0.050788");
    }
}
