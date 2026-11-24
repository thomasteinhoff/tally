use async_trait::async_trait;

use crate::dto::{CdiStatusDto, IntegrityReportDto, SettingsDto};
use crate::error::AppError;

#[async_trait]
pub trait BootstrapRepository: Send + Sync {
    async fn schema_version(&self) -> Result<i64, AppError>;
    async fn settings(&self) -> Result<SettingsDto, AppError>;
    async fn update_settings(&self, backup_retention: u32) -> Result<SettingsDto, AppError>;
    async fn system_account_count(&self) -> Result<i64, AppError>;
    async fn integrity_check(&self) -> Result<IntegrityReportDto, AppError>;
}

/// Uma observação da SGS 12: valor bruto em `% p.d.` e data `YYYY-MM-DD`.
#[derive(Debug, Clone, PartialEq)]
pub struct CdiRate {
    pub rate_date: String,
    pub raw_value: String,
    pub daily_rate: f64,
}

/// Intervalo enviado ao SGS: apenas série e datas, nenhum dado local.
#[derive(Debug, Clone, PartialEq)]
pub struct CdiFetchRange {
    pub from: String,
    pub to: String,
}

#[async_trait]
pub trait CdiRateSource: Send + Sync {
    async fn fetch(&self, range: CdiFetchRange) -> Result<Vec<CdiRate>, AppError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CdiCacheState {
    Empty,
    Fresh,
    Stale,
    RefreshFailed,
}

/// Estado mínimo necessário para decidir o incremental e exibir o aviso.
#[derive(Debug, Clone, PartialEq)]
pub struct CdiCacheInfo {
    pub state: CdiCacheState,
    pub last_rate_date: Option<String>,
    pub last_success_at: Option<String>,
}

#[async_trait]
pub trait CdiRatesRepository: Send + Sync {
    async fn info(&self) -> Result<CdiCacheInfo, AppError>;
    async fn status(&self) -> Result<CdiStatusDto, AppError>;
    /// Substitui o intervalo informado e marca sucesso em uma transação.
    async fn replace(
        &self,
        series: &str,
        rates: Vec<CdiRate>,
        fetched_at: &str,
    ) -> Result<(), AppError>;
    /// Registra a tentativa e o erro sem tocar nas taxas já cacheadas.
    async fn record_failure(&self, attempted_at: &str, error: &str) -> Result<(), AppError>;
}
