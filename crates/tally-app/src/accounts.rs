use async_trait::async_trait;
use chrono::Local;
use tally_domain::account::AccountType;
use tally_domain::ledger::{Posting, validate_postings};
use tally_domain::money::Money;
use uuid::Uuid;

use crate::dto::{AccountDto, CreateAccountInput};
use crate::error::AppError;

/// Conta interna de patrimônio criada pela migration inicial.
const EQUITY_ACCOUNT_ID: &str = "00000000-0000-7000-8000-000000000001";

/// Limite de centavos representável sem perda no `number` do JavaScript.
const MAX_SAFE_CENTS: u64 = 9_007_199_254_740_991;

const MAX_NAME_CHARS: usize = 120;

#[derive(Clone)]
pub struct OpeningBalance {
    pub entry_id: String,
    pub occurred_on: String,
    pub postings: [Posting; 2],
}

#[derive(Clone)]
pub struct NewAccount {
    pub id: String,
    pub name: String,
    pub account_type: AccountType,
    pub credit_limit_cents: Option<i64>,
    pub opening_entry: Option<OpeningBalance>,
}

#[async_trait]
pub trait AccountsRepository: Send + Sync {
    async fn list(&self, include_archived: bool) -> Result<Vec<AccountDto>, AppError>;
    async fn create(&self, account: NewAccount) -> Result<AccountDto, AppError>;
    async fn rename(&self, id: &str, name: &str) -> Result<AccountDto, AppError>;
    async fn set_archived(&self, id: &str, archived: bool) -> Result<AccountDto, AppError>;
    async fn delete(&self, id: &str) -> Result<(), AppError>;
}

pub struct AccountsService<R> {
    repo: R,
}

impl<R: AccountsRepository> AccountsService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn list(&self, include_archived: bool) -> Result<Vec<AccountDto>, AppError> {
        self.repo.list(include_archived).await
    }

    pub async fn create(&self, input: CreateAccountInput) -> Result<AccountDto, AppError> {
        let name = normalize_name(&input.name)?;
        let account_type = AccountType::parse(&input.account_type).ok_or_else(|| {
            AppError::validation(format!("tipo de conta inválido: {}", input.account_type))
        })?;
        let cents = validate_cents(input.opening_balance_cents)?;
        if account_type.is_credit_card() && cents < 0 {
            return Err(AppError::validation(
                "a dívida inicial do cartão não pode ser negativa",
            ));
        }
        let credit_limit_cents = normalize_credit_limit(account_type, input.credit_limit_cents)?;
        let account_id = Uuid::now_v7().to_string();

        let opening_entry = if cents == 0 {
            None
        } else {
            // Cartão é passivo: dívida inicial positiva reduz o patrimônio.
            let (account_cents, equity_cents) = if account_type.is_credit_card() {
                let account_cents = cents
                    .checked_neg()
                    .ok_or_else(|| AppError::validation("dívida inicial fora do limite"))?;
                (account_cents, cents)
            } else {
                let equity_cents = cents
                    .checked_neg()
                    .ok_or_else(|| AppError::validation("saldo inicial fora do limite"))?;
                (cents, equity_cents)
            };

            let postings = [
                Posting::new(
                    Uuid::now_v7().to_string(),
                    account_id.clone(),
                    Money::from_cents(account_cents),
                ),
                Posting::new(
                    Uuid::now_v7().to_string(),
                    EQUITY_ACCOUNT_ID,
                    Money::from_cents(equity_cents),
                ),
            ];
            validate_postings(&postings).map_err(AppError::validation)?;

            Some(OpeningBalance {
                entry_id: Uuid::now_v7().to_string(),
                occurred_on: Local::now().date_naive().format("%Y-%m-%d").to_string(),
                postings,
            })
        };

        self.repo
            .create(NewAccount {
                id: account_id,
                name,
                account_type,
                credit_limit_cents,
                opening_entry,
            })
            .await
    }

    pub async fn rename(&self, id: &str, name: &str) -> Result<AccountDto, AppError> {
        let name = normalize_name(name)?;
        self.repo.rename(id, &name).await
    }

    pub async fn set_archived(&self, id: &str, archived: bool) -> Result<AccountDto, AppError> {
        self.repo.set_archived(id, archived).await
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        self.repo.delete(id).await
    }
}

fn normalize_name(raw: &str) -> Result<String, AppError> {
    let name = raw.trim();

    if name.is_empty() {
        return Err(AppError::validation("informe o nome da conta"));
    }

    if name.chars().count() > MAX_NAME_CHARS {
        return Err(AppError::validation(format!(
            "o nome da conta deve ter no máximo {MAX_NAME_CHARS} caracteres"
        )));
    }

    Ok(name.to_owned())
}

fn validate_cents(cents: i64) -> Result<i64, AppError> {
    if cents.unsigned_abs() > MAX_SAFE_CENTS {
        return Err(AppError::validation(
            "o saldo inicial excede o limite seguro de centavos",
        ));
    }

    Ok(cents)
}

fn normalize_credit_limit(
    account_type: AccountType,
    limit_cents: Option<i64>,
) -> Result<Option<i64>, AppError> {
    if !account_type.is_credit_card() {
        if limit_cents.is_some() {
            return Err(AppError::validation(
                "limite de crédito só existe em cartão de crédito",
            ));
        }
        return Ok(None);
    }

    let Some(limit_cents) = limit_cents else {
        return Ok(None);
    };

    if limit_cents < 0 || limit_cents.unsigned_abs() > MAX_SAFE_CENTS {
        return Err(AppError::validation("informe um limite de crédito válido"));
    }

    Ok(Some(limit_cents))
}

