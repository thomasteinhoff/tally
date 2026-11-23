use async_trait::async_trait;
use chrono::{Datelike, Local, Months, NaiveDate};
use tally_domain::movement::PaymentMethod;

use crate::dto::{
    CashFlowReportDto, CategoryReportDto, CommitmentsReportDto, DashboardDto, ExpenseTrendDto,
    NetWorthHistoryDto, NetWorthReportDto, PeriodSpendingReportDto, ReportFilter,
};
use crate::error::AppError;

const VALID_PERIODS: [&str; 5] = ["madrugada", "manha", "tarde", "noite", "custom"];
const DEFAULT_COMMITMENT_MONTHS: u32 = 12;
const MAX_COMMITMENT_MONTHS: u32 = 60;
const TREND_MONTHS: u32 = 6;

/// Filtro já resolvido (datas obrigatórias) usado pelo repositório.
#[derive(Debug, Clone)]
pub struct ReportQuery {
    pub from: String,
    pub to: String,
    pub period: Option<String>,
    pub category_id: Option<String>,
    pub account_id: Option<String>,
    pub payment_method: Option<String>,
}

#[async_trait]
pub trait ReportsRepository: Send + Sync {
    async fn category_report(
        &self,
        kind: &str,
        query: &ReportQuery,
    ) -> Result<CategoryReportDto, AppError>;

    async fn cash_flow(&self, query: &ReportQuery) -> Result<CashFlowReportDto, AppError>;

    async fn period_spending(
        &self,
        query: &ReportQuery,
    ) -> Result<PeriodSpendingReportDto, AppError>;

    /// `months` são os rótulos `AAAA-MM` da série, em ordem; a resposta é
    /// preenchida com zero para meses sem despesa.
    async fn expense_trend(
        &self,
        from: &str,
        to: &str,
        months: &[String],
    ) -> Result<ExpenseTrendDto, AppError>;

    /// `dates` são os fechamentos mensais da série, em ordem crescente.
    async fn net_worth_history(&self, dates: &[String]) -> Result<NetWorthHistoryDto, AppError>;

    async fn net_worth(&self, as_of: &str) -> Result<NetWorthReportDto, AppError>;

    async fn commitments(
        &self,
        as_of: &str,
        start_after: &str,
        horizon_end: &str,
    ) -> Result<CommitmentsReportDto, AppError>;

    async fn overdue_installments(&self, as_of: &str) -> Result<i64, AppError>;

    async fn card_totals(&self) -> Result<(i64, i64), AppError>;
}

pub struct ReportsService<R> {
    repo: R,
}

