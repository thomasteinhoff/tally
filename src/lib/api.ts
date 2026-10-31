import { invoke } from "@tauri-apps/api/core";

import type {
  AccountDto,
  AccountStatementDto,
  BootstrapDto,
  CategoryDto,
  CreateAccountInput,
  CreateAdjustmentInput,
  CreateCategoryInput,
  CreateExpenseInput,
  CreateIncomeInput,
  CreateTransferInput,
  IntegrityReportDto,
  MovementDto,
  MovementFilter,
  UpdateMovementInput,
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

export function updateAccount(id: string, name: string): Promise<AccountDto> {
  return invoke<AccountDto>("update_account", { id, name });
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

export function updateCategory(id: string, name: string): Promise<CategoryDto> {
  return invoke<CategoryDto>("update_category", { id, name });
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
