use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct SettingsDto {
    pub backup_retention: u32,
    pub first_run_completed: bool,
    pub last_backup_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct IntegrityReportDto {
    pub ok: bool,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct BootstrapDto {
    pub app_version: String,
    pub db_path: String,
    pub data_dir: String,
    #[ts(type = "number")]
    pub schema_version: i64,
    pub settings: SettingsDto,
    #[ts(type = "number")]
    pub system_accounts: i64,
    pub integrity: IntegrityReportDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct AccountDto {
    pub id: String,
    pub name: String,
    pub account_type: String,
    #[ts(type = "number | null")]
    pub credit_limit_cents: Option<i64>,
    #[ts(type = "number")]
    pub balance_cents: i64,
    pub archived_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CreateAccountInput {
    pub name: String,
    pub account_type: String,
    #[ts(type = "number")]
    pub opening_balance_cents: i64,
    #[ts(type = "number | null")]
    pub credit_limit_cents: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CategoryDto {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub archived_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CreateCategoryInput {
    pub name: String,
    pub kind: String,
}

/// Movimentação simples exibida na lista. `amount_cents` é o impacto assinado
/// na conta principal: negativo para despesa/transferência de saída e para
/// ajustes negativos.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct MovementDto {
    pub id: String,
    pub kind: String,
    pub occurred_on: String,
    pub occurred_time: Option<String>,
    pub manual_period: Option<String>,
    pub period: Option<String>,
    pub description: Option<String>,
    pub primary_account_id: String,
    pub primary_account_name: String,
    pub counterparty_account_id: Option<String>,
    pub counterparty_account_name: Option<String>,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub payment_method: Option<String>,
    #[ts(type = "number")]
    pub amount_cents: i64,
    pub created_at: String,
    pub installment_plan_id: Option<String>,
    #[ts(type = "number | null")]
    pub installment_count: Option<i64>,
    #[ts(type = "number | null")]
    pub first_installment_cents: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct MovementFilter {
    pub account_id: Option<String>,
    pub category_id: Option<String>,
    pub kind: Option<String>,
    pub period: Option<String>,
    pub payment_method: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CreateExpenseInput {
    pub account_id: String,
    pub category_id: String,
    #[ts(type = "number")]
    pub amount_cents: i64,
    pub occurred_on: String,
    pub occurred_time: Option<String>,
    pub manual_period: Option<String>,
    pub description: Option<String>,
    pub payment_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CreateIncomeInput {
    pub account_id: String,
    pub category_id: String,
    #[ts(type = "number")]
    pub amount_cents: i64,
    pub occurred_on: String,
    pub occurred_time: Option<String>,
    pub manual_period: Option<String>,
    pub description: Option<String>,
    pub payment_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CreateTransferInput {
    pub from_account_id: String,
    pub to_account_id: String,
    #[ts(type = "number")]
    pub amount_cents: i64,
    pub occurred_on: String,
    pub occurred_time: Option<String>,
    pub manual_period: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CreateAdjustmentInput {
    pub account_id: String,
    #[ts(type = "number")]
    pub amount_cents: i64,
    pub occurred_on: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct UpdateMovementInput {
    pub kind: String,
    pub primary_account_id: String,
    pub counterparty_account_id: Option<String>,
    pub category_id: Option<String>,
    #[ts(type = "number")]
    pub amount_cents: i64,
    pub occurred_on: String,
    pub occurred_time: Option<String>,
    pub manual_period: Option<String>,
    pub description: Option<String>,
    pub payment_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct StatementLineDto {
    pub entry_id: String,
    pub kind: String,
    pub occurred_on: String,
    pub occurred_time: Option<String>,
    pub period: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub counterparty_account_name: Option<String>,
    #[ts(type = "number")]
    pub amount_cents: i64,
    #[ts(type = "number")]
    pub running_balance_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct AccountStatementDto {
    pub account: AccountDto,
    pub from: Option<String>,
    pub to: Option<String>,
    #[ts(type = "number")]
    pub opening_balance_cents: i64,
    #[ts(type = "number")]
    pub closing_balance_cents: i64,
    pub lines: Vec<StatementLineDto>,
}

/// Situação agregada de um cartão. Valores de dívida são positivos.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CardSummaryDto {
    pub id: String,
    pub name: String,
    pub archived_at: Option<String>,
    #[ts(type = "number | null")]
    pub credit_limit_cents: Option<i64>,
    #[ts(type = "number")]
    pub debt_cents: i64,
    #[ts(type = "number")]
    pub credit_cents: i64,
    #[ts(type = "number | null")]
    pub available_limit_cents: Option<i64>,
    #[ts(type = "number")]
    pub initial_debt_cents: i64,
    #[ts(type = "number")]
    pub overdue_cents: i64,
    #[ts(type = "number")]
    pub current_month_cents: i64,
    #[ts(type = "number")]
    pub future_cents: i64,
    pub over_limit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct InstallmentDto {
    pub id: String,
    pub plan_id: String,
    #[ts(type = "number")]
    pub seq: i64,
    pub due_on: String,
    #[ts(type = "number")]
    pub amount_cents: i64,
    #[ts(type = "number")]
    pub paid_cents: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct InstallmentPlanDto {
    pub id: String,
    pub card_account_id: String,
    pub card_account_name: String,
    pub category_id: String,
    pub category_name: String,
    pub description: Option<String>,
    #[ts(type = "number")]
    pub total_cents: i64,
    #[ts(type = "number")]
    pub installment_count: i64,
    pub first_due_on: String,
    pub period: String,
    pub payment_method: String,
    pub purchased_on: String,
    pub created_at: String,
    #[ts(type = "number")]
    pub paid_cents: i64,
    #[ts(type = "number")]
    pub outstanding_cents: i64,
    pub installments: Vec<InstallmentDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct InstallmentAllocationDto {
    pub installment_id: String,
    pub plan_id: String,
    #[ts(type = "number")]
    pub seq: i64,
    pub due_on: String,
    #[ts(type = "number")]
    pub amount_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CardPaymentDto {
    pub id: String,
    pub card_account_id: String,
    pub card_account_name: String,
    pub bank_account_id: String,
    pub bank_account_name: String,
    pub entry_id: String,
    #[ts(type = "number")]
    pub amount_cents: i64,
    #[ts(type = "number")]
    pub unallocated_cents: i64,
    #[ts(type = "number")]
    pub initial_balance_cents: i64,
    pub occurred_on: String,
    pub occurred_time: Option<String>,
    pub period: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
    pub allocations: Vec<InstallmentAllocationDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CreateCardPurchaseInput {
    pub card_account_id: String,
    pub category_id: String,
    #[ts(type = "number")]
    pub total_cents: i64,
    #[ts(type = "number")]
    pub installment_count: i64,
    pub first_due_on: String,
    pub occurred_on: String,
    pub occurred_time: Option<String>,
    pub manual_period: Option<String>,
    pub description: Option<String>,
    pub payment_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct UpdateInstallmentPlanInput {
    pub category_id: String,
    #[ts(type = "number")]
    pub total_cents: i64,
    #[ts(type = "number")]
    pub installment_count: i64,
    pub first_due_on: String,
    pub period: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CreateCardPaymentInput {
    pub card_account_id: String,
    pub bank_account_id: String,
    #[ts(type = "number")]
    pub amount_cents: i64,
    pub occurred_on: String,
    pub occurred_time: Option<String>,
    pub manual_period: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct UpdateSettingsInput {
    pub backup_retention: u32,
}

// ---------------------------------------------------------------------------
// Relatórios
// ---------------------------------------------------------------------------

/// Filtro comum dos relatórios. `from`/`to` são datas ISO; `period` é o
/// período do dia armazenado no lançamento; `account_id` e `payment_method`
/// só afetam gastos, receitas e fluxo de caixa.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct ReportFilter {
    pub from: Option<String>,
    pub to: Option<String>,
    pub period: Option<String>,
    pub category_id: Option<String>,
    pub account_id: Option<String>,
    pub payment_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CategoryTotalDto {
    pub category_id: Option<String>,
    pub category_name: String,
    #[ts(type = "number")]
    pub total_cents: i64,
    #[ts(type = "number")]
    pub entry_count: i64,
}

/// Relatório por categoria usado tanto para gastos quanto para receitas.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CategoryReportDto {
    pub kind: String,
    pub from: String,
    pub to: String,
    pub period: Option<String>,
    #[ts(type = "number")]
    pub total_cents: i64,
    #[ts(type = "number")]
    pub entry_count: i64,
    pub categories: Vec<CategoryTotalDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CashFlowGroupDto {
    pub label: String,
    pub kind: String,
    #[ts(type = "number")]
    pub inflow_cents: i64,
    #[ts(type = "number")]
    pub outflow_cents: i64,
    #[ts(type = "number")]
    pub net_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CashFlowMonthDto {
    pub month: String,
    #[ts(type = "number")]
    pub inflow_cents: i64,
    #[ts(type = "number")]
    pub outflow_cents: i64,
    #[ts(type = "number")]
    pub net_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CashFlowReportDto {
    pub from: String,
    pub to: String,
    pub period: Option<String>,
    #[ts(type = "number")]
    pub inflow_cents: i64,
    #[ts(type = "number")]
    pub outflow_cents: i64,
    #[ts(type = "number")]
    pub net_cents: i64,
    pub groups: Vec<CashFlowGroupDto>,
    pub months: Vec<CashFlowMonthDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct NetWorthAccountDto {
    pub account_id: String,
    pub name: String,
    pub account_type: Option<String>,
    #[ts(type = "number")]
    pub balance_cents: i64,
}

/// Patrimônio em uma data. `assets_cents` inclui a conta interna
/// "Despesa a reconhecer", exposta separadamente em `deferred_expense_cents`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct NetWorthReportDto {
    pub as_of: String,
    #[ts(type = "number")]
    pub assets_cents: i64,
    #[ts(type = "number")]
    pub liabilities_cents: i64,
    #[ts(type = "number")]
    pub net_worth_cents: i64,
    #[ts(type = "number")]
    pub deferred_expense_cents: i64,
    pub accounts: Vec<NetWorthAccountDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CommitmentDto {
    pub installment_id: String,
    pub plan_id: String,
    pub card_account_id: String,
    pub card_account_name: String,
    pub category_name: String,
    pub description: Option<String>,
    #[ts(type = "number")]
    pub seq: i64,
    #[ts(type = "number")]
    pub installment_count: i64,
    pub due_on: String,
    #[ts(type = "number")]
    pub amount_cents: i64,
    #[ts(type = "number")]
    pub paid_cents: i64,
    #[ts(type = "number")]
    pub outstanding_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CommitmentMonthDto {
    pub month: String,
    #[ts(type = "number")]
    pub total_cents: i64,
    pub items: Vec<CommitmentDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CommitmentsReportDto {
    pub as_of: String,
    /// Parcelas com vencimento estritamente depois desta data são futuras.
    pub start_after: String,
    pub horizon_end: String,
    #[ts(type = "number")]
    pub total_cents: i64,
    #[ts(type = "number")]
    pub overdue_cents: i64,
    pub months: Vec<CommitmentMonthDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct DashboardDto {
    pub as_of: String,
    pub month_from: String,
    pub month_to: String,
    #[ts(type = "number")]
    pub net_worth_cents: i64,
    #[ts(type = "number")]
    pub assets_cents: i64,
    #[ts(type = "number")]
    pub liabilities_cents: i64,
    #[ts(type = "number")]
    pub month_income_cents: i64,
    #[ts(type = "number")]
    pub month_expense_cents: i64,
    #[ts(type = "number")]
    pub month_result_cents: i64,
    #[ts(type = "number")]
    pub card_debt_cents: i64,
    #[ts(type = "number")]
    pub card_credit_cents: i64,
    #[ts(type = "number")]
    pub overdue_installments_cents: i64,
    #[ts(type = "number")]
    pub future_commitments_cents: i64,
    pub top_expenses: Vec<CategoryTotalDto>,
}

// ---------------------------------------------------------------------------
// Dados: backup, restauração e CSV
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct BackupDto {
    pub file_name: String,
    pub path: String,
    #[ts(type = "number")]
    pub size_bytes: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct RestoreResultDto {
    pub backup: BackupDto,
    pub integrity: IntegrityReportDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct ExportCsvInput {
    pub account_id: Option<String>,
    pub category_id: Option<String>,
    pub kind: Option<String>,
    pub period: Option<String>,
    pub payment_method: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct CsvExportDto {
    pub file_name: String,
    pub path: String,
    #[ts(type = "number")]
    pub row_count: i64,
}
