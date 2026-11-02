use async_trait::async_trait;
use chrono::NaiveDate;
use tally_domain::installment::build_schedule;
use tally_domain::ledger::{Posting, validate_postings};
use tally_domain::money::Money;
use tally_domain::movement::PaymentMethod;
use tally_domain::period::Period;
use uuid::Uuid;

use crate::dto::{
    CardPaymentDto, CardSummaryDto, CreateCardPaymentInput, CreateCardPurchaseInput,
    InstallmentPlanDto,
};
use crate::error::AppError;
use crate::validation::{
    normalize_description, normalize_occurred, validate_date, validate_positive_cents,
};

/// Conta interna de despesas.
pub const EXPENSE_ACCOUNT_ID: &str = "00000000-0000-7000-8000-000000000003";
/// Conta interna "Despesa a reconhecer".
pub const RECOGNITION_ACCOUNT_ID: &str = "00000000-0000-7000-8000-000000000004";

#[derive(Debug, Clone)]
pub struct NewRecognitionEntry {
    pub id: String,
    pub postings: [Posting; 2],
}

#[derive(Debug, Clone)]
pub struct NewInstallment {
    pub id: String,
    pub seq: i64,
    pub due_on: String,
    pub amount_cents: i64,
    pub recognition_entry: NewRecognitionEntry,
}

#[derive(Debug, Clone)]
pub struct NewCardPurchase {
    pub plan_id: String,
    pub card_account_id: String,
    pub category_id: String,
    pub description: Option<String>,
    pub total_cents: i64,
    pub installment_count: i64,
    pub first_due_on: String,
    pub period: String,
    pub payment_method: PaymentMethod,
    pub purchased_on: String,
    pub purchase_entry_id: String,
    pub purchase_postings: [Posting; 2],
    pub installments: Vec<NewInstallment>,
}

#[derive(Debug, Clone)]
pub struct NewCardPayment {
    pub id: String,
    pub card_account_id: String,
    pub bank_account_id: String,
    pub entry_id: String,
    pub amount_cents: i64,
    pub occurred_on: String,
    pub occurred_time: Option<String>,
    pub manual_period: Option<String>,
    pub description: Option<String>,
    pub postings: [Posting; 2],
}

#[async_trait]
pub trait CardsRepository: Send + Sync {
    async fn list_cards(&self) -> Result<Vec<CardSummaryDto>, AppError>;
    async fn card_summary(&self, card_account_id: &str) -> Result<CardSummaryDto, AppError>;
    async fn create_purchase(
        &self,
        purchase: NewCardPurchase,
    ) -> Result<InstallmentPlanDto, AppError>;
    async fn list_plans(
        &self,
        card_account_id: Option<&str>,
    ) -> Result<Vec<InstallmentPlanDto>, AppError>;
    async fn get_plan(&self, plan_id: &str) -> Result<InstallmentPlanDto, AppError>;
    async fn delete_plan(&self, plan_id: &str) -> Result<(), AppError>;
    async fn create_payment(&self, payment: NewCardPayment) -> Result<CardPaymentDto, AppError>;
    async fn list_payments(
        &self,
        card_account_id: Option<&str>,
    ) -> Result<Vec<CardPaymentDto>, AppError>;
    async fn delete_payment(&self, payment_id: &str) -> Result<(), AppError>;
}

pub struct CardsService<R> {
    repo: R,
}

