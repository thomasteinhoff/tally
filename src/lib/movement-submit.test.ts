import { beforeEach, describe, expect, it, vi } from "vitest";

const api = vi.hoisted(() => ({
  createAdjustment: vi.fn(),
  createCardPayment: vi.fn(),
  createCardPurchase: vi.fn(),
  createExpense: vi.fn(),
  createIncome: vi.fn(),
  createTransfer: vi.fn(),
  updateMovement: vi.fn(),
}));

vi.mock("./api", () => api);

import type { CreateCardPaymentInput, CreateCardPurchaseInput } from "./api/bindings";
import type { MovementDraft } from "./movement-form";
import { submitMovementForm } from "./movement-submit";

const draft: MovementDraft = {
  kind: "expense",
  primaryAccountId: "account-1",
  counterpartyAccountId: "",
  categoryId: "category-1",
  amountCents: 12_345,
  occurredOn: "2026-11-11",
  occurredTime: "09:30",
  manualPeriod: "",
  description: "  mercado  ",
  paymentMethod: "pix",
};

const purchase: CreateCardPurchaseInput = {
  card_account_id: "card-1",
  category_id: "category-1",
  total_cents: 120_000,
  installment_count: 3,
  first_due_on: "2026-12-01",
  occurred_on: "2026-11-11",
  occurred_time: null,
  manual_period: "manha",
  description: null,
  payment_method: "credit",
};

const payment: CreateCardPaymentInput = {
  card_account_id: "card-1",
  bank_account_id: "account-1",
  amount_cents: 50_000,
  occurred_on: "2026-11-11",
  occurred_time: "10:00",
  manual_period: null,
  description: null,
};

describe("submitMovementForm", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("despacha compra e pagamento de cartão para os comandos próprios", async () => {
    await submitMovementForm({ form: "card_purchase", input: purchase });
    await submitMovementForm({ form: "card_payment", input: payment });

    expect(api.createCardPurchase).toHaveBeenCalledWith(purchase);
    expect(api.createCardPayment).toHaveBeenCalledWith(payment);
  });

  it("cria despesa simples com descrição normalizada", async () => {
    await submitMovementForm({
      form: "movement",
      mode: "create",
      id: null,
      draft,
    });

    expect(api.createExpense).toHaveBeenCalledWith({
      account_id: "account-1",
      category_id: "category-1",
      amount_cents: 12_345,
      occurred_on: "2026-11-11",
      occurred_time: "09:30",
      manual_period: null,
      description: "mercado",
      payment_method: "pix",
    });
  });

  it("edita movimentação pelo comando de atualização", async () => {
    await submitMovementForm({
      form: "movement",
      mode: "edit",
      id: "movement-1",
      draft,
    });

    expect(api.updateMovement).toHaveBeenCalledWith(
      "movement-1",
      expect.objectContaining({
        kind: "expense",
        amount_cents: 12_345,
        description: "mercado",
      }),
    );
    expect(api.createExpense).not.toHaveBeenCalled();
  });
});