#[cfg(test)]
mod tests {
    use super::{AccountsService, EQUITY_ACCOUNT_ID, NewAccount};
    use crate::accounts::AccountsRepository;
    use crate::dto::{AccountDto, CreateAccountInput};
    use crate::error::AppError;
    use async_trait::async_trait;
    use tally_domain::ledger::validate_postings;

    #[derive(Clone, Default)]
    struct FakeRepository {
        created: std::sync::Arc<std::sync::Mutex<Option<NewAccount>>>,
    }

    #[async_trait]
    impl AccountsRepository for FakeRepository {
        async fn list(&self, _include_archived: bool) -> Result<Vec<AccountDto>, AppError> {
            Ok(Vec::new())
        }

        async fn create(&self, account: NewAccount) -> Result<AccountDto, AppError> {
            *self.created.lock().expect("lock") = Some(account.clone());
            Ok(AccountDto {
                id: account.id,
                name: account.name,
                account_type: account.account_type.as_str().to_owned(),
                credit_limit_cents: account.credit_limit_cents,
                balance_cents: 0,
                archived_at: None,
                created_at: "2026-10-31T12:00:00Z".to_owned(),
            })
        }

        async fn rename(&self, _id: &str, _name: &str) -> Result<AccountDto, AppError> {
            Err(AppError::not_found("conta não encontrada"))
        }

        async fn set_archived(&self, _id: &str, _archived: bool) -> Result<AccountDto, AppError> {
            Err(AppError::not_found("conta não encontrada"))
        }

        async fn delete(&self, _id: &str) -> Result<(), AppError> {
            Ok(())
        }
    }

    fn input(name: &str, account_type: &str, opening_balance_cents: i64) -> CreateAccountInput {
        CreateAccountInput {
            name: name.to_owned(),
            account_type: account_type.to_owned(),
            opening_balance_cents,
            credit_limit_cents: None,
        }
    }

    #[tokio::test]
    async fn composes_balanced_opening_balance_against_equity() {
        let repo = FakeRepository::default();
        let service = AccountsService::new(repo.clone());

        service
            .create(input("  Banco  ", "bank", 100_000))
            .await
            .expect("create should work");

        let created = repo
            .created
            .lock()
            .expect("lock")
            .clone()
            .expect("account should be captured");
        assert_eq!(created.name, "Banco");

        let entry = created.opening_entry.expect("opening entry");
        assert_eq!(entry.postings.len(), 2);
        assert_eq!(entry.postings[0].account_id, created.id);
        assert_eq!(entry.postings[0].amount.cents(), 100_000);
        assert_eq!(entry.postings[1].account_id, EQUITY_ACCOUNT_ID);
        assert_eq!(entry.postings[1].amount.cents(), -100_000);
        assert!(validate_postings(&entry.postings).is_ok());
        assert_eq!(entry.occurred_on.len(), 10);
    }

    #[tokio::test]
    async fn credit_card_debt_is_posted_as_a_liability() {
        let repo = FakeRepository::default();
        let service = AccountsService::new(repo.clone());

        service
            .create(CreateAccountInput {
                name: "Cartão".to_owned(),
                account_type: "credit_card".to_owned(),
                opening_balance_cents: 20_000,
                credit_limit_cents: Some(500_000),
            })
            .await
            .expect("create should work");

        let created = repo
            .created
            .lock()
            .expect("lock")
            .clone()
            .expect("account should be captured");
        assert_eq!(created.credit_limit_cents, Some(500_000));

        let entry = created.opening_entry.expect("opening entry");
        assert_eq!(entry.postings[0].amount.cents(), -20_000);
        assert_eq!(entry.postings[1].amount.cents(), 20_000);
        assert!(validate_postings(&entry.postings).is_ok());
    }

    #[tokio::test]
    async fn zero_opening_balance_skips_the_journal_entry() {
        let repo = FakeRepository::default();
        let service = AccountsService::new(repo.clone());

        service
            .create(input("Carteira", "cash", 0))
            .await
            .expect("create should work");

        let created = repo
            .created
            .lock()
            .expect("lock")
            .clone()
            .expect("account should be captured");
        assert!(created.opening_entry.is_none());
    }

    #[tokio::test]
    async fn rejects_invalid_input_before_persisting() {
        let repo = FakeRepository::default();
        let service = AccountsService::new(repo);

        assert!(matches!(
            service.create(input("   ", "bank", 0)).await,
            Err(AppError::Validation { .. })
        ));
        assert!(matches!(
            service.create(input("Banco", "invalid", 0)).await,
            Err(AppError::Validation { .. })
        ));
        assert!(matches!(
            service.create(input("Banco", "bank", i64::MAX)).await,
            Err(AppError::Validation { .. })
        ));
        assert!(matches!(
            service.create(input("Cartão", "credit_card", -100)).await,
            Err(AppError::Validation { .. })
        ));
        assert!(matches!(
            service
                .create(CreateAccountInput {
                    name: "Banco".to_owned(),
                    account_type: "bank".to_owned(),
                    opening_balance_cents: 0,
                    credit_limit_cents: Some(100),
                })
                .await,
            Err(AppError::Validation { .. })
        ));
    }
}
