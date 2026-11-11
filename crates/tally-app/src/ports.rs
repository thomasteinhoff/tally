use async_trait::async_trait;

use crate::dto::{IntegrityReportDto, SettingsDto};
use crate::error::AppError;

#[async_trait]
pub trait BootstrapRepository: Send + Sync {
    async fn schema_version(&self) -> Result<i64, AppError>;
    async fn settings(&self) -> Result<SettingsDto, AppError>;
    async fn update_settings(&self, backup_retention: u32) -> Result<SettingsDto, AppError>;
    async fn system_account_count(&self) -> Result<i64, AppError>;
    async fn integrity_check(&self) -> Result<IntegrityReportDto, AppError>;
}