impl<R: CardsRepository> CardsService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn list_cards(&self) -> Result<Vec<CardSummaryDto>, AppError> {
        self.repo.list_cards().await
    }

    pub async fn card_summary(&self, card_account_id: &str) -> Result<CardSummaryDto, AppError> {
        let card_account_id = required_id(card_account_id, "cartão")?;
        self.repo.card_summary(&card_account_id).await
    }

    pub async fn create_purchase(
        &self,
        input: CreateCardPurchaseInput,
    ) -> Result<InstallmentPlanDto, AppError> {
        let purchase = build_purchase(
            Uuid::now_v7().to_string(),
            input.card_account_id,
            input.category_id,
            input.total_cents,
            input.installment_count,
            input.first_due_on,
            input.occurred_on,
            input.occurred_time.as_deref(),
            input.manual_period.as_deref(),
            input.description.as_deref(),
            &input.payment_method,
        )?;

        self.repo.create_purchase(purchase).await
    }

    pub async fn list_plans(
        &self,
        card_account_id: Option<String>,
    ) -> Result<Vec<InstallmentPlanDto>, AppError> {
        let card_account_id = normalize_optional_id(card_account_id);
        self.repo.list_plans(card_account_id.as_deref()).await
    }

    pub async fn get_plan(&self, plan_id: &str) -> Result<InstallmentPlanDto, AppError> {
        let plan_id = required_id(plan_id, "parcelamento")?;
        self.repo.get_plan(&plan_id).await
    }

    pub async fn delete_plan(&self, plan_id: &str) -> Result<(), AppError> {
        let plan_id = required_id(plan_id, "parcelamento")?;
        self.repo.delete_plan(&plan_id).await
    }

    pub async fn create_payment(
        &self,
        input: CreateCardPaymentInput,
    ) -> Result<CardPaymentDto, AppError> {
        let payment = build_payment(
            Uuid::now_v7().to_string(),
            input.card_account_id,
            input.bank_account_id,
            input.amount_cents,
            input.occurred_on,
            input.occurred_time.as_deref(),
            input.manual_period.as_deref(),
            input.description.as_deref(),
        )?;

        self.repo.create_payment(payment).await
    }

    pub async fn list_payments(
        &self,
        card_account_id: Option<String>,
    ) -> Result<Vec<CardPaymentDto>, AppError> {
        let card_account_id = normalize_optional_id(card_account_id);
        self.repo.list_payments(card_account_id.as_deref()).await
    }

    pub async fn delete_payment(&self, payment_id: &str) -> Result<(), AppError> {
        let payment_id = required_id(payment_id, "pagamento")?;
        self.repo.delete_payment(&payment_id).await
    }
}

