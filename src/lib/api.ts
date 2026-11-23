import { invoke } from "@tauri-apps/api/core";

import type {
  AccountDto,
  AccountStatementDto,
  BackupDto,
  BootstrapDto,
  CardPaymentDto,
  CardSummaryDto,
  CashFlowReportDto,
  CategoryDto,
  CategoryReportDto,
  CommitmentsReportDto,
  CreateAccountInput,
  CreateAdjustmentInput,
  CreateCardPaymentInput,
  CreateCardPurchaseInput,
  CreateCategoryInput,
  CreateExpenseInput,
  CreateIncomeInput,
  CreateSavingsYieldInput,
  CreateTransferInput,
  CsvExportDto,
  DashboardDto,
  ExpenseTrendDto,
  ExportCsvInput,
  InstallmentPlanDto,
  IntegrityReportDto,
  MovementDto,
  MovementFilter,
  NetWorthHistoryDto,
  NetWorthReportDto,
  PeriodSpendingReportDto,
  ReportFilter,
  RestoreResultDto,
  ReverseSavingsYieldInput,
  SavingsOverviewDto,
  SavingsYieldDto,
  SettingsDto,
  UpdateInstallmentPlanInput,
  UpdateMovementInput,
  UpdateSettingsInput,
} from "./api/bindings";

export function getBootstrap(): Promise<BootstrapDto> {
  return invoke<BootstrapDto>("get_bootstrap");
}

export function runIntegrityCheck(): Promise<IntegrityReportDto> {
  return invoke<IntegrityReportDto>("run_integrity_check");
}

export function listAccounts(includeArchived = false): Promise<AccountDto[]> {
  return invoke<AccountDto[]>("list_accounts", { includeArchived });
}

export function createAccount(input: CreateAccountInput): Promise<AccountDto> {
  return invoke<AccountDto>("create_account", { input });
}

export function updateAccount(
  id: string,
  name: string,
  creditLimitCents: number | null = null,
): Promise<AccountDto> {
  return invoke<AccountDto>("update_account", {
    id,
    name,
    creditLimitCents,
  });
}

export function setAccountArchived(
  id: string,
  archived: boolean,
): Promise<AccountDto> {
  return invoke<AccountDto>("set_account_archived", { id, archived });
}

export function deleteAccount(id: string): Promise<void> {
  return invoke<void>("delete_account", { id });
}

export function getSavingsOverview(
  accountId: string,
): Promise<SavingsOverviewDto> {
  return invoke<SavingsOverviewDto>("get_savings_overview", { accountId });
}

export function createSavingsYield(
  input: CreateSavingsYieldInput,
): Promise<SavingsYieldDto> {
  return invoke<SavingsYieldDto>("create_savings_yield", { input });
}

export function reverseSavingsYield(
  input: ReverseSavingsYieldInput,
): Promise<SavingsYieldDto> {
  return invoke<SavingsYieldDto>("reverse_savings_yield", { input });
}

export function listCategories(
  includeArchived = false,
): Promise<CategoryDto[]> {
  return invoke<CategoryDto[]>("list_categories", { includeArchived });
}

export function createCategory(
  input: CreateCategoryInput,
): Promise<CategoryDto> {
  return invoke<CategoryDto>("create_category", { input });
}

export function updateCategory(
  id: string,
  name: string,
  icon: string,
): Promise<CategoryDto> {
  return invoke<CategoryDto>("update_category", { id, name, icon });
}

export function setCategoryArchived(
  id: string,
  archived: boolean,
): Promise<CategoryDto> {
  return invoke<CategoryDto>("set_category_archived", { id, archived });
}

export function deleteCategory(id: string): Promise<void> {
  return invoke<void>("delete_category", { id });
}

export function listMovements(
  filter?: MovementFilter,
): Promise<MovementDto[]> {
  return invoke<MovementDto[]>("list_movements", { filter: filter ?? null });
}

export function createExpense(
  input: CreateExpenseInput,
): Promise<MovementDto> {
  return invoke<MovementDto>("create_expense", { input });
}

export function createIncome(input: CreateIncomeInput): Promise<MovementDto> {
  return invoke<MovementDto>("create_income", { input });
}

export function createTransfer(
  input: CreateTransferInput,
): Promise<MovementDto> {
  return invoke<MovementDto>("create_transfer", { input });
}

export function createAdjustment(
  input: CreateAdjustmentInput,
): Promise<MovementDto> {
  return invoke<MovementDto>("create_adjustment", { input });
}

export function updateMovement(
  id: string,
  input: UpdateMovementInput,
): Promise<MovementDto> {
  return invoke<MovementDto>("update_movement", { id, input });
}

export function deleteMovement(id: string): Promise<void> {
  return invoke<void>("delete_movement", { id });
}

