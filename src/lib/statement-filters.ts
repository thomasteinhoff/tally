import type { StatementLineDto } from "./api/bindings";

export type StatementLineFilters = {
  categoryId: string;
  kind: string;
  period: string;
};

export function hasStatementLineFilters(
  filters: StatementLineFilters,
): boolean {
  return Boolean(filters.categoryId || filters.kind || filters.period);
}

export function filterStatementLines(
  lines: readonly StatementLineDto[],
  filters: StatementLineFilters,
): StatementLineDto[] {
  return lines.filter(
    (line) =>
      (!filters.categoryId || line.category_id === filters.categoryId) &&
      (!filters.kind || line.kind === filters.kind) &&
      (!filters.period || line.period === filters.period),
  );
}
