use std::time::Duration;

use rusqlite::Connection;
use tally_app::{
    AccountsService, AppError, BootstrapService, CategoriesService, CreateAccountInput,
    CreateCategoryInput, CreateExpenseInput, MovementFilter, MovementsService, UpdateSettingsInput,
};
use tally_infra::{
    DataTools, Database, SqliteAccountsRepository, SqliteBootstrapRepository,
    SqliteCategoriesRepository, SqliteMovementsRepository,
};
use tempfile::TempDir;

struct Fixture {
    _dir: TempDir,
    db: Database,
    backups: std::path::PathBuf,
    tools: DataTools,
    accounts: AccountsService<SqliteAccountsRepository>,
    categories: CategoriesService<SqliteCategoriesRepository>,
    movements: MovementsService<SqliteMovementsRepository>,
    bootstrap: BootstrapService<SqliteBootstrapRepository>,
}

async fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("temp dir");
    let db = Database::open(dir.path().join("tally.db"))
        .await
        .expect("database should open");
    let backups = dir.path().join("backups");
    let exports = dir.path().join("exports");

    Fixture {
        tools: DataTools::new(db.clone(), backups.clone(), exports),
        accounts: AccountsService::new(SqliteAccountsRepository::new(db.clone())),
        categories: CategoriesService::new(SqliteCategoriesRepository::new(db.clone())),
        movements: MovementsService::new(SqliteMovementsRepository::new(db.clone())),
        bootstrap: BootstrapService::new(
            SqliteBootstrapRepository::new(db.clone()),
            "test",
            dir.path().join("tally.db").display().to_string(),
            dir.path().display().to_string(),
        ),
        db,
        backups,
        _dir: dir,
    }
}

async fn count(fixture: &Fixture, table: &'static str) -> i64 {
    fixture
        .db
        .call(move |connection| {
            connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
        })
        .await
        .expect("count")
}

async fn seed_expense(fixture: &Fixture) {
    let account = fixture
        .accounts
        .create(CreateAccountInput {
            name: "Banco".to_owned(),
            account_type: "bank".to_owned(),
            opening_balance_cents: 0,
            credit_limit_cents: None,
            is_savings: None,
            yield_mode: None,
            cdi_percent: None,
        })
        .await
        .expect("account")
        .id;
    let category = fixture
        .categories
        .create(CreateCategoryInput {
            name: "Mercado".to_owned(),
            kind: "expense".to_owned(),
            icon: String::new(),
        })
        .await
        .expect("category")
        .id;

    fixture
        .movements
        .create_expense(CreateExpenseInput {
            account_id: account,
            category_id: category,
            amount_cents: 1_234,
            occurred_on: "2026-02-03".to_owned(),
            occurred_time: Some("14:00".to_owned()),
            manual_period: None,
            description: Some("Mercado; teste".to_owned()),
            payment_method: "pix".to_owned(),
        })
        .await
        .expect("expense");
}

#[tokio::test]
async fn creates_lists_and_prunes_backups_by_retention() {
    let fixture = fixture().await;
    fixture
        .bootstrap
        .update_settings(UpdateSettingsInput {
            backup_retention: 2,
        })
        .await
        .expect("settings");

    let mut created = Vec::new();
    for _ in 0..3 {
        created.push(
            fixture
                .tools
                .create_backup()
                .await
                .expect("backup should be created"),
        );
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    assert!(fixture.backups.is_dir());
    assert_eq!(created.len(), 3);
    assert!(created[0].file_name.starts_with("tally-backup-"));

    let listed = fixture.tools.list_backups().expect("list");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].file_name, created[2].file_name);
    assert_eq!(listed[1].file_name, created[1].file_name);

    let settings = fixture.bootstrap.settings().await.expect("settings");
    assert_eq!(settings.backup_retention, 2);
    assert!(settings.last_backup_at.is_some());
}

