use async_trait::async_trait;
use tally_domain::ledger::{Posting, validate_postings};
use tally_domain::money::Money;
use tally_domain::movement::{MovementKind, PaymentMethod};
use tally_domain::period::Period;
use uuid::Uuid;

use crate::dto::{
    AccountStatementDto, CreateAdjustmentInput, CreateExpenseInput, CreateIncomeInput,
    CreateTransferInput, MovementDto, MovementFilter, UpdateMovementInput,
};
use crate::error::AppError;
use crate::validation::{
    normalize_description, normalize_occurred, validate_date, validate_filter_date,
    validate_nonzero_cents, validate_positive_cents,
};

const EQUITY_ACCOUNT_ID: &str = "00000000-0000-7000-8000-000000000001";
const INCOME_ACCOUNT_ID: &str = "00000000-0000-7000-8000-000000000002";
const EXPENSE_ACCOUNT_ID: &str = "00000000-0000-7000-8000-000000000003";

#[derive(Debug, Clone)]
pub struct NewMovement {
    pub id: String,
    pub kind: MovementKind,
    pub occurred_on: String,
    pub occurred_time: Option<String>,
    pub manual_period: Option<String>,
    pub description: Option<String>,
    pub primary_account_id: String,
    pub counterparty_account_id: Option<String>,
    pub category_id: Option<String>,
    pub payment_method: Option<PaymentMethod>,
    pub postings: Vec<Posting>,
}

#[async_trait]
pub trait MovementsRepository: Send + Sync {
    async fn entry_kind(&self, id: &str) -> Result<String, AppError>;
    async fn create(&self, movement: NewMovement) -> Result<MovementDto, AppError>;
    async fn update(&self, id: &str, movement: NewMovement) -> Result<MovementDto, AppError>;
    async fn delete(&self, id: &str) -> Result<(), AppError>;
    async fn list(&self, filter: MovementFilter) -> Result<Vec<MovementDto>, AppError>;
    async fn statement(
        &self,
        account_id: &str,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<AccountStatementDto, AppError>;
}

pub struct MovementsService<R> {
    repo: R,
}

impl<R: MovementsRepository> MovementsService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn list(&self, filter: Option<MovementFilter>) -> Result<Vec<MovementDto>, AppError> {
        self.repo.list(normalize_filter(filter)?).await
    }

