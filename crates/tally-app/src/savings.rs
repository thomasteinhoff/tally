use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use uuid::Uuid;

use crate::dto::{
    AccountDto, CreateSavingsYieldInput, ReverseSavingsYieldInput, SavingsOverviewDto,
    SavingsYieldDto,
};
use crate::error::AppError;
use crate::validation::validate_date;

/// CDI anual usado como referência, em pontos-base (1_000 = 10,00% a.a.).
///
/// ponytail: valor provisório e isolado aqui porque a taxa exata ainda depende
/// de decisão de produto. Trocar esta única constante quando confirmada; os
/// snapshots já gravados guardam a taxa usada e não são recalculados.
pub const CDI_ANNUAL_BPS: i64 = 1_000;

const BPS_PER_UNIT: f64 = 10_000.0;
const PERCENT_TO_BPS: f64 = 100.0;
const MAX_CDI_PERCENT: f64 = 10_000.0;

/// Capitalização diária: 252 dias úteis ou 365 dias corridos. As duas bases
/// não são intercambiáveis; cada modalidade usa a sua.
const BUSINESS_DAYS_PER_YEAR: f64 = 252.0;
const CALENDAR_DAYS_PER_YEAR: f64 = 365.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YieldMode {
    Calendar,
    Business,
}

impl YieldMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Calendar => "calendar",
            Self::Business => "business",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "calendar" => Some(Self::Calendar),
            "business" => Some(Self::Business),
            _ => None,
        }
    }

    const fn periods_per_year(self) -> f64 {
        match self {
            Self::Calendar => CALENDAR_DAYS_PER_YEAR,
            Self::Business => BUSINESS_DAYS_PER_YEAR,
        }
    }
}

/// Configuração persistida na conta. Só existe para cofrinhos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavingsConfig {
    pub mode: YieldMode,
    pub cdi_percent_bps: i64,
}

/// Normaliza a configuração vinda da UI. Contas comuns não podem carregar
/// campos de cofrinho; cofrinhos exigem modalidade e percentual válidos.
pub fn normalize_savings_config(
    is_savings: bool,
    yield_mode: Option<&str>,
    cdi_percent: Option<f64>,
) -> Result<Option<SavingsConfig>, AppError> {
    if !is_savings {
        if yield_mode.is_some() || cdi_percent.is_some() {
            return Err(AppError::validation(
                "modalidade e percentual do CDI só existem em cofrinhos",
            ));
        }
        return Ok(None);
    }

    let mode = yield_mode
        .and_then(YieldMode::parse)
        .ok_or_else(|| AppError::validation("modalidade de rendimento inválida"))?;
    let percent = cdi_percent.ok_or_else(|| {
        AppError::validation("informe o percentual contratado do CDI do cofrinho")
    })?;

    if !percent.is_finite() || percent <= 0.0 || percent > MAX_CDI_PERCENT {
        return Err(AppError::validation(
            "informe um percentual contratado entre 0 e 10000",
        ));
    }

    let cdi_percent_bps = (percent * PERCENT_TO_BPS).round() as i64;
    if cdi_percent_bps <= 0 {
        return Err(AppError::validation("percentual contratado muito pequeno"));
    }

    Ok(Some(SavingsConfig {
        mode,
        cdi_percent_bps,
    }))
}

/// Converte a taxa anual efetiva contratada em taxa diária equivalente.
///
/// A taxa anual contratada é `CDI anual × percentual`. A conversão é
/// geométrica — `(1 + anual)^(1/n) - 1` — para que capitalizar `n` dias
/// reproduza a taxa anual. Dividir a taxa anual por 365 ou 252 não é a
/// mesma convenção e não deve ser usado aqui.
pub fn daily_rate(mode: YieldMode, cdi_percent_bps: i64) -> f64 {
    let annual_rate =
        (CDI_ANNUAL_BPS as f64 / BPS_PER_UNIT) * (cdi_percent_bps as f64 / BPS_PER_UNIT);
    (1.0 + annual_rate).powf(1.0 / mode.periods_per_year()) - 1.0
}

