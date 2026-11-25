import type {
  CardSummaryDto,
  CreateCardPaymentInput,
  CreateCardPurchaseInput,
} from "./api/bindings";

/// Compromisso do ciclo atual: dívida inicial + vencidas + vencendo no mês.
/// Parcelas futuras ficam de fora, assim como qualquer crédito em favor do titular.
export function suggestedCardPaymentCents(card: CardSummaryDto): number {
  return card.initial_debt_cents + card.overdue_cents + card.current_month_cents;
}

export type SimpleMovementKind =
  | "expense"
  | "income"
  | "transfer"
  | "adjustment";

export type MovementFormKind =
  | SimpleMovementKind
  | "card_purchase"
  | "card_payment";

export type MovementDraft = {
  kind: SimpleMovementKind;
  primaryAccountId: string;
  counterpartyAccountId: string;
  categoryId: string;
  amountCents: number;
  occurredOn: string;
  occurredTime: string;
  manualPeriod: string;
  description: string;
  paymentMethod: string;
};

export type SaveMovementRequest = {
  mode: "create" | "edit";
  id: string | null;
  draft: MovementDraft;
};

export type MovementFormRequest =
  | ({ form: "movement" } & SaveMovementRequest)
  | { form: "card_purchase"; input: CreateCardPurchaseInput }
  | { form: "card_payment"; input: CreateCardPaymentInput };
