use rusqlite::params;
use tally_app::{
    AccountsRepository, AccountsService, AppError, CreateAccountInput, NewAccount, OpeningBalance,
};
use tally_domain::account::AccountType;
use tally_domain::ledger::Posting;
use tally_domain::money::Money;
use tally_infra::{Database, SqliteAccountsRepository};
use tempfile::TempDir;

const NOW: &str = "2026-10-31T12:00:00Z";
const SYSTEM_EXPENSE: &str = "00000000-0000-7000-8000-000000000003";

async fn open_db() -> (TempDir, Database) {
    let dir = tempfile::tempdir().expect("temp dir");
    let db = Database::open(dir.path().join("tally.db"))
        .await
        .expect("database should open");
    (dir, db)
}

fn service(db: &Database) -> AccountsService<SqliteAccountsRepository> {
    AccountsService::new(SqliteAccountsRepository::new(db.clone()))
}

fn input(name: &str, account_type: &str, opening_balance_cents: i64) -> CreateAccountInput {
    CreateAccountInput {
        name: name.to_owned(),
        account_type: account_type.to_owned(),
        opening_balance_cents,
        credit_limit_cents: None,
    }
}

async fn count(db: &Database, sql: &'static str) -> i64 {
    db.call(move |connection| connection.query_row(sql, [], |row| row.get::<_, i64>(0)))
        .await
        .expect("count should work")
}

async fn seed_expense(db: &Database, account_id: &str, cents: i64, posted: bool, suffix: &str) {
    let account_id = account_id.to_owned();
    let suffix = suffix.to_owned();

    db.call(move |connection| {
        let entry_id = format!("entry-expense-{suffix}");
        let category_id = format!("category-{suffix}");
        let out_posting_id = format!("posting-out-{suffix}");
        let in_posting_id = format!("posting-in-{suffix}");
        let posted_at = posted.then_some(NOW);

        connection.execute(
            "INSERT INTO categories (id, name, kind, archived_at, created_at, updated_at)
             VALUES (?1, ?2, 'expense', NULL, ?3, ?3)",
            params![category_id, format!("Mercado {suffix}"), NOW],
        )?;

        connection.execute(
            "INSERT INTO journal_entries (
                id, kind, occurred_on, occurred_time, manual_period, description,
                primary_account_id, counterparty_account_id, category_id, payment_method,
                posted_at, created_at, updated_at
             ) VALUES (
                ?1, 'expense', '2026-10-30', '12:00', NULL, 'teste',
                ?2, NULL, ?3, 'cash', NULL, ?4, ?4
             )",
            params![entry_id, account_id, category_id, NOW],
        )?;

        connection.execute(
            "INSERT INTO postings (id, entry_id, account_id, amount_cents, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![out_posting_id, entry_id, account_id, -cents, NOW],
        )?;

        connection.execute(
            "INSERT INTO postings (id, entry_id, account_id, amount_cents, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![in_posting_id, entry_id, SYSTEM_EXPENSE, cents, NOW],
        )?;

        if let Some(posted_at) = posted_at {
            connection.execute(
                "UPDATE journal_entries SET posted_at = ?1 WHERE id = ?2",
                params![posted_at, entry_id],
            )?;
        }

        Ok(())
    })
    .await
    .expect("expense seed should work");
}

#[tokio::test]
async fn create_posts_balanced_opening_balance_and_returns_derived_balance() {
    let (_dir, db) = open_db().await;
    let service = service(&db);

    let account = service
        .create(input("Banco", "bank", 100_000))
        .await
        .expect("create should work");

    assert_eq!(account.name, "Banco");
    assert_eq!(account.account_type, "bank");
    assert_eq!(account.balance_cents, 100_000);
    assert!(account.archived_at.is_none());

    let (postings, total): (i64, i64) = db
        .call(|connection| {
            let postings =
                connection.query_row("SELECT COUNT(*) FROM postings", [], |row| row.get(0))?;
            let total = connection.query_row(
                "SELECT COALESCE(SUM(amount_cents), 0) FROM postings",
                [],
                |row| row.get(0),
            )?;
            Ok((postings, total))
        })
        .await
        .expect("ledger should be readable");

    assert_eq!(postings, 2);
    assert_eq!(total, 0);

    let listed = service.list(false).await.expect("list should work");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, account.id);
    assert_eq!(listed[0].balance_cents, 100_000);
}

#[tokio::test]
async fn zero_opening_balance_creates_no_journal_entry() {
    let (_dir, db) = open_db().await;
    let service = service(&db);

    let account = service
        .create(input("Carteira", "cash", 0))
        .await
        .expect("create should work");

    assert_eq!(account.balance_cents, 0);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM journal_entries").await, 0);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM postings").await, 0);
}

