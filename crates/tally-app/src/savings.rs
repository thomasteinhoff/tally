use async_trait::async_trait;
use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use uuid::Uuid;

use crate::cdi;
use crate::dto::{
    AccountDto, CreateSavingsYieldInput, ReplaceSavingsYieldInput, ReverseSavingsYieldInput,
    SavingsOverviewDto, SavingsYieldDto,
};
use crate::error::AppError;
use crate::ports::{CdiRateSnapshot, CdiRatesRepository};
use crate::validation::validate_not_future;

/// Série do Banco Central usada como fonte da taxa diária (SGS 12, `% p.d.`).
pub const CDI_SERIES: &str = cdi::CDI_SERIES;

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

/// Fator de capitalização diário com a taxa publicada do CDI.
///
/// `r` é a taxa diária da SGS 12 em fração decimal e `p = percentual / 10.000`.
/// Em dias úteis, `f = 1 + r × p`. A modalidade `calendar` é uma aproximação
/// matemática do Tally: distribui a mesma taxa em 365 dias corridos por
/// `f = (1 + r × p)^(252/365)`. Não é uma convenção contratual de nenhum
/// produto financeiro específico.
pub fn growth_factor(mode: YieldMode, daily_rate: f64, cdi_percent_bps: i64) -> f64 {
    let scaled = daily_rate * (cdi_percent_bps as f64 / BPS_PER_UNIT);
    match mode {
        YieldMode::Business => 1.0 + scaled,
        YieldMode::Calendar => (1.0 + scaled).powf(BUSINESS_DAYS_PER_YEAR / CALENDAR_DAYS_PER_YEAR),
    }
}

/// Resultado registrado no snapshot: todos os parâmetros necessários para
/// reproduzir o cálculo original, mesmo que a configuração mude depois.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct YieldCalculation {
    pub mode: YieldMode,
    pub contracted_percent_bps: i64,
    pub daily_rate: f64,
    pub factor: f64,
    pub days: i64,
    pub yield_cents: i64,
}

