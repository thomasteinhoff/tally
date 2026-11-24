use rusqlite::params;
use tally_app::BootstrapService;
use tally_infra::{Database, SqliteBootstrapRepository};
use tempfile::TempDir;

const NOW: &str = "2026-10-30T12:00:00Z";
const USER_ASSET: &str = "10000000-0000-7000-8000-000000000001";
const EXPENSE_CATEGORY: &str = "20000000-0000-7000-8000-000000000001";
const SYSTEM_EXPENSE: &str = "00000000-0000-7000-8000-000000000003";

async fn open_db() -> (TempDir, Database) {
    let dir = tempfile::tempdir().expect("temp dir");
    let db = Database::open(dir.path().join("tally.db"))
        .await
        .expect("database should open");
    (dir, db)
}

async fn seed(db: &Database) {
    db.call(|connection| {
        connection.execute(
            "INSERT INTO accounts (
                id, name, class, type, is_system, credit_limit_cents,
                archived_at, created_at, updated_at
             ) VALUES (?1, 'Conta teste', 'asset', 'bank', 0, NULL, NULL, ?2, ?2)",
            params![USER_ASSET, NOW],
        )?;

        connection.execute(
            "INSERT INTO categories (
                id, name, kind, archived_at, created_at, updated_at
             ) VALUES (?1, 'Mercado', 'expense', NULL, ?2, ?2)",
            params![EXPENSE_CATEGORY, NOW],
        )?;

        Ok(())
    })
    .await
    .expect("seed should work");
}

async fn insert_expense(db: &Database, id: &str, time: Option<&str>, manual: Option<&str>) {
    let id = id.to_owned();
    let time = time.map(str::to_owned);
    let manual = manual.map(str::to_owned);

    db.call(move |connection| {
        connection.execute(
            "INSERT INTO journal_entries (
                id, kind, occurred_on, occurred_time, manual_period, description,
                primary_account_id, counterparty_account_id, category_id, payment_method,
                posted_at, created_at, updated_at
             ) VALUES (
                ?1, 'expense', '2026-10-30', ?2, ?3, 'teste',
                ?4, NULL, ?5, 'cash', NULL, ?6, ?6
             )",
            params![id, time, manual, USER_ASSET, EXPENSE_CATEGORY, NOW],
        )?;

        connection.execute(
            "INSERT INTO postings (id, entry_id, account_id, amount_cents, created_at)
             VALUES (?1, ?2, ?3, 100, ?4)",
            params![format!("{id}-expense"), id, SYSTEM_EXPENSE, NOW],
        )?;

        connection.execute(
            "INSERT INTO postings (id, entry_id, account_id, amount_cents, created_at)
             VALUES (?1, ?2, ?3, -100, ?4)",
            params![format!("{id}-asset"), id, USER_ASSET, NOW],
        )?;

        Ok(())
    })
    .await
    .expect("expense should be inserted");
}

async fn post_entry(db: &Database, id: &str) {
    let id = id.to_owned();
    db.call(move |connection| {
        connection.execute(
            "UPDATE journal_entries SET posted_at = ?1 WHERE id = ?2",
            params![NOW, id],
        )?;
        Ok(())
    })
    .await
    .expect("entry should post");
}

async fn period_for(db: &Database, id: &str) -> String {
    let id = id.to_owned();
    db.call(move |connection| {
        connection.query_row(
            "SELECT period FROM journal_entries WHERE id = ?1",
            [id],
            |row| row.get::<_, String>(0),
        )
    })
    .await
    .expect("period should be readable")
}

