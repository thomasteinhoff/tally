use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct SettingsDto {
    pub backup_retention: u32,
    pub first_run_completed: bool,
    pub last_backup_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct IntegrityReportDto {
    pub ok: bool,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct BootstrapDto {
    pub app_version: String,
    pub db_path: String,
    pub data_dir: String,
    #[ts(type = "number")]
    pub schema_version: i64,
    pub settings: SettingsDto,
    #[ts(type = "number")]
    pub system_accounts: i64,
    pub integrity: IntegrityReportDto,
}
