import {
  createAdjustment,
  createCardPayment,
  createCardPurchase,
  createExpense,
  createIncome,
  createTransfer,
  updateMovement,
} from "./api";
import type { MovementDto } from "./api/bindings";
import type { MovementFormRequest, SaveMovementRequest } from "./movement-form";

/** Despacha o formulário único para o comando de backend correspondente. */
export function submitMovementForm(
  request: MovementFormRequest,
): Promise<unknown> {
  if (request.form === "card_purchase") {
    return createCardPurchase(request.input);
  }
  if (request.form === "card_payment") {
    return createCardPayment(request.input);
  }
  return submitMovement(request);
}

function submitMovement(request: SaveMovementRequest): Promise<MovementDto> {
  const { draft } = request;
  const shared = {
    account_id: draft.primaryAccountId,
    category_id: draft.categoryId || null,
    amount_cents: draft.amountCents,
    occurred_on: draft.occurredOn,
    occurred_time: draft.occurredTime || null,
    manual_period: draft.manualPeriod || null,
    description: draft.description.trim() || null,
  };

  if (request.mode === "edit") {
    if (!request.id) {
      throw new Error("Movimentação sem identificador.");
    }

    return updateMovement(request.id, {
      kind: draft.kind,
      primary_account_id: draft.primaryAccountId,
      counterparty_account_id: draft.counterpartyAccountId || null,
      category_id: draft.categoryId || null,
      amount_cents: draft.amountCents,
      occurred_on: draft.occurredOn,
      occurred_time: draft.occurredTime || null,
      manual_period: draft.manualPeriod || null,
      description: draft.description.trim() || null,
      payment_method: draft.paymentMethod || null,
    });
  }

  switch (draft.kind) {
    case "expense":
      if (!shared.category_id || !draft.paymentMethod) {
        throw new Error("Categoria e método são obrigatórios.");
      }
      return createExpense({
        ...shared,
        category_id: shared.category_id,
        payment_method: draft.paymentMethod,
      });
    case "income":
      if (!shared.category_id || !draft.paymentMethod) {
        throw new Error("Categoria e método são obrigatórios.");
      }
      return createIncome({
        ...shared,
        category_id: shared.category_id,
        payment_method: draft.paymentMethod,
      });
    case "transfer":
      return createTransfer({
        from_account_id: draft.primaryAccountId,
        to_account_id: draft.counterpartyAccountId,
        amount_cents: draft.amountCents,
        occurred_on: draft.occurredOn,
        occurred_time: draft.occurredTime || null,
        manual_period: draft.manualPeriod || null,
        description: draft.description.trim() || null,
      });
    case "adjustment":
      return createAdjustment({
        account_id: draft.primaryAccountId,
        amount_cents: draft.amountCents,
        occurred_on: draft.occurredOn,
        description: draft.description.trim() || null,
      });
  }
}