#[tokio::test]
async fn migrates_and_seeds_system_accounts() {
    let (_dir, db) = open_db().await;

    let schema_version = db
        .call(|connection| {
            connection.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        })
        .await
        .expect("schema version");
    assert_eq!(schema_version, 6);

    let system_accounts = db
        .call(|connection| {
            connection.query_row(
                "SELECT COUNT(*) FROM accounts WHERE is_system = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
        })
        .await
        .expect("system account count");
    assert_eq!(system_accounts, 4);

    let journal_mode = db
        .call(|connection| {
            connection.query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
        })
        .await
        .expect("journal mode");
    assert_eq!(journal_mode.to_lowercase(), "wal");

    let foreign_keys = db
        .call(|connection| {
            connection.query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
        })
        .await
        .expect("foreign keys");
    assert_eq!(foreign_keys, 1);

    let backup_retention = db
        .call(|connection| {
            connection.query_row(
                "SELECT value FROM settings WHERE key = 'backup_retention'",
                [],
                |row| row.get::<_, String>(0),
            )
        })
        .await
        .expect("backup setting");
    assert_eq!(backup_retention, "10");
}

#[tokio::test]
async fn bootstrap_reads_real_database_state() {
    let (_dir, db) = open_db().await;
    let repository = SqliteBootstrapRepository::new(db);
    let service = BootstrapService::new(repository, "0.1.0", "tally.db", "Tally");

    let dto = service.bootstrap().await.expect("bootstrap should work");

    assert_eq!(dto.app_version, "0.1.0");
    assert_eq!(dto.schema_version, 6);
    assert_eq!(dto.system_accounts, 4);
    assert_eq!(dto.settings.backup_retention, 10);
    assert!(!dto.settings.first_run_completed);
    assert!(dto.integrity.ok);
    assert!(dto.integrity.issues.is_empty());
}

#[tokio::test]
async fn derives_period_at_all_boundaries() {
    let (_dir, db) = open_db().await;
    seed(&db).await;

    for (index, (time, expected)) in [
        ("04:59", "madrugada"),
        ("05:00", "manha"),
        ("11:59", "manha"),
        ("12:00", "tarde"),
        ("18:59", "tarde"),
        ("19:00", "noite"),
        ("23:59", "noite"),
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("30000000-0000-7000-8000-{:012}", index + 1);
        insert_expense(&db, &id, Some(time), None).await;
        post_entry(&db, &id).await;
        assert_eq!(period_for(&db, &id).await, expected);
    }
}

#[tokio::test]
async fn manual_period_is_used_when_time_is_absent() {
    let (_dir, db) = open_db().await;
    seed(&db).await;

    let id = "30000000-0000-7000-8000-000000000101";
    insert_expense(&db, id, None, Some("noite")).await;
    post_entry(&db, id).await;
    assert_eq!(period_for(&db, id).await, "noite");

    let custom_id = "30000000-0000-7000-8000-000000000102";
    insert_expense(&db, custom_id, None, Some("custom")).await;
    post_entry(&db, custom_id).await;
    assert_eq!(period_for(&db, custom_id).await, "custom");
}

#[tokio::test]
async fn rejects_time_and_manual_period_together() {
    let (_dir, db) = open_db().await;
    seed(&db).await;

    let result = db
        .call(|connection| {
            connection.execute(
                "INSERT INTO journal_entries (
                    id, kind, occurred_on, occurred_time, manual_period, description,
                    primary_account_id, category_id, payment_method, posted_at, created_at, updated_at
                 ) VALUES (
                    '30000000-0000-7000-8000-000000000201', 'expense', '2026-10-30',
                    '12:00', 'tarde', 'teste', ?1, ?2, 'cash', NULL, ?3, ?3
                 )",
                params![USER_ASSET, EXPENSE_CATEGORY, NOW],
            )
        })
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn rejects_entry_without_time_or_manual_period() {
    let (_dir, db) = open_db().await;
    seed(&db).await;

    let result = db
        .call(|connection| {
            connection.execute(
                "INSERT INTO journal_entries (
                    id, kind, occurred_on, occurred_time, manual_period, description,
                    primary_account_id, category_id, payment_method, posted_at, created_at, updated_at
                 ) VALUES (
                    '30000000-0000-7000-8000-000000000202', 'expense', '2026-10-30',
                    NULL, NULL, 'teste', ?1, ?2, 'cash', NULL, ?3, ?3
                 )",
                params![USER_ASSET, EXPENSE_CATEGORY, NOW],
            )
        })
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn generated_period_recalculates_after_unpost_and_edit() {
    let (_dir, db) = open_db().await;
    seed(&db).await;

    let id = "30000000-0000-7000-8000-000000000301";
    insert_expense(&db, id, Some("04:59"), None).await;
    post_entry(&db, id).await;
    assert_eq!(period_for(&db, id).await, "madrugada");

    let id_for_unpost = id.to_owned();
    db.call(move |connection| {
        connection.execute(
            "UPDATE journal_entries SET posted_at = NULL WHERE id = ?1",
            [id_for_unpost],
        )?;
        Ok(())
    })
    .await
    .expect("entry should unpost");

    let id_for_edit = id.to_owned();
    db.call(move |connection| {
        connection.execute(
            "UPDATE journal_entries SET occurred_time = '12:00' WHERE id = ?1",
            [id_for_edit],
        )?;
        Ok(())
    })
    .await
    .expect("draft should be editable");

    post_entry(&db, id).await;
    assert_eq!(period_for(&db, id).await, "tarde");
}

#[tokio::test]
async fn rejects_unbalanced_entry_on_post() {
    let (_dir, db) = open_db().await;
    seed(&db).await;

    let id = "30000000-0000-7000-8000-000000000401";
    let id_for_insert = id.to_owned();
    db.call(move |connection| {
        connection.execute(
            "INSERT INTO journal_entries (
                id, kind, occurred_on, occurred_time, manual_period, description,
                primary_account_id, category_id, payment_method, posted_at, created_at, updated_at
             ) VALUES (?1, 'expense', '2026-10-30', '12:00', NULL, 'teste',
                ?2, ?3, 'cash', NULL, ?4, ?4)",
            params![id_for_insert, USER_ASSET, EXPENSE_CATEGORY, NOW],
        )?;
        connection.execute(
            "INSERT INTO postings (id, entry_id, account_id, amount_cents, created_at)
             VALUES ('posting-unbalanced', ?1, ?2, 100, ?3)",
            params![id, SYSTEM_EXPENSE, NOW],
        )?;
        Ok(())
    })
    .await
    .expect("draft should be inserted");

    let id_for_post = id.to_owned();
    let result = db
        .call(move |connection| {
            connection.execute(
                "UPDATE journal_entries SET posted_at = ?1 WHERE id = ?2",
                params![NOW, id_for_post],
            )
        })
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn posted_entries_and_postings_are_immutable() {
    let (_dir, db) = open_db().await;
    seed(&db).await;

    let id = "30000000-0000-7000-8000-000000000501";
    insert_expense(&db, id, Some("12:00"), None).await;
    post_entry(&db, id).await;

    let id_for_edit = id.to_owned();
    let edit_result = db
        .call(move |connection| {
            connection.execute(
                "UPDATE journal_entries SET description = 'alterado' WHERE id = ?1",
                [id_for_edit],
            )
        })
        .await;
    assert!(edit_result.is_err());

    let id_for_posting_edit = id.to_owned();
    let posting_result = db
        .call(move |connection| {
            connection.execute(
                "UPDATE postings SET amount_cents = 200 WHERE entry_id = ?1",
                [id_for_posting_edit],
            )
        })
        .await;
    assert!(posting_result.is_err());
}

#[tokio::test]
async fn rejects_entry_inserted_already_posted() {
    let (_dir, db) = open_db().await;
    seed(&db).await;

    let result = db
        .call(|connection| {
            connection.execute(
                "INSERT INTO journal_entries (
                    id, kind, occurred_on, occurred_time, manual_period, description,
                    primary_account_id, category_id, payment_method, posted_at, created_at, updated_at
                 ) VALUES (
                    '30000000-0000-7000-8000-000000000601', 'expense', '2026-10-30',
                    '12:00', NULL, 'teste', ?1, ?2, 'cash', ?3, ?3, ?3
                 )",
                params![USER_ASSET, EXPENSE_CATEGORY, NOW],
            )
        })
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn rejects_foreign_key_violation() {
    let (_dir, db) = open_db().await;

    let result = db
        .call(|connection| {
            connection.execute(
                "INSERT INTO postings (id, entry_id, account_id, amount_cents, created_at)
                 VALUES ('posting-orphan', 'missing-entry', 'missing-account', 1, ?1)",
                [NOW],
            )
        })
        .await;

    assert!(result.is_err());
}
