use chrono::{Local, NaiveDate};
use tally_domain::period::Period;

use crate::error::AppError;

const MAX_SAFE_CENTS: u64 = 9_007_199_254_740_991;
const MAX_DESCRIPTION_CHARS: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OccurredAt {
    pub time: Option<String>,
    pub manual_period: Option<String>,
}

pub(crate) fn normalize_name(
    raw: &str,
    entity: &str,
    max_chars: usize,
) -> Result<String, AppError> {
    let name = raw.trim();

    if name.is_empty() {
        return Err(AppError::validation(format!("informe o nome da {entity}")));
    }

    if name.chars().count() > max_chars {
        return Err(AppError::validation(format!(
            "o nome da {entity} deve ter no máximo {max_chars} caracteres"
        )));
    }

    Ok(name.to_owned())
}

pub(crate) fn validate_date(raw: &str) -> Result<String, AppError> {
    let value = raw.trim();
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|date| date.format("%Y-%m-%d").to_string())
        .map_err(|_| AppError::validation("informe uma data válida no formato AAAA-MM-DD"))
}

/// Datas de rendimento/estorno novos não podem estar no futuro: a operação
/// depende do saldo no início do dia e de taxas já publicadas.
/// `operation` deixa a mensagem declarar qual ação foi bloqueada.
pub fn validate_not_future(raw: &str, operation: &str) -> Result<String, AppError> {
    let date = validate_date(raw)?;
    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
    if date > today {
        return Err(AppError::validation(format!(
            "não é possível {operation} em data futura"
        )));
    }
    Ok(date)
}

pub(crate) fn normalize_occurred(
    time: Option<&str>,
    manual_period: Option<&str>,
) -> Result<OccurredAt, AppError> {
    let time = time.map(str::trim).filter(|value| !value.is_empty());
    let manual_period = manual_period
        .map(str::trim)
        .filter(|value| !value.is_empty());

    match (time, manual_period) {
        (Some(time), None) => {
            Period::from_time(time).ok_or_else(|| {
                AppError::validation("informe um horário válido no formato HH:MM")
            })?;
            Ok(OccurredAt {
                time: Some(time.to_owned()),
                manual_period: None,
            })
        }
        (None, Some(period)) => {
            let period = Period::parse(period)
                .ok_or_else(|| AppError::validation("informe um período válido"))?;
            Ok(OccurredAt {
                time: None,
                manual_period: Some(period.as_str().to_owned()),
            })
        }
        (Some(_), Some(_)) => Err(AppError::validation(
            "informe horário ou período manual, não os dois",
        )),
        (None, None) => Err(AppError::validation(
            "informe um horário ou escolha um período manual",
        )),
    }
}

pub(crate) fn normalize_description(raw: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(value) = raw else {
        return Ok(None);
    };
    let value = value.trim();

    if value.is_empty() {
        return Ok(None);
    }

    if value.chars().count() > MAX_DESCRIPTION_CHARS {
        return Err(AppError::validation(format!(
            "a descrição deve ter no máximo {MAX_DESCRIPTION_CHARS} caracteres"
        )));
    }

    Ok(Some(value.to_owned()))
}

pub(crate) fn validate_nonzero_cents(cents: i64) -> Result<i64, AppError> {
    if cents == 0 {
        return Err(AppError::validation("informe um valor diferente de zero"));
    }

    if cents.unsigned_abs() > MAX_SAFE_CENTS {
        return Err(AppError::validation(
            "o valor excede o limite seguro de centavos",
        ));
    }

    Ok(cents)
}

pub(crate) fn validate_positive_cents(cents: i64) -> Result<i64, AppError> {
    if cents <= 0 {
        return Err(AppError::validation("informe um valor maior que zero"));
    }

    validate_nonzero_cents(cents)
}

pub(crate) fn validate_filter_date(raw: Option<&str>) -> Result<Option<String>, AppError> {
    raw.map(validate_date).transpose()
}