    pub async fn statement(
        &self,
        account_id: &str,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<AccountStatementDto, AppError> {
        let account_id = required_id(account_id, "conta")?;
        let from = validate_filter_date(from)?;
        let to = validate_filter_date(to)?;
        validate_date_range(from.as_deref(), to.as_deref())?;

        self.repo
            .statement(&account_id, from.as_deref(), to.as_deref())
            .await
    }

    pub async fn create_expense(&self, input: CreateExpenseInput) -> Result<MovementDto, AppError> {
        let account_id = required_id(&input.account_id, "conta")?;
        let category_id = required_id(&input.category_id, "categoria")?;
        let amount = validate_positive_cents(input.amount_cents)?;
        let occurred_on = validate_date(&input.occurred_on)?;
        let occurred = normalize_occurred(
            input.occurred_time.as_deref(),
            input.manual_period.as_deref(),
        )?;
        let description = normalize_description(input.description.as_deref())?;
        let payment_method = PaymentMethod::parse_phase_one(&input.payment_method)
            .ok_or_else(|| AppError::validation("método de pagamento inválido na Phase 1"))?;

        let postings = vec![
            Posting::new(
                Uuid::now_v7().to_string(),
                account_id.clone(),
                Money::from_cents(-amount),
            ),
            Posting::new(
                Uuid::now_v7().to_string(),
                EXPENSE_ACCOUNT_ID,
                Money::from_cents(amount),
            ),
        ];
        validate_postings(&postings).map_err(AppError::validation)?;

        self.repo
            .create(NewMovement {
                id: Uuid::now_v7().to_string(),
                kind: MovementKind::Expense,
                occurred_on,
                occurred_time: occurred.time,
                manual_period: occurred.manual_period,
                description,
                primary_account_id: account_id,
                counterparty_account_id: None,
                category_id: Some(category_id),
                payment_method: Some(payment_method),
                postings,
            })
            .await
    }

    pub async fn create_income(&self, input: CreateIncomeInput) -> Result<MovementDto, AppError> {
        let account_id = required_id(&input.account_id, "conta")?;
        let category_id = required_id(&input.category_id, "categoria")?;
        let amount = validate_positive_cents(input.amount_cents)?;
        let occurred_on = validate_date(&input.occurred_on)?;
        let occurred = normalize_occurred(
            input.occurred_time.as_deref(),
            input.manual_period.as_deref(),
        )?;
        let description = normalize_description(input.description.as_deref())?;
        let payment_method = PaymentMethod::parse_phase_one(&input.payment_method)
            .ok_or_else(|| AppError::validation("método de pagamento inválido na Phase 1"))?;

        let postings = vec![
            Posting::new(
                Uuid::now_v7().to_string(),
                account_id.clone(),
                Money::from_cents(amount),
            ),
            Posting::new(
                Uuid::now_v7().to_string(),
                INCOME_ACCOUNT_ID,
                Money::from_cents(-amount),
            ),
        ];
        validate_postings(&postings).map_err(AppError::validation)?;

        self.repo
            .create(NewMovement {
                id: Uuid::now_v7().to_string(),
                kind: MovementKind::Income,
                occurred_on,
                occurred_time: occurred.time,
                manual_period: occurred.manual_period,
                description,
                primary_account_id: account_id,
                counterparty_account_id: None,
                category_id: Some(category_id),
                payment_method: Some(payment_method),
                postings,
            })
            .await
    }

    pub async fn create_transfer(
        &self,
        input: CreateTransferInput,
    ) -> Result<MovementDto, AppError> {
        let from_account_id = required_id(&input.from_account_id, "conta de origem")?;
        let to_account_id = required_id(&input.to_account_id, "conta de destino")?;
        if from_account_id == to_account_id {
            return Err(AppError::validation(
                "as contas de origem e destino devem ser diferentes",
            ));
        }
        let amount = validate_positive_cents(input.amount_cents)?;
        let occurred_on = validate_date(&input.occurred_on)?;
        let occurred = normalize_occurred(
            input.occurred_time.as_deref(),
            input.manual_period.as_deref(),
        )?;
        let description = normalize_description(input.description.as_deref())?;

        let postings = vec![
            Posting::new(
                Uuid::now_v7().to_string(),
                from_account_id.clone(),
                Money::from_cents(-amount),
            ),
            Posting::new(
                Uuid::now_v7().to_string(),
                to_account_id.clone(),
                Money::from_cents(amount),
            ),
        ];
        validate_postings(&postings).map_err(AppError::validation)?;

        self.repo
            .create(NewMovement {
                id: Uuid::now_v7().to_string(),
                kind: MovementKind::Transfer,
                occurred_on,
                occurred_time: occurred.time,
                manual_period: occurred.manual_period,
                description,
                primary_account_id: from_account_id,
                counterparty_account_id: Some(to_account_id),
                category_id: None,
                payment_method: None,
                postings,
            })
            .await
    }

    pub async fn create_adjustment(
        &self,
        input: CreateAdjustmentInput,
    ) -> Result<MovementDto, AppError> {
        let account_id = required_id(&input.account_id, "conta")?;
        let amount = validate_nonzero_cents(input.amount_cents)?;
        let occurred_on = validate_date(&input.occurred_on)?;
        let description = normalize_description(input.description.as_deref())?;
        let counter_amount = amount
            .checked_neg()
            .ok_or_else(|| AppError::validation("valor de ajuste fora do limite"))?;

        let postings = vec![
            Posting::new(
                Uuid::now_v7().to_string(),
                account_id.clone(),
                Money::from_cents(amount),
            ),
            Posting::new(
                Uuid::now_v7().to_string(),
                EQUITY_ACCOUNT_ID,
                Money::from_cents(counter_amount),
            ),
        ];
        validate_postings(&postings).map_err(AppError::validation)?;

        self.repo
            .create(NewMovement {
                id: Uuid::now_v7().to_string(),
                kind: MovementKind::Adjustment,
                occurred_on,
                occurred_time: None,
                manual_period: None,
                description,
                primary_account_id: account_id,
                counterparty_account_id: None,
                category_id: None,
                payment_method: None,
                postings,
            })
            .await
    }

    pub async fn update(
        &self,
        id: &str,
        input: UpdateMovementInput,
    ) -> Result<MovementDto, AppError> {
        let id = required_id(id, "movimentação")?;
        let existing_kind = MovementKind::parse(&self.repo.entry_kind(&id).await?)
            .ok_or_else(|| AppError::conflict("esta entrada interna não pode ser editada"))?;

        if input.kind != existing_kind.as_str() {
            return Err(AppError::validation(
                "o tipo de uma movimentação não pode ser alterado",
            ));
        }

        let movement = build_update(existing_kind, input)?;
        self.repo.update(&id, movement).await
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        let id = required_id(id, "movimentação")?;
        let existing_kind = MovementKind::parse(&self.repo.entry_kind(&id).await?)
            .ok_or_else(|| AppError::conflict("esta entrada interna não pode ser excluída"))?;

        if !existing_kind.is_simple() {
            return Err(AppError::conflict(
                "esta entrada interna não pode ser excluída",
            ));
        }

        self.repo.delete(&id).await
    }
}

fn build_update(kind: MovementKind, input: UpdateMovementInput) -> Result<NewMovement, AppError> {
    let primary_account_id = required_id(&input.primary_account_id, "conta")?;
    let occurred_on = validate_date(&input.occurred_on)?;
    let description = normalize_description(input.description.as_deref())?;

    let (counterparty_account_id, category_id, payment_method, occurred, amount) = match kind {
        MovementKind::Expense | MovementKind::Income => {
            if input.counterparty_account_id.is_some() {
                return Err(AppError::validation(
                    "despesa ou receita não aceita conta de contraparte",
                ));
            }
            let category_id = required_id(
                input.category_id.as_deref().unwrap_or_default(),
                "categoria",
            )?;
            let payment_method =
                PaymentMethod::parse_phase_one(input.payment_method.as_deref().unwrap_or_default())
                    .ok_or_else(|| {
                        AppError::validation("método de pagamento inválido na Phase 1")
                    })?;
            let occurred = normalize_occurred(
                input.occurred_time.as_deref(),
                input.manual_period.as_deref(),
            )?;
            let amount = validate_positive_cents(input.amount_cents)?;

            (
                None,
                Some(category_id),
                Some(payment_method),
                Some(occurred),
                amount,
            )
        }
        MovementKind::Transfer => {
            if input.category_id.is_some() || input.payment_method.is_some() {
                return Err(AppError::validation(
                    "transferência não aceita categoria nem método de pagamento",
                ));
            }
            let to_account_id = required_id(
                input.counterparty_account_id.as_deref().unwrap_or_default(),
                "conta de destino",
            )?;
            if to_account_id == primary_account_id {
                return Err(AppError::validation(
                    "as contas de origem e destino devem ser diferentes",
                ));
            }
            let occurred = normalize_occurred(
                input.occurred_time.as_deref(),
                input.manual_period.as_deref(),
            )?;
            let amount = validate_positive_cents(input.amount_cents)?;

            (Some(to_account_id), None, None, Some(occurred), amount)
        }
        MovementKind::Adjustment => {
            if input.counterparty_account_id.is_some()
                || input.category_id.is_some()
                || input.payment_method.is_some()
            {
                return Err(AppError::validation(
                    "ajuste não aceita categoria, contraparte nem método de pagamento",
                ));
            }
            if input.occurred_time.is_some() || input.manual_period.is_some() {
                return Err(AppError::validation(
                    "ajuste não aceita horário nem período manual",
                ));
            }
            let amount = validate_nonzero_cents(input.amount_cents)?;

            (None, None, None, None, amount)
        }
    };

    let postings = match kind {
        MovementKind::Expense => vec![
            Posting::new(
                Uuid::now_v7().to_string(),
                primary_account_id.clone(),
                Money::from_cents(-amount),
            ),
            Posting::new(
                Uuid::now_v7().to_string(),
                EXPENSE_ACCOUNT_ID,
                Money::from_cents(amount),
            ),
        ],
        MovementKind::Income => vec![
            Posting::new(
                Uuid::now_v7().to_string(),
                primary_account_id.clone(),
                Money::from_cents(amount),
            ),
            Posting::new(
                Uuid::now_v7().to_string(),
                INCOME_ACCOUNT_ID,
                Money::from_cents(-amount),
            ),
        ],
        MovementKind::Transfer => vec![
            Posting::new(
                Uuid::now_v7().to_string(),
                primary_account_id.clone(),
                Money::from_cents(-amount),
            ),
            Posting::new(
                Uuid::now_v7().to_string(),
                counterparty_account_id
                    .clone()
                    .ok_or_else(|| AppError::validation("informe a conta de destino"))?,
                Money::from_cents(amount),
            ),
        ],
        MovementKind::Adjustment => vec![
            Posting::new(
                Uuid::now_v7().to_string(),
                primary_account_id.clone(),
                Money::from_cents(amount),
            ),
            Posting::new(
                Uuid::now_v7().to_string(),
                EQUITY_ACCOUNT_ID,
                Money::from_cents(
                    amount
                        .checked_neg()
                        .ok_or_else(|| AppError::validation("valor de ajuste fora do limite"))?,
                ),
            ),
        ],
    };
    validate_postings(&postings).map_err(AppError::validation)?;

    Ok(NewMovement {
        id: Uuid::now_v7().to_string(),
        kind,
        occurred_on,
        occurred_time: occurred.as_ref().and_then(|value| value.time.clone()),
        manual_period: occurred.and_then(|value| value.manual_period),
        description,
        primary_account_id,
        counterparty_account_id,
        category_id,
        payment_method,
        postings,
    })
}

fn normalize_filter(filter: Option<MovementFilter>) -> Result<MovementFilter, AppError> {
    let filter = filter.unwrap_or(MovementFilter {
        account_id: None,
        category_id: None,
        kind: None,
        period: None,
        from: None,
        to: None,
    });

    let kind = filter
        .kind
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            MovementKind::parse(value)
                .map(|kind| kind.as_str().to_owned())
                .ok_or_else(|| AppError::validation("tipo de movimentação inválido"))
        })
        .transpose()?;

    let period = filter
        .period
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            Period::parse(value)
                .map(|period| period.as_str().to_owned())
                .ok_or_else(|| AppError::validation("período inválido"))
        })
        .transpose()?;

    let from = validate_filter_date(filter.from.as_deref())?;
    let to = validate_filter_date(filter.to.as_deref())?;
    validate_date_range(from.as_deref(), to.as_deref())?;

    Ok(MovementFilter {
        account_id: normalize_optional_id(filter.account_id),
        category_id: normalize_optional_id(filter.category_id),
        kind,
        period,
        from,
        to,
    })
}