impl<R: ReportsRepository> ReportsService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn spending(&self, filter: ReportFilter) -> Result<CategoryReportDto, AppError> {
        let query = resolve_query(filter)?;
        self.repo.category_report("expense", &query).await
    }

    pub async fn income(&self, filter: ReportFilter) -> Result<CategoryReportDto, AppError> {
        let query = resolve_query(filter)?;
        self.repo.category_report("income", &query).await
    }

    pub async fn cash_flow(&self, filter: ReportFilter) -> Result<CashFlowReportDto, AppError> {
        let query = resolve_query(filter)?;
        self.repo.cash_flow(&query).await
    }

    /// Gastos por faixa do dia no mês de `as_of` (padrão: mês corrente).
    pub async fn period_spending(
        &self,
        as_of: Option<String>,
    ) -> Result<PeriodSpendingReportDto, AppError> {
        let date = resolve_date(as_of, "as_of")?.unwrap_or_else(today);
        let query = ReportQuery {
            from: format_date(first_of_month(date)),
            to: format_date(end_of_month(date)),
            period: None,
            category_id: None,
            account_id: None,
            payment_method: None,
        };
        self.repo.period_spending(&query).await
    }

    /// Despesas por competência nos seis meses terminando no mês de `as_of`.
    pub async fn expense_trend(&self, as_of: Option<String>) -> Result<ExpenseTrendDto, AppError> {
        let date = resolve_date(as_of, "as_of")?.unwrap_or_else(today);
        let months = month_starts_ending_at(date, TREND_MONTHS);
        let from = format_date(months[0]);
        let to = format_date(end_of_month(months[months.len() - 1]));
        let labels = months
            .iter()
            .map(|month| month.format("%Y-%m").to_string())
            .collect::<Vec<_>>();
        self.repo.expense_trend(&from, &to, &labels).await
    }

    /// Patrimônio líquido no fechamento de cada um dos seis meses terminando
    /// no mês de `as_of`; o último ponto é o fechamento do mês selecionado.
    pub async fn net_worth_history(
        &self,
        as_of: Option<String>,
    ) -> Result<NetWorthHistoryDto, AppError> {
        let date = resolve_date(as_of, "as_of")?.unwrap_or_else(today);
        let dates = month_starts_ending_at(date, TREND_MONTHS)
            .iter()
            .map(|month| format_date(end_of_month(*month)))
            .collect::<Vec<_>>();
        self.repo.net_worth_history(&dates).await
    }

    pub async fn net_worth(&self, as_of: Option<String>) -> Result<NetWorthReportDto, AppError> {
        let as_of = resolve_as_of(as_of)?;
        self.repo.net_worth(&as_of).await
    }

    pub async fn commitments(
        &self,
        as_of: Option<String>,
        months: Option<u32>,
    ) -> Result<CommitmentsReportDto, AppError> {
        let as_of_date = resolve_date(as_of, "as_of")?.unwrap_or_else(today);
        let months = months.unwrap_or(DEFAULT_COMMITMENT_MONTHS);

        if !(1..=MAX_COMMITMENT_MONTHS).contains(&months) {
            return Err(AppError::validation(format!(
                "o horizonte deve estar entre 1 e {MAX_COMMITMENT_MONTHS} meses"
            )));
        }

        let as_of = format_date(as_of_date);
        let start_after = format_date(end_of_month(as_of_date));
        let horizon_end = format_date(add_months_end(as_of_date, months));

        self.repo
            .commitments(&as_of, &start_after, &horizon_end)
            .await
    }

    pub async fn dashboard(&self) -> Result<DashboardDto, AppError> {
        let as_of_date = today();
        let as_of = format_date(as_of_date);
        let month_query = ReportQuery {
            from: format_date(first_of_month(as_of_date)),
            to: format_date(end_of_month(as_of_date)),
            period: None,
            category_id: None,
            account_id: None,
            payment_method: None,
        };
        let start_after = format_date(end_of_month(as_of_date));
        let horizon_end = format_date(add_months_end(as_of_date, DEFAULT_COMMITMENT_MONTHS));

        let net_worth = self.repo.net_worth(&as_of).await?;
        let spending = self.repo.category_report("expense", &month_query).await?;
        let income = self.repo.category_report("income", &month_query).await?;
        let (card_debt_cents, card_credit_cents) = self.repo.card_totals().await?;
        let overdue_installments_cents = self.repo.overdue_installments(&as_of).await?;
        let commitments = self
            .repo
            .commitments(&as_of, &start_after, &horizon_end)
            .await?;

        Ok(DashboardDto {
            as_of,
            month_from: month_query.from,
            month_to: month_query.to,
            net_worth_cents: net_worth.net_worth_cents,
            assets_cents: net_worth.assets_cents,
            liabilities_cents: net_worth.liabilities_cents,
            savings_cents: net_worth.savings_cents,
            total_with_savings_cents: net_worth.total_with_savings_cents,
            month_income_cents: income.total_cents,
            month_expense_cents: spending.total_cents,
            month_result_cents: income.total_cents - spending.total_cents,
            card_debt_cents,
            card_credit_cents,
            overdue_installments_cents,
            future_commitments_cents: commitments.total_cents,
            top_expenses: spending.categories.into_iter().take(5).collect(),
        })
    }
}

fn resolve_query(filter: ReportFilter) -> Result<ReportQuery, AppError> {
    let today = today();
    let from = resolve_date(filter.from, "de")?.unwrap_or_else(|| first_of_month(today));
    let to = resolve_date(filter.to, "até")?.unwrap_or_else(|| end_of_month(today));

    if from > to {
        return Err(AppError::validation(
            "a data inicial não pode ser posterior à data final",
        ));
    }

    if let Some(period) = filter.period.as_deref()
        && !VALID_PERIODS.contains(&period)
    {
        return Err(AppError::validation("período inválido"));
    }

    let payment_method = normalize_optional(filter.payment_method).map(|value| {
        PaymentMethod::parse(&value)
            .map(|method| method.as_str().to_owned())
            .ok_or_else(|| AppError::validation("método de pagamento inválido"))
    });
    let payment_method = payment_method.transpose()?;

    Ok(ReportQuery {
        from: format_date(from),
        to: format_date(to),
        period: filter.period,
        category_id: filter.category_id,
        account_id: normalize_optional(filter.account_id),
        payment_method,
    })
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn resolve_as_of(as_of: Option<String>) -> Result<String, AppError> {
    Ok(format_date(
        resolve_date(as_of, "as_of")?.unwrap_or_else(today),
    ))
}

fn resolve_date(raw: Option<String>, field: &str) -> Result<Option<NaiveDate>, AppError> {
    raw.map(|value| {
        NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|_| {
            AppError::validation(format!("data inválida em `{field}`: use AAAA-MM-DD"))
        })
    })
    .transpose()
}