#[tokio::test]
async fn balance_is_derived_from_posted_postings_only() {
    let (_dir, db) = open_db().await;
    let service = service(&db);

    let account = service
        .create(input("Banco", "bank", 100_000))
        .await
        .expect("create should work");

    seed_expense(&db, &account.id, 25_000, true, "posted").await;
    seed_expense(&db, &account.id, 10_000, false, "draft").await;

    let listed = service.list(false).await.expect("list should work");
    assert_eq!(listed[0].balance_cents, 75_000);
}

#[tokio::test]
async fn rename_preserves_id_and_history() {
    let (_dir, db) = open_db().await;
    let service = service(&db);

    let account = service
        .create(input("Banco", "bank", 100_000))
        .await
        .expect("create should work");

    let renamed = service
        .update(&account.id, "  Banco novo  ", None)
        .await
        .expect("rename should work");

    assert_eq!(renamed.id, account.id);
    assert_eq!(renamed.name, "Banco novo");
    assert_eq!(renamed.balance_cents, 100_000);
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM postings").await,
        2,
        "renomear não pode tocar no histórico"
    );
}

#[tokio::test]
async fn archive_hides_from_active_list_and_keeps_history() {
    let (_dir, db) = open_db().await;
    let service = service(&db);

    let account = service
        .create(input("Poupança", "savings", 50_000))
        .await
        .expect("create should work");

    let archived = service
        .set_archived(&account.id, true)
        .await
        .expect("archive should work");
    assert!(archived.archived_at.is_some());
    assert!(service.list(false).await.expect("list").is_empty());

    let all = service.list(true).await.expect("list all");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].balance_cents, 50_000);

    let restored = service
        .set_archived(&account.id, false)
        .await
        .expect("unarchive should work");
    assert!(restored.archived_at.is_none());
    assert_eq!(service.list(false).await.expect("list").len(), 1);
}

#[tokio::test]
async fn delete_removes_fresh_account_and_its_opening_balance() {
    let (_dir, db) = open_db().await;
    let service = service(&db);

    let account = service
        .create(input("Banco", "bank", 100_000))
        .await
        .expect("create should work");

    service
        .delete(&account.id)
        .await
        .expect("delete should work");

    assert!(service.list(true).await.expect("list").is_empty());
    assert_eq!(count(&db, "SELECT COUNT(*) FROM journal_entries").await, 0);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM postings").await, 0);
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM accounts WHERE is_system = 0").await,
        0
    );
}

#[tokio::test]
async fn delete_with_movements_is_a_conflict() {
    let (_dir, db) = open_db().await;
    let service = service(&db);

    let account = service
        .create(input("Banco", "bank", 100_000))
        .await
        .expect("create should work");
    seed_expense(&db, &account.id, 1_000, true, "history").await;

    let result = service.delete(&account.id).await;
    assert!(matches!(result, Err(AppError::Conflict { .. })));

    let listed = service.list(false).await.expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].balance_cents, 99_000);
}

#[tokio::test]
async fn duplicate_name_is_a_conflict_and_rolls_back() {
    let (_dir, db) = open_db().await;
    let service = service(&db);

    service
        .create(input("Banco", "bank", 100_000))
        .await
        .expect("first create should work");

    let result = service.create(input("banco", "bank", 200_000)).await;
    assert!(matches!(result, Err(AppError::Conflict { .. })));

    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM accounts WHERE is_system = 0").await,
        1
    );
    assert_eq!(count(&db, "SELECT COUNT(*) FROM journal_entries").await, 1);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM postings").await, 2);

    let listed = service.list(false).await.expect("list");
    assert_eq!(listed[0].balance_cents, 100_000);
}

#[tokio::test]
async fn unbalanced_opening_entry_rolls_back_the_account() {
    let (_dir, db) = open_db().await;
    let repository = SqliteAccountsRepository::new(db.clone());
    let account_id = "10000000-0000-7000-8000-000000000001";

    let result = repository
        .create(NewAccount {
            id: account_id.to_owned(),
            name: "Conta inválida".to_owned(),
            account_type: AccountType::Bank,
            credit_limit_cents: None,
            opening_entry: Some(OpeningBalance {
                entry_id: "30000000-0000-7000-8000-000000000001".to_owned(),
                occurred_on: "2026-10-31".to_owned(),
                postings: [
                    Posting::new("posting-account", account_id, Money::from_cents(100)),
                    Posting::new(
                        "posting-equity",
                        "00000000-0000-7000-8000-000000000001",
                        Money::from_cents(-90),
                    ),
                ],
            }),
        })
        .await;

    assert!(result.is_err());
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM accounts WHERE is_system = 0").await,
        0
    );
    assert_eq!(count(&db, "SELECT COUNT(*) FROM journal_entries").await, 0);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM postings").await, 0);
}
