export type SimpleMovementKind =
  | "expense"
  | "income"
  | "transfer"
  | "adjustment";

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