fn normalize_optional_id(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn required_id(value: &str, entity: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::validation(format!("informe a {entity}")));
    }
    Ok(value.to_owned())
}

fn validate_date_range(from: Option<&str>, to: Option<&str>) -> Result<(), AppError> {
    if let (Some(from), Some(to)) = (from, to)
        && from > to
    {
        return Err(AppError::validation(
            "a data inicial não pode ser posterior à data final",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{EXPENSE_ACCOUNT_ID, MovementsRepository, MovementsService, NewMovement};
    use crate::dto::{CreateExpenseInput, MovementDto};
    use crate::error::AppError;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use tally_domain::ledger::validate_postings;

    #[derive(Clone, Default)]
    struct FakeRepository {
        created: Arc<Mutex<Option<NewMovement>>>,
    }

    #[async_trait]
    impl MovementsRepository for FakeRepository {
        async fn entry_kind(&self, _id: &str) -> Result<String, AppError> {
            Ok("expense".to_owned())
        }

        async fn create(&self, movement: NewMovement) -> Result<MovementDto, AppError> {
            *self.created.lock().expect("lock") = Some(movement.clone());
            Ok(MovementDto {
                id: movement.id,
                kind: movement.kind.as_str().to_owned(),
                occurred_on: movement.occurred_on,
                occurred_time: movement.occurred_time,
                manual_period: movement.manual_period,
                period: Some("tarde".to_owned()),
                description: movement.description,
                primary_account_id: movement.primary_account_id,
                primary_account_name: "Banco".to_owned(),
                counterparty_account_id: None,
                counterparty_account_name: None,
                category_id: movement.category_id,
                category_name: Some("Mercado".to_owned()),
                payment_method: movement
                    .payment_method
                    .map(|value| value.as_str().to_owned()),
                amount_cents: -100,
                created_at: "2026-10-31T12:00:00Z".to_owned(),
                installment_plan_id: None,
                installment_count: None,
                first_installment_cents: None,
            })
        }

        async fn update(&self, _id: &str, _movement: NewMovement) -> Result<MovementDto, AppError> {
            Err(AppError::not_found("movimentação não encontrada"))
        }

        async fn delete(&self, _id: &str) -> Result<(), AppError> {
            Ok(())
        }

        async fn list(
            &self,
            _filter: crate::dto::MovementFilter,
        ) -> Result<Vec<MovementDto>, AppError> {
            Ok(Vec::new())
        }

        async fn statement(
            &self,
            _account_id: &str,
            _from: Option<&str>,
            _to: Option<&str>,
        ) -> Result<crate::dto::AccountStatementDto, AppError> {
            Err(AppError::not_found("conta não encontrada"))
        }
    }

    fn expense(cents: i64) -> CreateExpenseInput {
        CreateExpenseInput {
            account_id: "account".to_owned(),
            category_id: "category".to_owned(),
            amount_cents: cents,
            occurred_on: "2026-10-31".to_owned(),
            occurred_time: Some("12:00".to_owned()),
            manual_period: None,
            description: Some("  Mercado  ".to_owned()),
            payment_method: "pix".to_owned(),
        }
    }

    #[tokio::test]
    async fn composes_balanced_expense_postings() {
        let repo = FakeRepository::default();
        let service = MovementsService::new(repo.clone());

        service
            .create_expense(expense(12_500))
            .await
            .expect("expense should work");

        let movement = repo
            .created
            .lock()
            .expect("lock")
            .clone()
            .expect("movement should be captured");
        assert_eq!(movement.postings.len(), 2);
        assert_eq!(movement.postings[0].amount.cents(), -12_500);
        assert_eq!(movement.postings[1].account_id, EXPENSE_ACCOUNT_ID);
        assert_eq!(movement.postings[1].amount.cents(), 12_500);
        assert_eq!(movement.description.as_deref(), Some("Mercado"));
        assert!(validate_postings(&movement.postings).is_ok());
    }

    #[tokio::test]
    async fn rejects_invalid_expense_before_persisting() {
        let repo = FakeRepository::default();
        let service = MovementsService::new(repo.clone());

        assert!(matches!(
            service.create_expense(expense(0)).await,
            Err(AppError::Validation { .. })
        ));
        assert!(matches!(
            service.create_expense(expense(-10)).await,
            Err(AppError::Validation { .. })
        ));

        let mut both_periods = expense(100);
        both_periods.manual_period = Some("noite".to_owned());
        assert!(matches!(
            service.create_expense(both_periods).await,
            Err(AppError::Validation { .. })
        ));

        assert!(repo.created.lock().expect("lock").is_none());
    }
}