export function getAccountStatement(
  accountId: string,
  from: string | null = null,
  to: string | null = null,
): Promise<AccountStatementDto> {
  return invoke<AccountStatementDto>("get_account_statement", {
    accountId,
    from,
    to,
  });
}

export function listCards(): Promise<CardSummaryDto[]> {
  return invoke<CardSummaryDto[]>("list_cards");
}

export function getCard(cardAccountId: string): Promise<CardSummaryDto> {
  return invoke<CardSummaryDto>("get_card", { cardAccountId });
}

export function createCardPurchase(
  input: CreateCardPurchaseInput,
): Promise<InstallmentPlanDto> {
  return invoke<InstallmentPlanDto>("create_card_purchase", { input });
}

export function listInstallmentPlans(
  cardAccountId: string | null = null,
): Promise<InstallmentPlanDto[]> {
  return invoke<InstallmentPlanDto[]>("list_installment_plans", {
    cardAccountId,
  });
}

export function getInstallmentPlan(
  planId: string,
): Promise<InstallmentPlanDto> {
  return invoke<InstallmentPlanDto>("get_installment_plan", { planId });
}

export function deleteInstallmentPlan(planId: string): Promise<void> {
  return invoke<void>("delete_installment_plan", { planId });
}

export function updateInstallmentPlan(
  planId: string,
  input: UpdateInstallmentPlanInput,
): Promise<InstallmentPlanDto> {
  return invoke<InstallmentPlanDto>("update_installment_plan", {
    planId,
    input,
  });
}

export function createCardPayment(
  input: CreateCardPaymentInput,
): Promise<CardPaymentDto> {
  return invoke<CardPaymentDto>("create_card_payment", { input });
}

export function listCardPayments(
  cardAccountId: string | null = null,
): Promise<CardPaymentDto[]> {
  return invoke<CardPaymentDto[]>("list_card_payments", { cardAccountId });
}

export function deleteCardPayment(paymentId: string): Promise<void> {
  return invoke<void>("delete_card_payment", { paymentId });
}

export function getDashboard(): Promise<DashboardDto> {
  return invoke<DashboardDto>("get_dashboard");
}

export function getSpendingReport(
  filter: ReportFilter,
): Promise<CategoryReportDto> {
  return invoke<CategoryReportDto>("get_spending_report", { filter });
}

export function getIncomeReport(
  filter: ReportFilter,
): Promise<CategoryReportDto> {
  return invoke<CategoryReportDto>("get_income_report", { filter });
}

export function getCashFlowReport(
  filter: ReportFilter,
): Promise<CashFlowReportDto> {
  return invoke<CashFlowReportDto>("get_cash_flow_report", { filter });
}

export function getNetWorthReport(
  asOf: string | null = null,
): Promise<NetWorthReportDto> {
  return invoke<NetWorthReportDto>("get_net_worth_report", { asOf });
}

export function getPeriodSpendingReport(
  asOf: string | null = null,
): Promise<PeriodSpendingReportDto> {
  return invoke<PeriodSpendingReportDto>("get_period_spending_report", {
    asOf,
  });
}

export function getExpenseTrendReport(
  asOf: string | null = null,
): Promise<ExpenseTrendDto> {
  return invoke<ExpenseTrendDto>("get_expense_trend_report", { asOf });
}

export function getNetWorthHistoryReport(
  asOf: string | null = null,
): Promise<NetWorthHistoryDto> {
  return invoke<NetWorthHistoryDto>("get_net_worth_history_report", {
    asOf,
  });
}

export function getCommitmentsReport(
  asOf: string | null = null,
  months: number | null = null,
): Promise<CommitmentsReportDto> {
  return invoke<CommitmentsReportDto>("get_commitments_report", {
    asOf,
    months,
  });
}

export function createBackup(): Promise<BackupDto> {
  return invoke<BackupDto>("create_backup");
}

export function listBackups(): Promise<BackupDto[]> {
  return invoke<BackupDto[]>("list_backups");
}

export function restoreBackup(fileName: string): Promise<RestoreResultDto> {
  return invoke<RestoreResultDto>("restore_backup", { fileName });
}

export function exportCsv(input: ExportCsvInput): Promise<CsvExportDto> {
  return invoke<CsvExportDto>("export_csv", { input });
}

export function getSettings(): Promise<SettingsDto> {
  return invoke<SettingsDto>("get_settings");
}

export function updateSettings(
  input: UpdateSettingsInput,
): Promise<SettingsDto> {
  return invoke<SettingsDto>("update_settings", { input });
}

export function openAppFolder(folder: "data" | "logs"): Promise<void> {
  return invoke<void>("open_app_folder", { folder });
}
