import { describe, expect, it } from "vitest";

import type { CardSummaryDto } from "./api/bindings";
import { suggestedCardPaymentCents } from "./movement-form";

function card(values: Partial<CardSummaryDto> = {}): CardSummaryDto {
  return {
    id: "card",
    name: "Cartão",
    archived_at: null,
    credit_limit_cents: 100_000,
    debt_cents: 0,
    credit_cents: 0,
    available_limit_cents: 100_000,
    initial_debt_cents: 0,
    overdue_cents: 0,
    current_month_cents: 0,
    future_cents: 0,
    over_limit: false,
    ...values,
  };
}

describe("suggestedCardPaymentCents", () => {
  it("soma dívida inicial, vencidas e parcelas do mês", () => {
    expect(
      suggestedCardPaymentCents(
        card({
          initial_debt_cents: 5_000,
          overdue_cents: 30_000,
          current_month_cents: 20_000,
          future_cents: 90_000,
          debt_cents: 145_000,
        }),
      ),
    ).toBe(55_000);
  });

  it("não inclui parcelas futuras", () => {
    expect(
      suggestedCardPaymentCents(card({ future_cents: 90_000, debt_cents: 90_000 })),
    ).toBe(0);
  });

  it("não desconta crédito em favor do titular do valor sugerido", () => {
    expect(
      suggestedCardPaymentCents(
        card({
          initial_debt_cents: 5_000,
          overdue_cents: 30_000,
          current_month_cents: 20_000,
          credit_cents: 40_000,
          debt_cents: 15_000,
        }),
      ),
    ).toBe(55_000);
  });
});