/// Rendimento de um dia sobre o saldo no início do dia.
///
/// O fator é calculado com a taxa publicada e arredondado para centavos inteiros
/// antes de virar `Money`. Nenhum valor de taxa entra no ledger.
pub fn calculate_yield(
    base_cents: i64,
    mode: YieldMode,
    daily_rate: f64,
    cdi_percent_bps: i64,
) -> YieldCalculation {
    let factor = growth_factor(mode, daily_rate, cdi_percent_bps);
    let yield_cents = ((base_cents as f64) * (factor - 1.0)).round() as i64;

    YieldCalculation {
        mode,
        contracted_percent_bps: cdi_percent_bps,
        daily_rate,
        factor,
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
    pub contracted_percent_bps: i64,
    pub daily_rate: f64,
    pub days: i64,
    pub base_cents: i64,
    pub yield_cents: i64,
    /// Procedência da taxa; `None` apenas em snapshots antigos já gravados.
    pub rate: Option<CdiRateSnapshot>,
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
    /// Saldo com todas as postings publicadas até `as_of`, inclusive rendimentos
    /// anteriores do próprio dia.
    async fn balance_through(&self, account_id: &str, as_of: &str) -> Result<i64, AppError>;
    async fn current_balance(&self, account_id: &str) -> Result<i64, AppError>;
    async fn create_yield(&self, snapshot: NewSavingsYield) -> Result<SavingsYieldDto, AppError>;
    async fn reverse_yield(
        &self,
        yield_id: &str,
        reversal: NewSavingsReversal,
    ) -> Result<SavingsYieldDto, AppError>;
    /// Estorno e novo rendimento em uma única transação.
    async fn replace_yield(
        &self,
        yield_id: &str,
        reversal: NewSavingsReversal,
        snapshot: NewSavingsYield,
    ) -> Result<SavingsYieldDto, AppError>;
    async fn find_yield(&self, yield_id: &str) -> Result<SavingsYieldDto, AppError>;
    async fn list_yields(&self, account_id: &str) -> Result<Vec<SavingsYieldDto>, AppError>;
}

pub struct SavingsService<R, C> {
    repo: R,
    cdi: C,
}

impl<R: SavingsRepository, C: CdiRatesRepository> SavingsService<R, C> {
    pub fn new(repo: R, cdi: C) -> Self {
        Self { repo, cdi }
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

    /// Gera o rendimento sobre o saldo disponível até a data informada.
    pub async fn create_yield(
        &self,
        input: CreateSavingsYieldInput,
    ) -> Result<SavingsYieldDto, AppError> {
        let as_of = validate_not_future(&input.as_of, "gerar rendimento")?;
        let snapshot = self.build_yield(&input.account_id, &as_of, None).await?;
        self.repo.create_yield(snapshot).await
    }

    /// Estorno compensatório: preserva o rendimento original e registra um novo
    /// lançamento, sem apagar nem reescrever o histórico.
    pub async fn reverse_yield(
        &self,
        input: ReverseSavingsYieldInput,
    ) -> Result<SavingsYieldDto, AppError> {
        let yield_id = required_id(&input.yield_id, "rendimento")?;
        let requested = validate_not_future(&input.as_of, "estornar")?;
        let snapshot = self.repo.find_yield(&yield_id).await?;
        let as_of = compensating_entry_date(&snapshot, &requested);
        self.validate_reversal(&snapshot, &as_of).await?;

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

    /// Estorna e recalcula na data original sem janela para falha parcial: as
    /// duas operações vivem na mesma transação. Com vários rendimentos no dia,
    /// apenas o último confirmado pode ser substituído, e a base original do
    /// snapshot é reutilizada para não absorver os rendimentos anteriores.
    pub async fn replace_yield(
        &self,
        input: ReplaceSavingsYieldInput,
    ) -> Result<SavingsYieldDto, AppError> {
        let yield_id = required_id(&input.yield_id, "rendimento")?;
        let snapshot = self.repo.find_yield(&yield_id).await?;
        self.ensure_last_confirmed_of_day(&snapshot).await?;
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        let as_of = compensating_entry_date(&snapshot, &today);
        self.validate_reversal(&snapshot, &as_of).await?;

        let replacement = self
            .build_yield(
                &snapshot.account_id,
                &snapshot.as_of,
                Some(snapshot.base_cents),
            )
            .await?;
        let reversal_description = format!("Estorno de rendimento em {as_of}");
        let reversal = NewSavingsReversal {
            entry_id: Uuid::now_v7().to_string(),
            as_of,
            description: reversal_description,
        };
        self.repo
            .replace_yield(&yield_id, reversal, replacement)
            .await
    }

    /// Também usada pela substituição, que reaproveita a base original e pode
    /// recalcular um snapshot legado com data futura; o bloqueio de data futura
    /// fica em `create_yield`.
    async fn build_yield(
        &self,
        account_id: &str,
        as_of: &str,
        base_override: Option<i64>,
    ) -> Result<NewSavingsYield, AppError> {
        let account_id = required_id(account_id, "cofrinho")?;
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
        let date = NaiveDate::parse_from_str(as_of, "%Y-%m-%d")
            .map_err(|_| AppError::validation("data de rendimento inválida"))?;
        if !is_yield_day(mode, date) {
            return Err(AppError::validation(
                "esta modalidade não gera rendimento na data informada",
            ));
        }

        let base_cents = match base_override {
            Some(base_cents) => base_cents,
            None => self.repo.balance_through(&account_id, as_of).await?,
        };
        if base_cents <= 0 {
            return Err(AppError::validation(
                "o cofrinho não tem saldo disponível na data informada",
            ));
        }

        let rate = match mode {
            YieldMode::Business => self.cdi.rate_on(as_of).await?,
            YieldMode::Calendar => self.cdi.rate_on_or_before(as_of).await?,
        }
        .ok_or_else(|| match mode {
            YieldMode::Business => AppError::validation(
                "não há taxa CDI publicada para esta data; atualize as taxas no SGS 12",
            ),
            YieldMode::Calendar => AppError::validation(
                "não há taxa CDI em cache para esta data ou anteriores; atualize as taxas",
            ),
        })?;

        let calculation = calculate_yield(base_cents, mode, rate.daily_rate, percent_bps);
        if calculation.yield_cents <= 0 {
            return Err(AppError::validation("o rendimento calculado é zero"));
        }

        let description = format!("Rendimento do cofrinho em {as_of}");
        Ok(NewSavingsYield {
            id: Uuid::now_v7().to_string(),
            entry_id: Uuid::now_v7().to_string(),
            account_id,
            as_of: as_of.to_owned(),
            mode: calculation.mode,
            contracted_percent_bps: calculation.contracted_percent_bps,
            daily_rate: calculation.daily_rate,
            days: calculation.days,
            base_cents,
            yield_cents: calculation.yield_cents,
            rate: Some(rate),
            description,
        })
    }

    async fn ensure_last_confirmed_of_day(
        &self,
        snapshot: &SavingsYieldDto,
    ) -> Result<(), AppError> {
        let history = self.repo.list_yields(&snapshot.account_id).await?;
        let last = history
            .iter()
            .find(|item| item.as_of == snapshot.as_of && item.status == "confirmed");
        if last.map(|item| item.id.as_str()) != Some(snapshot.id.as_str()) {
            return Err(AppError::conflict(
                "só é possível substituir o último rendimento confirmado da data",
            ));
        }
        Ok(())
    }

    async fn validate_reversal(
        &self,
        snapshot: &SavingsYieldDto,
        as_of: &str,
    ) -> Result<(), AppError> {
        if snapshot.status != "confirmed" || snapshot.reversal_entry_id.is_some() {
            return Err(AppError::conflict("este rendimento já foi estornado"));
        }
        if as_of < snapshot.as_of.as_str() {
            return Err(AppError::validation(format!(
                "a data do estorno ({as_of}) não pode ser anterior à data do rendimento ({})",
                snapshot.as_of
            )));
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
                "o valor deste rendimento já foi resgatado ou não está disponível no cofrinho",
            ));
        }
        Ok(())
    }
}

/// Registros legados podem ter `as_of` no futuro. Nesse caso não existe data
/// passada que satisfaça `estorno >= rendimento`, então a compensação usa a
/// própria data do rendimento. Registros normais mantêm a data pedida e a
/// validação contra datas anteriores.
fn compensating_entry_date(snapshot: &SavingsYieldDto, requested: &str) -> String {
    let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
    if snapshot.as_of > today {
        snapshot.as_of.clone()
    } else {
        requested.to_owned()
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
        YieldMode, calculate_yield, growth_factor, is_yield_day, normalize_savings_config,
    };

    /// Taxa real da SGS 12 em 19/11/2026: `0,050788% ao dia`.
    const REAL_DAILY_RATE: f64 = 0.00050788;

    fn date(value: &str) -> NaiveDate {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").expect("data válida")
    }

    #[test]
    fn published_rate_matches_official_sgs_12_cross_check() {
        let factor = growth_factor(YieldMode::Business, REAL_DAILY_RATE, 10_000);
        let annual = factor.powf(252.0) - 1.0;

        assert!((factor - 1.00050788).abs() < 1e-12);
        assert!((annual - 0.136499893).abs() < 1e-8, "anual {annual}");
    }

    #[test]
    fn contracted_percent_scales_the_published_rate() {
        let base = growth_factor(YieldMode::Business, REAL_DAILY_RATE, 10_000);
        let high = growth_factor(YieldMode::Business, REAL_DAILY_RATE, 13_000);

        assert!((high - 1.000660244).abs() < 1e-12);
        assert!(high > base);
    }

    #[test]
    fn real_rate_produces_the_official_expected_yields() {
        for (percent, business, calendar) in [(10_000, 51, 35), (11_500, 58, 40), (13_000, 66, 46)]
        {
            let business_yield =
                calculate_yield(100_000, YieldMode::Business, REAL_DAILY_RATE, percent);
            let calendar_yield =
                calculate_yield(100_000, YieldMode::Calendar, REAL_DAILY_RATE, percent);

            assert_eq!(business_yield.yield_cents, business, "business {percent}");
            assert_eq!(calendar_yield.yield_cents, calendar, "calendar {percent}");
            assert_eq!(business_yield.contracted_percent_bps, percent);
            assert_eq!(business_yield.days, 1);
            if percent == 10_000 {
                assert!(
                    (calendar_yield.factor - 1.00035061835717).abs() < 1e-8,
                    "fator calendar: {}",
                    calendar_yield.factor
                );
            }
        }
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
