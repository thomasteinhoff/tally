use async_trait::async_trait;
use chrono::NaiveDate;
use reqwest::StatusCode;
use serde::Deserialize;
use tally_app::cdi::parse_daily_rate;
use tally_app::error::AppError;
use tally_app::ports::{CdiFetchRange, CdiRate, CdiRateSource};

const BASE_URL: &str = "https://api.bcb.gov.br/dados/serie/bcdata.sgs.12/dados";
const SGS_NOT_MODIFIED_MESSAGE: &str = "o SGS devolveu 304 sem corpo utilizável";

#[derive(Debug, Deserialize)]
struct SgsObservation {
    #[serde(rename = "data")]
    date: String,
    #[serde(rename = "valor")]
    value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResponseFormat {
    Json,
    Csv,
}

impl ResponseFormat {
    fn name(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Csv => "csv",
        }
    }
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

    /// `None` significa 304: o CDN devolveu uma resposta sem corpo.
    async fn fetch_body(
        &self,
        format: ResponseFormat,
        from: &str,
        to: &str,
    ) -> Result<Option<String>, AppError> {
        let response = self
            .client
            .get(BASE_URL)
            .query(&[
                ("formato", format.name()),
                ("dataInicial", from),
                ("dataFinal", to),
            ])
            .send()
            .await
            .map_err(|error| {
                AppError::io("não foi possível consultar as taxas CDI", error.to_string())
            })?;
        let status = response.status();
        if is_not_modified(status) {
            return Ok(None);
        }
        let body = response.text().await.map_err(|error| {
            AppError::io("não foi possível ler a resposta do SGS", error.to_string())
        })?;
        if !status.is_success() {
            return Err(http_status_error(status, format.name(), &body));
        }
        Ok(Some(body))
    }
}

#[async_trait]
impl CdiRateSource for HttpCdiRateSource {
    async fn fetch(&self, range: CdiFetchRange) -> Result<Vec<CdiRate>, AppError> {
        let from = convert_date(&range.from)?;
        let to = convert_date(&range.to)?;

        let (body, format) = match self.fetch_body(ResponseFormat::Json, &from, &to).await? {
            Some(body) => (body, ResponseFormat::Json),
            None => {
                // ponytail: fallback sem mudar o intervalo; CDN devolve 304 para
                // algumas URLs JSON em cache, mas a mesma janela em CSV responde 200.
                match self.fetch_body(ResponseFormat::Csv, &from, &to).await? {
                    Some(body) => (body, ResponseFormat::Csv),
                    None => {
                        return Err(AppError::io(
                            SGS_NOT_MODIFIED_MESSAGE,
                            "HTTP 304 Not Modified nas representações JSON e CSV; sem corpo para desserializar",
                        ));
                    }
                }
            }
        };

        match format {
            ResponseFormat::Json => parse_json(&body),
            ResponseFormat::Csv => parse_csv(&body),
        }
    }
}

fn is_not_modified(status: StatusCode) -> bool {
    status == StatusCode::NOT_MODIFIED
}

fn http_status_error(status: StatusCode, format: &str, body: &str) -> AppError {
    let body = truncate(body.trim(), 1200);
    let details = if body.is_empty() {
        format!("HTTP {status} ({format})")
    } else {
        format!("HTTP {status} ({format}): {body}")
    };
    let message = if status.is_client_error() {
        "o SGS recusou a consulta"
    } else if status.is_server_error() {
        "o SGS falhou ao responder a consulta"
    } else {
        "o SGS respondeu com status inesperado"
    };
    AppError::io(message, details)
}

fn truncate(value: &str, limit: usize) -> String {
    let shortened: String = value.chars().take(limit).collect();
    if value.chars().count() > limit {
        format!("{shortened}…")
    } else {
        shortened
    }
}

fn parse_json(body: &str) -> Result<Vec<CdiRate>, AppError> {
    let observations: Vec<SgsObservation> = serde_json::from_str(body)
        .map_err(|error| AppError::io("resposta inválida do SGS", format!("JSON: {error}")))?;
    observations
        .into_iter()
        .map(|item| build_rate(&item.date, &item.value))
        .collect()
}

