use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{Duration, Local};
use rusqlite::params;
use tally_app::cdi::earliest_sgs_date;
use tally_app::{AppError, CdiFetchRange, CdiRate, CdiRateSource, CdiService, CdiStatusDto};
use tally_infra::{Database, SqliteCdiRepository};
use tempfile::TempDir;

#[derive(Clone)]
struct FakeSource {
    calls: Arc<Mutex<Vec<CdiFetchRange>>>,
    error: Option<AppError>,
}

#[async_trait]
impl CdiRateSource for FakeSource {
    async fn fetch(&self, range: CdiFetchRange) -> Result<Vec<CdiRate>, AppError> {
        self.calls.lock().expect("lock").push(range.clone());
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        Ok(vec![CdiRate {
            rate_date: range.to,
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
        error: None,
    };
    (dir, db, source)
}

fn service(db: Database, source: FakeSource) -> CdiService<SqliteCdiRepository, FakeSource> {
    CdiService::new(SqliteCdiRepository::new(db), source)
}

fn failing_source(error: AppError) -> FakeSource {
    FakeSource {
        calls: Arc::new(Mutex::new(Vec::new())),
        error: Some(error),
    }
}

#[tokio::test]
async fn refresh_is_incremental_and_substitutes_cache_atomically() {
    let (_dir, db, source) = fixture().await;
    let today = Local::now().date_naive();
    let today_iso = today.format("%Y-%m-%d").to_string();
    let service = service(db, source.clone());

    let first: CdiStatusDto = service
        .refresh(Some("2026-01-01"))
        .await
        .expect("primeiro refresh");
    assert_eq!(first.rate_count, 1);
    assert_eq!(first.last_rate_date.as_deref(), Some(today_iso.as_str()));

    let second = service
        .refresh(Some("2026-01-01"))
        .await
        .expect("segundo refresh");
    assert_eq!(second.rate_count, 1);
    let calls = source.calls.lock().expect("lock").clone();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].from, "2026-01-01");
    assert_eq!(
        calls[1].from,
        (today - Duration::days(7)).format("%Y-%m-%d").to_string()
    );
}

#[tokio::test]
async fn empty_cache_defaults_to_ten_years() {
    let (_dir, db, source) = fixture().await;
    let today = Local::now().date_naive();
    let expected_from = earliest_sgs_date(today).format("%Y-%m-%d").to_string();
    let service = service(db, source.clone());

    service.refresh(None).await.expect("refresh");

    let calls = source.calls.lock().expect("lock").clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].from, expected_from);
    assert_eq!(calls[0].to, today.format("%Y-%m-%d").to_string());
}

#[tokio::test]
async fn explicit_date_older_than_ten_years_is_rejected_without_request() {
    let (_dir, db, source) = fixture().await;
    let old = earliest_sgs_date(Local::now().date_naive()) - Duration::days(1);
    let old_iso = old.format("%Y-%m-%d").to_string();
    let service = service(db, source.clone());

    let error = service
        .refresh(Some(&old_iso))
        .await
        .expect_err("data antiga deve falhar");

    assert_eq!(error.code(), "VALIDATION");
    assert!(error.message().contains("10 anos"), "{}", error.message());
    assert!(source.calls.lock().expect("lock").is_empty());
}

#[tokio::test]
async fn old_cached_date_is_clamped_to_ten_years() {
    let (_dir, db, source) = fixture().await;
    let today = Local::now().date_naive();
    let old = earliest_sgs_date(today) - Duration::days(30);
    let old_iso = old.format("%Y-%m-%d").to_string();

    db.call(move |connection| {
        connection.execute(
            "INSERT INTO cdi_rates (series, rate_date, raw_value, daily_rate, fetched_at)
             VALUES ('12', ?1, '0.050788', 0.00050788, '2026-01-01T00:00:00+00:00')",
            params![old_iso],
        )?;
        connection.execute(
            "UPDATE cdi_sync_status SET first_rate_date = ?1, last_rate_date = ?1 WHERE singleton = 1",
            params![old_iso],
        )?;
        Ok(())
    })
    .await
    .expect("seed");
    let service = service(db, source.clone());

    service.refresh(None).await.expect("refresh");

    let calls = source.calls.lock().expect("lock").clone();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].from,
        earliest_sgs_date(today).format("%Y-%m-%d").to_string()
    );
}

#[tokio::test]
async fn failed_refresh_never_erases_existing_rates() {
    let (_dir, db, source) = fixture().await;
    let service = service(db.clone(), source);
    service.refresh(None).await.expect("seed");

    let failing = failing_source(AppError::io("sem rede", "offline"));
    let failing_service = CdiService::new(SqliteCdiRepository::new(db), failing);
    assert!(failing_service.refresh(None).await.is_err());

    let status = failing_service.status().await.expect("status");
    assert_eq!(status.rate_count, 1);
    assert!(status.last_error.is_some());
    assert!(status.last_success_at.is_some());
}

#[tokio::test]
async fn connection_failure_keeps_its_own_message_and_cache() {
    let (_dir, db, source) = fixture().await;
    let service = service(db.clone(), source);
    service.refresh(None).await.expect("seed");

    let failing = failing_source(AppError::io(
        "não foi possível consultar as taxas CDI",
        "dns falhou",
    ));
    let failing_service = CdiService::new(SqliteCdiRepository::new(db), failing);
    let error = failing_service
        .refresh(None)
        .await
        .expect_err("falha de conexão");

    assert_eq!(error.message(), "não foi possível consultar as taxas CDI");
    let status = failing_service.status().await.expect("status");
    assert_eq!(status.rate_count, 1);
    assert!(
        status
            .last_error
            .expect("erro persistido")
            .contains("dns falhou")
    );
}

#[tokio::test]
async fn http_406_details_are_persisted() {
    let (_dir, db, source) = fixture().await;
    let service = service(db.clone(), source);
    service.refresh(None).await.expect("seed");

    let failing = failing_source(AppError::io(
        "o SGS recusou a consulta",
        "HTTP 406 Not Acceptable (json): {\"error\":\"janela de, no máximo, 10 anos\"}",
    ));
    let failing_service = CdiService::new(SqliteCdiRepository::new(db), failing);
    assert!(failing_service.refresh(None).await.is_err());

    let last_error = failing_service
        .status()
        .await
        .expect("status")
        .last_error
        .expect("erro persistido");
    assert!(last_error.contains("o SGS recusou a consulta"));
    assert!(last_error.contains("Detalhes: HTTP 406 Not Acceptable"));
    assert!(last_error.contains("10 anos"));
}

#[tokio::test]
async fn http_304_is_not_reported_as_generic_connection_failure() {
    let (_dir, db, source) = fixture().await;
    let service = service(db.clone(), source);
    service.refresh(None).await.expect("seed");

    let failing = failing_source(AppError::io(
        "o SGS devolveu 304 sem corpo utilizável",
        "HTTP 304 Not Modified nas representações JSON e CSV",
    ));
    let failing_service = CdiService::new(SqliteCdiRepository::new(db), failing);
    let error = failing_service
        .refresh(None)
        .await
        .expect_err("304 sem fallback deve falhar");

    assert!(error.message().contains("304"));
    assert!(!error.message().contains("não respondeu"));
    let status = failing_service.status().await.expect("status");
    assert_eq!(status.rate_count, 1);
    assert!(status.last_error.expect("erro persistido").contains("304"));
}
