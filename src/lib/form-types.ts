export type AccountFormRequest = {
  mode: "create" | "edit";
  id: string | null;
  name: string;
  accountType: string;
  openingBalanceCents: number;
  creditLimitCents: number | null;
};

export type CategoryFormRequest = {
  mode: "create" | "edit";
  id: string | null;
  name: string;
  kind: string;
};