fn parse_csv(body: &str) -> Result<Vec<CdiRate>, AppError> {
    let mut lines = body.lines();
    let header = lines.next().unwrap_or_default();
    if !header.contains("data") || !header.contains("valor") {
        return Err(AppError::io(
            "resposta inválida do SGS",
            format!("CSV: cabeçalho inesperado ({header})"),
        ));
    }
    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut fields = line.split(';');
            let date = unquote(fields.next().unwrap_or_default());
            let raw = unquote(fields.next().unwrap_or_default()).replace(',', ".");
            build_rate(date, &raw)
        })
        .collect()
}

fn unquote(value: &str) -> &str {
    value.trim().trim_matches('"').trim()
}

fn build_rate(date: &str, raw_value: &str) -> Result<CdiRate, AppError> {
    let rate_date = NaiveDate::parse_from_str(date, "%d/%m/%Y")
        .map_err(|_| AppError::validation("data inválida na resposta do SGS"))?
        .format("%Y-%m-%d")
        .to_string();
    Ok(CdiRate {
        rate_date,
        daily_rate: parse_daily_rate(raw_value)?,
        raw_value: raw_value.trim().to_owned(),
    })
}

fn convert_date(value: &str) -> Result<String, AppError> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AppError::validation("data de consulta do CDI inválida"))?;
    Ok(date.format("%d/%m/%Y").to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        ResponseFormat, SgsObservation, convert_date, http_status_error, is_not_modified,
        parse_csv, parse_json,
    };

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

    #[test]
    fn parses_json_observations() {
        let rates = parse_json(
            r#"[{"data":"19/11/2026","valor":"0.050788"},{"data":"18/11/2026","valor":"0.051660"}]"#,
        )
        .expect("json");
        assert_eq!(rates.len(), 2);
        assert_eq!(rates[0].rate_date, "2026-11-19");
        assert_eq!(rates[0].raw_value, "0.050788");
        assert!((rates[0].daily_rate - 0.00050788).abs() < 1e-12);
    }

    #[test]
    fn parses_csv_fallback_observations() {
        let body = "\"data\";\"valor\"\r\n\"19/11/2026\";\"0,050788\"\r\n";
        let rates = parse_csv(body).expect("csv");
        assert_eq!(rates.len(), 1);
        assert_eq!(rates[0].rate_date, "2026-11-19");
        assert_eq!(rates[0].raw_value, "0.050788");
        assert!((rates[0].daily_rate - 0.00050788).abs() < 1e-12);
    }

    #[test]
    fn rejects_unexpected_csv_header() {
        assert!(parse_csv("data,valor\n19/11/2026,0.050788").is_err());
    }

    #[test]
    fn keeps_status_and_body_on_client_error() {
        let body = r#"{"error":"O sistema aceita uma janela de consulta de, no máximo, 10 anos"}"#;
        let error = http_status_error(reqwest::StatusCode::NOT_ACCEPTABLE, "json", body);
        assert_eq!(error.message(), "o SGS recusou a consulta");
        assert!(
            error
                .details()
                .expect("detalhes")
                .contains("406 Not Acceptable")
        );
        assert!(error.details().expect("detalhes").contains("10 anos"));
    }

    #[test]
    fn keeps_status_and_body_on_server_error() {
        let error = http_status_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, "json", "boom");
        assert_eq!(error.message(), "o SGS falhou ao responder a consulta");
        assert!(error.details().expect("detalhes").contains("500"));
        assert!(error.details().expect("detalhes").contains("boom"));
    }

    #[test]
    fn separates_not_modified_from_other_statuses() {
        assert!(is_not_modified(reqwest::StatusCode::NOT_MODIFIED));
        assert!(!is_not_modified(reqwest::StatusCode::OK));
        assert!(!is_not_modified(reqwest::StatusCode::NOT_ACCEPTABLE));
    }

    #[test]
    fn keeps_format_name_for_query() {
        assert_eq!(ResponseFormat::Json.name(), "json");
        assert_eq!(ResponseFormat::Csv.name(), "csv");
    }
}