/// Resultado registrado no snapshot: todos os parâmetros necessários para
/// reproduzir o cálculo original, mesmo que a configuração mude depois.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct YieldCalculation {
    pub mode: YieldMode,
    pub cdi_annual_bps: i64,
    pub contracted_percent_bps: i64,
    pub daily_rate: f64,
    pub days: i64,
    pub yield_cents: i64,
}

/// Rendimento de um dia sobre o saldo no início do dia.
///
/// A taxa é contínua por natureza; o resultado é arredondado para centavos
/// inteiros antes de virar `Money`. Nenhum valor de taxa entra no ledger.
pub fn calculate_yield(base_cents: i64, mode: YieldMode, cdi_percent_bps: i64) -> YieldCalculation {
    let rate = daily_rate(mode, cdi_percent_bps);
    let yield_cents = ((base_cents as f64) * rate).round() as i64;

    YieldCalculation {
        mode,
        cdi_annual_bps: CDI_ANNUAL_BPS,
        contracted_percent_bps: cdi_percent_bps,
        daily_rate: rate,
        days: 1,
        yield_cents,
    }
}

/// Um rendimento diário só é gerado em dias produtivos da modalidade.
pub fn is_yield_day(mode: YieldMode, date: NaiveDate) -> bool {
    match mode {
        YieldMode::Calendar => true,
        YieldMode::Business => is_business_day(date),
    }
}

/// Feriados nacionais brasileiros, calculados offline e de forma determinística.
///
/// Inclui o Dia Nacional de Zumbi e da Consciência Negra a partir de 2024,
/// quando a Lei 14.759/2023 o tornou feriado nacional.
///
/// ponytail: Carnaval (-48/-47) e Corpus Christi (+60) são pontos facultativos
/// federais, tratados como não úteis por serem observados pelo sistema
/// bancário. Feriados estaduais/municipais e pontes ficam de fora; adicionar
/// tabela por região se um cofrinho precisar dessa cobertura.
fn national_holidays(year: i32) -> Vec<NaiveDate> {
    let fixed = [
        (1, 1),
        (4, 21),
        (5, 1),
        (9, 7),
        (10, 12),
        (11, 2),
        (11, 15),
        (12, 25),
    ];
    let mut holidays = fixed
        .iter()
        .filter_map(|(month, day)| date(year, *month, *day))
        .collect::<Vec<_>>();

    if year >= 2024 {
        holidays.push(date(year, 11, 20).expect("20 de novembro é data válida"));
    }

    let easter = easter_sunday(year);
    for offset in [-48, -47, -2, 60] {
        holidays.push(easter + Duration::days(offset));
    }

    holidays
}

fn date(year: i32, month: u32, day: u32) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, day)
}

/// Algoritmo de Meeus/Jones/Butcher para o domingo de Páscoa gregoriano.
fn easter_sunday(year: i32) -> NaiveDate {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = ((h + l - 7 * m + 114) % 31) + 1;

    date(year, month as u32, day as u32).expect("data de Páscoa válida")
}

fn is_business_day(value: NaiveDate) -> bool {
    !matches!(value.weekday(), Weekday::Sat | Weekday::Sun)
        && !national_holidays(value.year()).contains(&value)
}

/// Snapshot completo de um rendimento, gravado antes do lançamento para que
/// os triggers de cofrinho reconheçam o entry como rendimento.
#[derive(Debug, Clone)]
pub struct NewSavingsYield {
    pub id: String,
    pub entry_id: String,
    pub account_id: String,
    pub as_of: String,
    pub mode: YieldMode,
    pub cdi_annual_bps: i64,
    pub contracted_percent_bps: i64,
    pub daily_rate: f64,
    pub days: i64,
    pub base_cents: i64,
    pub yield_cents: i64,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct NewSavingsReversal {
    pub entry_id: String,
    pub as_of: String,
    pub description: String,
}

#[async_trait]
pub trait SavingsRepository: Send + Sync {
    async fn savings_account(&self, id: &str) -> Result<AccountDto, AppError>;
    /// Saldo com todas as postings publicadas em data estritamente anterior a `as_of`.
    async fn balance_before(&self, account_id: &str, as_of: &str) -> Result<i64, AppError>;
    async fn current_balance(&self, account_id: &str) -> Result<i64, AppError>;
    async fn create_yield(&self, snapshot: NewSavingsYield) -> Result<SavingsYieldDto, AppError>;
    async fn reverse_yield(
        &self,
        yield_id: &str,
        reversal: NewSavingsReversal,
    ) -> Result<SavingsYieldDto, AppError>;
    async fn find_yield(&self, yield_id: &str) -> Result<SavingsYieldDto, AppError>;
    async fn list_yields(&self, account_id: &str) -> Result<Vec<SavingsYieldDto>, AppError>;
}

pub struct SavingsService<R> {
    repo: R,
}

impl<R: SavingsRepository> SavingsService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn overview(&self, account_id: &str) -> Result<SavingsOverviewDto, AppError> {
        let account_id = required_id(account_id, "cofrinho")?;
        let account = self.repo.savings_account(&account_id).await?;
        if !account.is_savings {
            return Err(AppError::validation("a conta não é um cofrinho"));
        }
        let yields = self.repo.list_yields(&account_id).await?;
        Ok(SavingsOverviewDto { account, yields })
    }

