import { describe, expect, it } from "vitest";

import type { StatementLineDto } from "./api/bindings";
import {
  filterStatementLines,
  hasStatementLineFilters,
} from "./statement-filters";

function line(overrides: Partial<StatementLineDto>): StatementLineDto {
  return {
    entry_id: "entry",
    kind: "expense",
    occurred_on: "2026-10-31",
    occurred_time: "12:00",
    period: "tarde",
    description: null,
    category_id: null,
    category_name: null,
    counterparty_account_name: null,
    amount_cents: -100,
    running_balance_cents: 900,
    ...overrides,
  };
}

describe("filtros do extrato", () => {
  it("combina categoria, tipo e período sem alterar os saldos", () => {
    const lines = [
      line({ entry_id: "a", category_id: "market" }),
      line({ entry_id: "b", category_id: "market", kind: "income" }),
      line({ entry_id: "c", category_id: "salary", kind: "income" }),
      line({ entry_id: "d", category_id: null, kind: "opening_balance" }),
    ];

    expect(
      filterStatementLines(lines, {
        categoryId: "market",
        kind: "expense",
        period: "tarde",
      }).map((item) => item.entry_id),
    ).toEqual(["a"]);

    expect(
      filterStatementLines(lines, {
        categoryId: "",
        kind: "income",
        period: "",
      }).map((item) => item.entry_id),
    ).toEqual(["b", "c"]);

    expect(filterStatementLines(lines, {
      categoryId: "",
      kind: "",
      period: "",
    })[0].running_balance_cents).toBe(900);
  });

  it("identifica quando há filtros de linha ativos", () => {
    expect(
      hasStatementLineFilters({ categoryId: "", kind: "", period: "" }),
    ).toBe(false);
    expect(
      hasStatementLineFilters({ categoryId: "market", kind: "", period: "" }),
    ).toBe(true);
  });
});
