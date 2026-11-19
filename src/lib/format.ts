const thousands = /\B(?=(\d{3})+(?!\d))/g;

function groupThousands(value: string): string {
  return value.replace(thousands, ".");
}

/**
 * Formata centavos inteiros como BRL usando apenas manipulação de string.
 * Nenhuma regra financeira atravessa a fronteira da UI.
 */
export function formatCents(cents: number): string {
  if (!Number.isSafeInteger(cents)) {
    throw new RangeError("formatCents exige centavos inteiros e seguros.");
  }

  const sign = cents < 0 ? "-" : "";
  const raw = String(Math.abs(cents)).padStart(3, "0");
  const reais = raw.slice(0, -2);
  const centavos = raw.slice(-2);

  return `${sign}R$ ${groupThousands(reais)},${centavos}`;
}

/** Disponível e limite no formato `R$ 1.400,00 / R$ 2.000,00`. */
export function formatLimit(
  availableCents: number | null,
  totalCents: number | null,
): string {
  if (totalCents === null) {
    return "—";
  }
  const available =
    availableCents === null ? "—" : formatCents(availableCents);
  return `${available} / ${formatCents(totalCents)}`;
}

/** Valor sem sinal para campos de formulário. */
export function formatAmountInput(cents: number): string {
  const raw = String(Math.abs(cents)).padStart(3, "0");
  const reais = raw.slice(0, -2);
  const centavos = raw.slice(-2);
  return `${cents < 0 ? "-" : ""}${groupThousands(reais)},${centavos}`;
}

/**
 * O DTO assina o impacto na conta principal; o formulário só aceita valor
 * assinado em ajustes. Despesa, receita e transferência usam a magnitude.
 */
export function movementAmountForInput(kind: string, cents: number): number {
  return kind === "adjustment" ? cents : Math.abs(cents);
}

/**
 * Converte texto em centavos sem ponto flutuante.
 * Aceita `1.234,56`, `1234,56`, `1234.56` e `1234`.
 */
export function parseAmountToCents(raw: string): number | null {
  let value = raw.trim().replace(/\s+/g, "").replace(/^R\$/i, "");
  if (!value) {
    return null;
  }

  let negative = false;
  if (value.startsWith("-")) {
    negative = true;
    value = value.slice(1);
  } else if (value.startsWith("+")) {
    value = value.slice(1);
  }

  let reais: string;
  let decimals: string;

  if (value.includes(",")) {
    const parts = value.split(",");
    if (parts.length !== 2) {
      return null;
    }
    reais = parts[0].replace(/\./g, "") || "0";
    decimals = parts[1];
  } else if (/^\d{1,3}(\.\d{3})+$/.test(value)) {
    reais = value.replace(/\./g, "");
    decimals = "";
  } else if (/^\d+\.\d{1,2}$/.test(value)) {
    [reais, decimals] = value.split(".");
  } else {
    reais = value;
    decimals = "";
  }

  if (!/^\d+$/.test(reais) || !/^\d{0,2}$/.test(decimals)) {
    return null;
  }

  const cents =
    Number(reais) * 100 + Number(decimals.padEnd(2, "0") || "0");

  if (!Number.isSafeInteger(cents)) {
    return null;
  }

  return negative && cents !== 0 ? -cents : cents;
}

export function formatDate(isoDate: string): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(isoDate);
  return match ? `${match[3]}/${match[2]}/${match[1]}` : isoDate;
}

export function formatMonth(isoMonth: string): string {
  const match = /^(\d{4})-(\d{2})$/.exec(isoMonth);
  return match ? `${match[2]}/${match[1]}` : isoMonth;
}

/** Dias ativos no período, com plural correto. */
export function formatDays(count: number): string {
  return `${count} ${count === 1 ? "dia ativo" : "dias ativos"}`;
}

export function todayIso(): string {
  const now = new Date();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${now.getFullYear()}-${month}-${day}`;
}

export function currentTime(): string {
  const now = new Date();
  return `${String(now.getHours()).padStart(2, "0")}:${String(
    now.getMinutes(),
  ).padStart(2, "0")}`;
}

/** Normaliza horários 24h digitados ("9", "930", "9:30") para "HH:MM". */
export function normalizeTimeInput(raw: string): string | null {
  const value = raw.trim();
  if (!value) return "";
  let hoursText: string;
  let minutesText: string;
  if (value.includes(":")) {
    const parts = value.split(":");
    if (parts.length !== 2) return null;
    [hoursText, minutesText] = parts;
  } else if (/^\d{1,2}$/.test(value)) {
    hoursText = value;
    minutesText = "0";
  } else if (/^\d{3,4}$/.test(value)) {
    hoursText = value.slice(0, -2);
    minutesText = value.slice(-2);
  } else {
    return null;
  }
  if (!/^\d{1,2}$/.test(hoursText) || !/^\d{1,2}$/.test(minutesText)) {
    return null;
  }
  const hours = Number(hoursText);
  const minutes = Number(minutesText);
  if (hours > 23 || minutes > 59) return null;
  return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`;
}

const periodLabels: Record<string, string> = {
  madrugada: "Madrugada",
  manha: "Manhã",
  tarde: "Tarde",
  noite: "Noite",
  custom: "Personalizado",
};

const kindLabels: Record<string, string> = {
  expense: "Despesa",
  income: "Receita",
  transfer: "Transferência",
  adjustment: "Ajuste",
  opening_balance: "Saldo inicial",
  card_purchase: "Compra no cartão",
  card_payment: "Pagamento de cartão",
};

const paymentLabels: Record<string, string> = {
  pix: "Pix",
  debit: "Débito",
  credit: "Crédito",
  subscription_credit: "Crédito recorrente",
  cash: "Dinheiro",
};

const accountTypeLabels: Record<string, string> = {
  bank: "Conta bancária",
  cash: "Dinheiro",
  savings: "Poupança",
  investment: "Investimento",
  digital_wallet: "Carteira digital",
  other_asset: "Outro ativo",
  credit_card: "Cartão de crédito",
};

const installmentStatusLabels: Record<string, string> = {
  open: "Em aberto",
  partial: "Parcial",
  paid: "Paga",
  overdue: "Vencida",
};

export function installmentStatusLabel(status: string): string {
  return installmentStatusLabels[status] ?? status;
}

export function periodLabel(period: string | null): string {
  return period ? (periodLabels[period] ?? period) : "—";
}

const periodRanges: Record<string, string> = {
  madrugada: "Madrugada (00:00–04:59)",
  manha: "Manhã (05:00–11:59)",
  tarde: "Tarde (12:00–18:59)",
  noite: "Noite (19:00–23:59)",
};

export function periodRangeLabel(period: string | null): string {
  return period ? (periodRanges[period] ?? periodLabel(period)) : "—";
}

export function kindLabel(kind: string): string {
  return kindLabels[kind] ?? kind;
}

export function paymentLabel(method: string | null): string {
  return method ? (paymentLabels[method] ?? method) : "—";
}

export function accountTypeLabel(type: string | null): string {
  return type ? (accountTypeLabels[type] ?? type) : "—";
}