    /// Gera o rendimento de um dia sobre o saldo no início da data informada.
    pub async fn create_yield(
        &self,
        input: CreateSavingsYieldInput,
    ) -> Result<SavingsYieldDto, AppError> {
        let account_id = required_id(&input.account_id, "cofrinho")?;
        let as_of = validate_date(&input.as_of)?;
        let account = self.repo.savings_account(&account_id).await?;
        if !account.is_savings {
            return Err(AppError::validation("a conta não é um cofrinho"));
        }
        if account.archived_at.is_some() {
            return Err(AppError::validation(
                "cofrinhos arquivados não aceitam novos rendimentos",
            ));
        }

        let mode = account
            .yield_mode
            .as_deref()
            .and_then(YieldMode::parse)
            .ok_or_else(|| AppError::validation("modalidade de rendimento inválida"))?;
        let percent_bps = account
            .cdi_percent_bps
            .ok_or_else(|| AppError::validation("percentual contratado não configurado"))?;
        let date = NaiveDate::parse_from_str(&as_of, "%Y-%m-%d")
            .map_err(|_| AppError::validation("data de rendimento inválida"))?;
        if !is_yield_day(mode, date) {
            return Err(AppError::validation(
                "esta modalidade não gera rendimento na data informada",
            ));
        }

        let base_cents = self.repo.balance_before(&account_id, &as_of).await?;
        if base_cents <= 0 {
            return Err(AppError::validation(
                "o cofrinho não tem saldo no início do dia",
            ));
        }

        let calculation = calculate_yield(base_cents, mode, percent_bps);
        if calculation.yield_cents <= 0 {
            return Err(AppError::validation("o rendimento calculado é zero"));
        }

        let description = format!("Rendimento do cofrinho em {as_of}");
        self.repo
            .create_yield(NewSavingsYield {
                id: Uuid::now_v7().to_string(),
                entry_id: Uuid::now_v7().to_string(),
                account_id,
                as_of,
                mode: calculation.mode,
                cdi_annual_bps: calculation.cdi_annual_bps,
                contracted_percent_bps: calculation.contracted_percent_bps,
                daily_rate: calculation.daily_rate,
                days: calculation.days,
                base_cents,
                yield_cents: calculation.yield_cents,
                description,
            })
            .await
    }

    /// Estorno compensatório: preserva o rendimento original e libera a data
    /// para uma substituição.
    pub async fn reverse_yield(
        &self,
        input: ReverseSavingsYieldInput,
    ) -> Result<SavingsYieldDto, AppError> {
        let yield_id = required_id(&input.yield_id, "rendimento")?;
        let as_of = validate_date(&input.as_of)?;
        let snapshot = self.repo.find_yield(&yield_id).await?;
        if snapshot.status != "confirmed" || snapshot.reversal_entry_id.is_some() {
            return Err(AppError::conflict("este rendimento já foi estornado"));
        }
        if as_of < snapshot.as_of {
            return Err(AppError::validation(
                "a data do estorno não pode ser anterior ao rendimento",
            ));
        }

        let account = self.repo.savings_account(&snapshot.account_id).await?;
        if account.archived_at.is_some() {
            return Err(AppError::validation(
                "cofrinhos arquivados não aceitam estornos",
            ));
        }

        let balance_cents = self.repo.current_balance(&snapshot.account_id).await?;
        if balance_cents - snapshot.yield_cents < 0 {
            return Err(AppError::validation(
                "saldo insuficiente no cofrinho para estornar este rendimento",
            ));
        }

        let description = format!("Estorno de rendimento em {as_of}");
        self.repo
            .reverse_yield(
                &yield_id,
                NewSavingsReversal {
                    entry_id: Uuid::now_v7().to_string(),
                    as_of,
                    description,
                },
            )
            .await
    }
}

