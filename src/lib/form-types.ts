export type AccountFormRequest = {
  mode: "create" | "edit";
  id: string | null;
  name: string;
  accountType: string;
  openingBalanceCents: number;
  creditLimitCents: number | null;
  isSavings: boolean;
  yieldMode: string | null;
  cdiPercent: number | null;
};

export type CategoryFormRequest = {
  mode: "create" | "edit";
  id: string | null;
  name: string;
  kind: string;
  icon: string;
};
