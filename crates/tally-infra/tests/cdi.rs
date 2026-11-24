use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tally_app::{AppError, CdiFetchRange, CdiRate, CdiRateSource, CdiService, CdiStatusDto};
use tally_infra::{Database, SqliteCdiRepository};
use tempfile::TempDir;

#[derive(Clone)]
struct FakeSource {
    calls: Arc<Mutex<Vec<CdiFetchRange>>>,
    fail: bool,
}

#[async_trait]
impl CdiRateSource for FakeSource {
    async fn fetch(&self, range: CdiFetchRange) -> Result<Vec<CdiRate>, AppError> {
        self.calls.lock().expect("lock").push(range);
        if self.fail {
            return Err(AppError::io("sem rede", "offline"));
        }
        Ok(vec![CdiRate {
            rate_date: "2026-11-19".to_owned(),
            raw_value: "0.050788".to_owned(),
            daily_rate: 0.00050788,
        }])
    }
}

async fn fixture() -> (TempDir, Database, FakeSource) {
    let dir = tempfile::tempdir().expect("temp dir");
    let db = Database::open(dir.path().join("tally.db"))
        .await
        .expect("database");
    let source = FakeSource {
        calls: Arc::new(Mutex::new(Vec::new())),
        fail: false,
    };
    (dir, db, source)
}

fn service(db: Database, source: FakeSource) -> CdiService<SqliteCdiRepository, FakeSource> {
    CdiService::new(SqliteCdiRepository::new(db), source)
}

#[tokio::test]
async fn refresh_is_incremental_and_substitutes_cache_atomically() {
    let (_dir, db, source) = fixture().await;
    let service = service(db, source.clone());

    let first: CdiStatusDto = service
        .refresh(Some("2026-01-01"))
        .await
        .expect("primeiro refresh");
    assert_eq!(first.rate_count, 1);
    assert_eq!(first.last_rate_date.as_deref(), Some("2026-11-19"));

    let second = service
        .refresh(Some("2026-01-01"))
        .await
        .expect("segundo refresh");
    assert_eq!(second.rate_count, 1);
    let calls = source.calls.lock().expect("lock").clone();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].from, "2026-01-01");
    assert_eq!(calls[1].from, "2026-11-12");
}

#[tokio::test]
async fn failed_refresh_never_erases_existing_rates() {
    let (_dir, db, source) = fixture().await;
    let service = service(db.clone(), source);
    service.refresh(Some("2026-01-01")).await.expect("seed");

    let failing = FakeSource {
        calls: Arc::new(Mutex::new(Vec::new())),
        fail: true,
    };
    let failing_service = CdiService::new(SqliteCdiRepository::new(db), failing);
    assert!(failing_service.refresh(None).await.is_err());

    let status = failing_service.status().await.expect("status");
    assert_eq!(status.rate_count, 1);
    assert!(status.last_error.is_some());
    assert!(status.last_success_at.is_some());
}