fn required_id(value: &str, entity: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::validation(format!("informe o {entity}")));
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::{
        CDI_ANNUAL_BPS, YieldMode, calculate_yield, daily_rate, is_yield_day,
        normalize_savings_config,
    };

    fn date(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").expect("data válida")
    }

    #[test]
    fn geometric_rate_reproduces_the_annual_rate_on_each_basis() {
        let calendar = daily_rate(YieldMode::Calendar, 10_000);
        let business = daily_rate(YieldMode::Business, 10_000);

        assert!((1.0 + calendar).powf(365.0) - 1.0 - 0.10 < 1e-9);
        assert!((1.0 + business).powf(252.0) - 1.0 - 0.10 < 1e-9);
        assert!(business > calendar);
    }

    #[test]
    fn contracted_percent_scales_the_annual_rate() {
        let base = daily_rate(YieldMode::Calendar, 10_000);
        let contracted = daily_rate(YieldMode::Calendar, 11_500);
        let expected = (1.0_f64 + 0.10_f64 * 1.15_f64).powf(1.0 / 365.0) - 1.0;

        assert!((contracted - expected).abs() < 1e-12);
        assert!(contracted > base);

        let high = daily_rate(YieldMode::Calendar, 13_000);
        assert!((1.0 + high).powf(365.0) - 1.0 - 0.13 < 1e-9);
        assert!(high > contracted);
    }

    #[test]
    fn yield_uses_one_day_of_the_selected_mode() {
        let calendar = calculate_yield(100_000, YieldMode::Calendar, 10_000);
        let business = calculate_yield(100_000, YieldMode::Business, 10_000);

        assert_eq!(calendar.cdi_annual_bps, CDI_ANNUAL_BPS);
        assert_eq!(calendar.contracted_percent_bps, 10_000);
        assert_eq!(calendar.days, 1);
        assert_eq!(calendar.yield_cents, 26);
        assert!(business.yield_cents > calendar.yield_cents);
    }

    #[test]
    fn business_mode_skips_weekends_and_national_holidays() {
        assert!(is_yield_day(YieldMode::Calendar, date("2026-12-25")));
        assert!(!is_yield_day(YieldMode::Business, date("2026-12-25")));
        assert!(!is_yield_day(YieldMode::Business, date("2026-11-21")));
        // 20/11 é feriado nacional desde 2024, mas não era em 2023.
        assert!(!is_yield_day(YieldMode::Business, date("2026-11-20")));
        assert!(is_yield_day(YieldMode::Calendar, date("2026-11-20")));
        assert!(is_yield_day(YieldMode::Business, date("2023-11-20")));
    }

    #[test]
    fn savings_config_is_validated_without_ui() {
        assert!(
            normalize_savings_config(false, None, None)
                .expect("conta comum")
                .is_none()
        );
        assert!(normalize_savings_config(false, Some("calendar"), None).is_err());
        assert!(normalize_savings_config(true, None, Some(100.0)).is_err());
        assert!(normalize_savings_config(true, Some("invalid"), Some(100.0)).is_err());
        assert!(normalize_savings_config(true, Some("calendar"), Some(0.0)).is_err());

        let config = normalize_savings_config(true, Some("business"), Some(115.0))
            .expect("configuração válida")
            .expect("cofrinho");
        assert_eq!(config.mode, YieldMode::Business);
        assert_eq!(config.cdi_percent_bps, 11_500);

        let high = normalize_savings_config(true, Some("calendar"), Some(130.0))
            .expect("130% do CDI")
            .expect("cofrinho");
        assert_eq!(high.cdi_percent_bps, 13_000);
    }
}