#[tokio::test]
async fn pruning_never_removes_pre_migration_backups() {
    let fixture = fixture().await;
    fixture
        .bootstrap
        .update_settings(UpdateSettingsInput {
            backup_retention: 1,
        })
        .await
        .expect("settings");

    std::fs::create_dir_all(&fixture.backups).expect("backups dir");
    let legacy = fixture
        .backups
        .join("tally-pre-migration-20260101-120000-000.db");
    std::fs::write(&legacy, b"legacy").expect("legacy");

    fixture.tools.create_backup().await.expect("first backup");
    fixture.tools.create_backup().await.expect("second backup");

    assert!(legacy.exists(), "pre-migration backup must survive pruning");

    let regular = fixture
        .tools
        .list_backups()
        .expect("list")
        .into_iter()
        .filter(|backup| backup.file_name.starts_with("tally-backup-"))
        .count();
    assert_eq!(regular, 1);
}

#[tokio::test]
async fn restore_replaces_the_live_database_and_keeps_integrity() {
    let fixture = fixture().await;
    let baseline = count(&fixture, "categories").await;
    let backup = fixture.tools.create_backup().await.expect("backup");

    fixture
        .categories
        .create(CreateCategoryInput {
            name: "Depois do backup".to_owned(),
            kind: "expense".to_owned(),
            icon: String::new(),
        })
        .await
        .expect("category");
    assert_eq!(count(&fixture, "categories").await, baseline + 1);

    let restored = fixture
        .tools
        .restore_backup(&backup.file_name)
        .await
        .expect("restore");

    assert!(restored.integrity.ok);
    assert_eq!(restored.backup.file_name, backup.file_name);
    assert_eq!(count(&fixture, "categories").await, baseline);
}

#[tokio::test]
async fn restore_rejects_invalid_and_missing_backup_names() {
    let fixture = fixture().await;

    assert!(matches!(
        fixture.tools.restore_backup("").await,
        Err(AppError::Validation { .. })
    ));
    assert!(matches!(
        fixture.tools.restore_backup("../tally.db").await,
        Err(AppError::Validation { .. })
    ));
    assert!(matches!(
        fixture.tools.restore_backup("outro.db").await,
        Err(AppError::Validation { .. })
    ));
    assert!(matches!(
        fixture
            .tools
            .restore_backup("tally-backup-nao-existe.db")
            .await,
        Err(AppError::NotFound { .. })
    ));
}

#[tokio::test]
async fn restore_creates_a_pre_restore_snapshot_and_pruning_preserves_it() {
    let fixture = fixture().await;
    let baseline = count(&fixture, "categories").await;
    let backup = fixture.tools.create_backup().await.expect("backup");

    fixture
        .categories
        .create(CreateCategoryInput {
            name: "Depois do backup".to_owned(),
            kind: "expense".to_owned(),
            icon: String::new(),
        })
        .await
        .expect("category");
    assert_eq!(count(&fixture, "categories").await, baseline + 1);

    fixture
        .tools
        .restore_backup(&backup.file_name)
        .await
        .expect("restore");

    let snapshot = std::fs::read_dir(&fixture.backups)
        .expect("backups")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .find(|name| name.starts_with("tally-pre-restore-"))
        .expect("pre-restore snapshot");

    let connection = Connection::open(fixture.backups.join(&snapshot)).expect("snapshot db");
    let snapshot_categories: i64 = connection
        .query_row("SELECT COUNT(*) FROM categories", [], |row| row.get(0))
        .expect("snapshot count");
    assert_eq!(snapshot_categories, baseline + 1);

    fixture
        .bootstrap
        .update_settings(UpdateSettingsInput {
            backup_retention: 1,
        })
        .await
        .expect("settings");
    fixture.tools.create_backup().await.expect("first backup");
    fixture.tools.create_backup().await.expect("second backup");

    assert!(
        fixture.backups.join(&snapshot).exists(),
        "pre-restore snapshot must survive pruning"
    );
}

