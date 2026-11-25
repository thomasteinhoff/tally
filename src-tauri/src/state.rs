use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tally_infra::Database;
use tauri::async_runtime::Mutex;

use crate::paths::AppPaths;

pub struct AppState {
    pub db: Option<Database>,
    pub paths: AppPaths,
    pub app_version: String,
    pub boot_error: Option<String>,
    pub auto_refresh_started: AtomicBool,
    pub refresh_lock: Arc<Mutex<()>>,
}

impl AppState {
    pub fn claim_auto_refresh(&self) -> bool {
        self.db.is_some() && !self.auto_refresh_started.swap(true, Ordering::Relaxed)
    }
}
