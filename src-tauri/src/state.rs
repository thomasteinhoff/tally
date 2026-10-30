use tally_infra::Database;

use crate::paths::AppPaths;

pub struct AppState {
    pub db: Option<Database>,
    pub paths: AppPaths,
    pub app_version: String,
    pub boot_error: Option<String>,
}