fn today() -> NaiveDate {
    Local::now().date_naive()
}

fn first_of_month(date: NaiveDate) -> NaiveDate {
    date.with_day(1).expect("todo mês tem o dia 1")
}

fn end_of_month(date: NaiveDate) -> NaiveDate {
    first_of_month(date)
        .checked_add_months(Months::new(1))
        .and_then(|next| next.pred_opt())
        .expect("o fim do mês é sempre válido")
}

fn add_months_end(date: NaiveDate, months: u32) -> NaiveDate {
    end_of_month(date)
        .checked_add_months(Months::new(months))
        .map(end_of_month)
        .expect("o horizonte é limitado a 60 meses")
}

/// Primeiro dia de cada mês da série, em ordem crescente, terminando no mês
/// de `date`.
fn month_starts_ending_at(date: NaiveDate, count: u32) -> Vec<NaiveDate> {
    assert!(count > 0, "a série precisa de pelo menos um mês");
    let start = first_of_month(date)
        .checked_sub_months(Months::new(count - 1))
        .expect("série mensal limitada a valores válidos");

    (0..count)
        .map(|offset| {
            start
                .checked_add_months(Months::new(offset))
                .expect("mês da série é sempre válido")
        })
        .collect()
}

fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::{add_months_end, end_of_month, month_starts_ending_at, resolve_query};
    use crate::dto::ReportFilter;
    use crate::error::AppError;
    use chrono::NaiveDate;

    fn date(raw: &str) -> NaiveDate {
        NaiveDate::parse_from_str(raw, "%Y-%m-%d").expect("data")
    }

    #[test]
    fn rejects_inverted_ranges_and_invalid_periods() {
        assert!(matches!(
            resolve_query(ReportFilter {
                from: Some("2026-05-01".to_owned()),
                to: Some("2026-04-01".to_owned()),
                period: None,
                category_id: None,
                account_id: None,
                payment_method: None,
            }),
            Err(AppError::Validation { .. })
        ));
        assert!(matches!(
            resolve_query(ReportFilter {
                from: Some("2026-04-01".to_owned()),
                to: Some("2026-05-01".to_owned()),
                period: Some("invalido".to_owned()),
                category_id: None,
                account_id: None,
                payment_method: None,
            }),
            Err(AppError::Validation { .. })
        ));
        assert!(matches!(
            resolve_query(ReportFilter {
                from: None,
                to: None,
                period: None,
                category_id: None,
                account_id: None,
                payment_method: Some("boleto".to_owned()),
            }),
            Err(AppError::Validation { .. })
        ));
    }

    #[test]
    fn month_and_horizon_helpers_use_the_last_day() {
        assert_eq!(
            end_of_month(date("2028-02-10"))
                .format("%Y-%m-%d")
                .to_string(),
            "2028-02-29"
        );
        assert_eq!(
            add_months_end(date("2026-01-31"), 12)
                .format("%Y-%m-%d")
                .to_string(),
            "2027-01-31"
        );
    }

    #[test]
    fn six_month_series_crosses_the_year_boundary() {
        let months = month_starts_ending_at(date("2026-02-15"), 6)
            .iter()
            .map(|month| month.format("%Y-%m").to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            months,
            [
                "2025-09", "2025-10", "2025-11", "2025-12", "2026-01", "2026-02"
            ]
        );

        let ends = month_starts_ending_at(date("2026-03-31"), 6)
            .iter()
            .map(|month| end_of_month(*month).format("%Y-%m-%d").to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            ends,
            [
                "2025-10-31",
                "2025-11-30",
                "2025-12-31",
                "2026-01-31",
                "2026-02-28",
                "2026-03-31",
            ]
        );
    }
}