#[tokio::test]
async fn restore_rejects_corrupt_or_foreign_files_without_touching_the_database() {
    let fixture = fixture().await;
    let baseline = count(&fixture, "categories").await;
    std::fs::create_dir_all(&fixture.backups).expect("backups dir");

    std::fs::write(
        fixture.backups.join("tally-backup-corrupt.db"),
        b"isto nao e um banco sqlite",
    )
    .expect("corrupt backup");

    let corrupt = fixture
        .tools
        .restore_backup("tally-backup-corrupt.db")
        .await;
    assert!(matches!(corrupt, Err(AppError::Integrity { .. })));

    let foreign = fixture.backups.join("tally-backup-foreign.db");
    {
        let connection = Connection::open(&foreign).expect("foreign db");
        connection
            .execute_batch(include_str!("../migrations/0001_init.sql"))
            .expect("schema");
        connection
            .execute_batch(
                "PRAGMA user_version = 1;
                 PRAGMA foreign_keys = OFF;
                 INSERT INTO postings (id, entry_id, account_id, amount_cents, created_at)
                 VALUES ('posting', 'entrada-inexistente', 'conta-inexistente', 100, '2026-01-01');",
            )
            .expect("broken reference");
    }

    let broken = fixture
        .tools
        .restore_backup("tally-backup-foreign.db")
        .await;
    assert!(matches!(broken, Err(AppError::Integrity { .. })));
    assert_eq!(count(&fixture, "categories").await, baseline);
}

#[tokio::test]
async fn restores_a_legacy_backup_and_applies_pending_migrations() {
    let fixture = fixture().await;
    std::fs::create_dir_all(&fixture.backups).expect("backups dir");
    let legacy = fixture.backups.join("tally-backup-legacy.db");

    {
        let connection = Connection::open(&legacy).expect("legacy db");
        connection
            .execute_batch(include_str!("../migrations/0001_init.sql"))
            .expect("schema v1");
        connection
            .execute_batch("PRAGMA user_version = 1;")
            .expect("version");
    }

    let restored = fixture
        .tools
        .restore_backup("tally-backup-legacy.db")
        .await
        .expect("restore");

    assert!(restored.integrity.ok);

    let version: i64 = fixture
        .db
        .call(|connection| connection.query_row("PRAGMA user_version", [], |row| row.get(0)))
        .await
        .expect("version");
    assert_eq!(version, 7);

    let has_column: i64 = fixture
        .db
        .call(|connection| {
            connection.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('card_payments')
                 WHERE name = 'initial_balance_cents'",
                [],
                |row| row.get(0),
            )
        })
        .await
        .expect("column");
    assert_eq!(has_column, 1);

    let has_icon: i64 = fixture
        .db
        .call(|connection| {
            connection.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('categories')
                 WHERE name = 'icon'",
                [],
                |row| row.get(0),
            )
        })
        .await
        .expect("icon column");
    assert_eq!(has_icon, 1);
}

#[tokio::test]
async fn csv_export_respects_the_full_movement_filter() {
    let fixture = fixture().await;
    seed_expense(&fixture).await;

    let filtered = fixture
        .movements
        .list(Some(MovementFilter {
            account_id: None,
            category_id: None,
            kind: Some("expense".to_owned()),
            period: None,
            payment_method: Some("cash".to_owned()),
            from: None,
            to: None,
        }))
        .await
        .expect("filtered list");
    let export = fixture
        .tools
        .export_movements(&filtered)
        .expect("filtered export");

    assert_eq!(export.row_count, 0);
    let content = std::fs::read_to_string(&export.path).expect("csv");
    assert!(content.starts_with('\u{feff}'));
    assert_eq!(content.lines().count(), 1);
}

#[tokio::test]
async fn exports_movements_to_a_utf8_bom_csv_file() {
    let fixture = fixture().await;
    seed_expense(&fixture).await;

    let movements = fixture
        .movements
        .list(None)
        .await
        .expect("movements should list");
    let export = fixture.tools.export_movements(&movements).expect("export");

    assert_eq!(export.row_count, 1);
    assert!(export.file_name.starts_with("tally-movements-"));
    assert!(export.file_name.ends_with(".csv"));

    let content = std::fs::read_to_string(&export.path).expect("csv");
    assert!(content.starts_with('\u{feff}'));
    assert!(content.contains("Data;Hora;Periodo;Tipo;Descricao"));
    assert!(content.contains("\"Mercado; teste\""));
    assert!(content.contains("R$ 12,34"));
    assert!(content.ends_with("\r\n"));
}