#[allow(clippy::too_many_arguments)]
fn build_purchase(
    plan_id: String,
    card_account_id: String,
    category_id: String,
    total_cents: i64,
    installment_count: i64,
    first_due_on: String,
    occurred_on: String,
    occurred_time: Option<&str>,
    manual_period: Option<&str>,
    description: Option<&str>,
    payment_method: &str,
) -> Result<NewCardPurchase, AppError> {
    let card_account_id = required_id(&card_account_id, "cartão")?;
    let category_id = required_id(&category_id, "categoria")?;
    let total_cents = validate_positive_cents(total_cents)?;
    let installment_count = validate_installment_count(installment_count, total_cents)?;
    let first_due_on = validate_date(&first_due_on)?;
    let purchased_on = validate_date(&occurred_on)?;
    let description = normalize_description(description)?;
    let payment_method = parse_card_method(payment_method)?;
    let period = derive_period(occurred_time, manual_period)?;

    let first_due = NaiveDate::parse_from_str(&first_due_on, "%Y-%m-%d")
        .map_err(|_| AppError::validation("informe uma data de vencimento válida"))?;
    let schedule = build_schedule(total_cents, installment_count as u32, first_due)
        .map_err(AppError::validation)?;

    let purchase_postings = [
        Posting::new(
            Uuid::now_v7().to_string(),
            RECOGNITION_ACCOUNT_ID,
            Money::from_cents(total_cents),
        ),
        Posting::new(
            Uuid::now_v7().to_string(),
            card_account_id.clone(),
            Money::from_cents(-total_cents),
        ),
    ];
    validate_postings(&purchase_postings).map_err(AppError::validation)?;

    let installments = schedule
        .amounts
        .iter()
        .zip(schedule.due_dates.iter())
        .enumerate()
        .map(|(index, (amount, due_on))| {
            let amount_cents = amount.cents();
            let postings = [
                Posting::new(
                    Uuid::now_v7().to_string(),
                    EXPENSE_ACCOUNT_ID,
                    Money::from_cents(amount_cents),
                ),
                Posting::new(
                    Uuid::now_v7().to_string(),
                    RECOGNITION_ACCOUNT_ID,
                    Money::from_cents(-amount_cents),
                ),
            ];
            validate_postings(&postings).map_err(AppError::validation)?;

            Ok(NewInstallment {
                id: Uuid::now_v7().to_string(),
                seq: index as i64 + 1,
                due_on: due_on.format("%Y-%m-%d").to_string(),
                amount_cents,
                recognition_entry: NewRecognitionEntry {
                    id: Uuid::now_v7().to_string(),
                    postings,
                },
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(NewCardPurchase {
        plan_id,
        card_account_id,
        category_id,
        description,
        total_cents,
        installment_count,
        first_due_on,
        period: period.as_str().to_owned(),
        payment_method,
        purchased_on,
        purchase_entry_id: Uuid::now_v7().to_string(),
        purchase_postings,
        installments,
    })
}

#[allow(clippy::too_many_arguments)]
fn build_payment(
    id: String,
    card_account_id: String,
    bank_account_id: String,
    amount_cents: i64,
    occurred_on: String,
    occurred_time: Option<&str>,
    manual_period: Option<&str>,
    description: Option<&str>,
) -> Result<NewCardPayment, AppError> {
    let card_account_id = required_id(&card_account_id, "cartão")?;
    let bank_account_id = required_id(&bank_account_id, "conta de origem")?;
    if card_account_id == bank_account_id {
        return Err(AppError::validation(
            "o cartão e a conta de origem devem ser diferentes",
        ));
    }
    let amount_cents = validate_positive_cents(amount_cents)?;
    let occurred_on = validate_date(&occurred_on)?;
    let occurred = normalize_occurred(occurred_time, manual_period)?;
    let description = normalize_description(description)?;

    let postings = [
        Posting::new(
            Uuid::now_v7().to_string(),
            bank_account_id.clone(),
            Money::from_cents(-amount_cents),
        ),
        Posting::new(
            Uuid::now_v7().to_string(),
            card_account_id.clone(),
            Money::from_cents(amount_cents),
        ),
    ];
    validate_postings(&postings).map_err(AppError::validation)?;

    Ok(NewCardPayment {
        id,
        card_account_id,
        bank_account_id,
        entry_id: Uuid::now_v7().to_string(),
        amount_cents,
        occurred_on,
        occurred_time: occurred.time,
        manual_period: occurred.manual_period,
        description,
        postings,
    })
}

fn derive_period(time: Option<&str>, manual_period: Option<&str>) -> Result<Period, AppError> {
    let occurred = normalize_occurred(time, manual_period)?;

    match (occurred.time, occurred.manual_period) {
        (Some(time), _) => Period::from_time(&time),
        (None, Some(period)) => Period::parse(&period),
        (None, None) => None,
    }
    .ok_or_else(|| AppError::validation("informe um período válido"))
}

fn parse_card_method(value: &str) -> Result<PaymentMethod, AppError> {
    match PaymentMethod::parse(value.trim()) {
        Some(PaymentMethod::Credit) => Ok(PaymentMethod::Credit),
        Some(PaymentMethod::SubscriptionCredit) => Ok(PaymentMethod::SubscriptionCredit),
        _ => Err(AppError::validation(
            "método de compra no cartão deve ser crédito ou assinatura (crédito)",
        )),
    }
}

fn validate_installment_count(count: i64, total_cents: i64) -> Result<i64, AppError> {
    if !(1..=60).contains(&count) {
        return Err(AppError::validation(
            "a quantidade de parcelas deve ficar entre 1 e 60",
        ));
    }
    if count > total_cents {
        return Err(AppError::validation(
            "cada parcela precisa ter pelo menos um centavo",
        ));
    }
    Ok(count)
}

fn required_id(value: &str, entity: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::validation(format!("informe a {entity}")));
    }
    Ok(value.to_owned())
}

fn normalize_optional_id(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{CardsRepository, CardsService, NewCardPayment, NewCardPurchase};
    use crate::dto::{CardPaymentDto, CardSummaryDto, CreateCardPurchaseInput, InstallmentPlanDto};
    use crate::error::AppError;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct FakeRepository {
        purchase: Arc<Mutex<Option<NewCardPurchase>>>,
        payment: Arc<Mutex<Option<NewCardPayment>>>,
    }

    #[async_trait]
    impl CardsRepository for FakeRepository {
        async fn list_cards(&self) -> Result<Vec<CardSummaryDto>, AppError> {
            Ok(Vec::new())
        }

        async fn card_summary(&self, _card_account_id: &str) -> Result<CardSummaryDto, AppError> {
            Err(AppError::not_found("cartão não encontrado"))
        }

        async fn create_purchase(
            &self,
            purchase: NewCardPurchase,
        ) -> Result<InstallmentPlanDto, AppError> {
            *self.purchase.lock().expect("lock") = Some(purchase);
            Err(AppError::not_found("não usado"))
        }

        async fn list_plans(
            &self,
            _card_account_id: Option<&str>,
        ) -> Result<Vec<InstallmentPlanDto>, AppError> {
            Ok(Vec::new())
        }

        async fn get_plan(&self, _plan_id: &str) -> Result<InstallmentPlanDto, AppError> {
            Err(AppError::not_found("parcelamento não encontrado"))
        }

        async fn delete_plan(&self, _plan_id: &str) -> Result<(), AppError> {
            Ok(())
        }

        async fn create_payment(
            &self,
            payment: NewCardPayment,
        ) -> Result<CardPaymentDto, AppError> {
            *self.payment.lock().expect("lock") = Some(payment);
            Err(AppError::not_found("não usado"))
        }

        async fn list_payments(
            &self,
            _card_account_id: Option<&str>,
        ) -> Result<Vec<CardPaymentDto>, AppError> {
            Ok(Vec::new())
        }

        async fn delete_payment(&self, _payment_id: &str) -> Result<(), AppError> {
            Ok(())
        }
    }

    fn purchase(cents: i64, count: i64) -> CreateCardPurchaseInput {
        CreateCardPurchaseInput {
            card_account_id: "card".to_owned(),
            category_id: "category".to_owned(),
            total_cents: cents,
            installment_count: count,
            first_due_on: "2026-01-31".to_owned(),
            occurred_on: "2026-01-15".to_owned(),
            occurred_time: Some("12:00".to_owned()),
            manual_period: None,
            description: Some("  Notebook  ".to_owned()),
            payment_method: "credit".to_owned(),
        }
    }

    #[tokio::test]
    async fn composes_installments_with_exact_total_and_month_end_dates() {
        let repo = FakeRepository::default();
        let service = CardsService::new(repo.clone());

        let _ = service
            .create_purchase(purchase(90_000, 3))
            .await
            .expect_err("fake repository does not return the plan");

        let built = repo
            .purchase
            .lock()
            .expect("lock")
            .clone()
            .expect("purchase should be captured");

        assert_eq!(built.total_cents, 90_000);
        assert_eq!(built.installments.len(), 3);
        assert_eq!(
            built
                .installments
                .iter()
                .map(|installment| installment.amount_cents)
                .sum::<i64>(),
            90_000
        );
        assert_eq!(built.installments[1].due_on, "2026-02-28");
        assert_eq!(built.installments[2].due_on, "2026-03-31");
        assert_eq!(built.period, "tarde");
        assert_eq!(built.description.as_deref(), Some("Notebook"));
        assert!(tally_domain::ledger::validate_postings(&built.purchase_postings).is_ok());
        assert!(
            tally_domain::ledger::validate_postings(
                &built.installments[0].recognition_entry.postings
            )
            .is_ok()
        );
    }

    #[tokio::test]
    async fn rejects_invalid_purchase_before_persisting() {
        let repo = FakeRepository::default();
        let service = CardsService::new(repo.clone());

        assert!(matches!(
            service.create_purchase(purchase(0, 1)).await,
            Err(AppError::Validation { .. })
        ));
        assert!(matches!(
            service.create_purchase(purchase(90_000, 0)).await,
            Err(AppError::Validation { .. })
        ));
        assert!(matches!(
            service.create_purchase(purchase(90_000, 61)).await,
            Err(AppError::Validation { .. })
        ));
        assert!(matches!(
            service.create_purchase(purchase(2, 3)).await,
            Err(AppError::Validation { .. })
        ));

        let mut both_periods = purchase(90_000, 3);
        both_periods.manual_period = Some("noite".to_owned());
        assert!(matches!(
            service.create_purchase(both_periods).await,
            Err(AppError::Validation { .. })
        ));

        let mut wrong_method = purchase(90_000, 3);
        wrong_method.payment_method = "pix".to_owned();
        assert!(matches!(
            service.create_purchase(wrong_method).await,
            Err(AppError::Validation { .. })
        ));

        assert!(repo.purchase.lock().expect("lock").is_none());
    }
}
