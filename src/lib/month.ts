const MONTH_PATTERN = /^(\d{4})-(\d{2})$/;

function parseMonth(isoMonth: string): [year: number, month: number] {
  const match = MONTH_PATTERN.exec(isoMonth);
  if (!match) {
    throw new RangeError(`Mês inválido: ${isoMonth}`);
  }
  const year = Number(match[1]);
  const month = Number(match[2]);
  if (month < 1 || month > 12) {
    throw new RangeError(`Mês inválido: ${isoMonth}`);
  }
  return [year, month];
}

/** Mês corrente no fuso local, no formato `YYYY-MM`. */
export function currentMonthIso(): string {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}`;
}

/** Desloca meses preservando a virada de ano (aceita valores negativos). */
export function shiftMonth(isoMonth: string, delta: number): string {
  if (!Number.isInteger(delta)) {
    throw new RangeError("shiftMonth exige deslocamento inteiro.");
  }
  const [year, month] = parseMonth(isoMonth);
  const index = year * 12 + (month - 1) + delta;
  const shiftedYear = Math.floor(index / 12);
  const shiftedMonth = (((index % 12) + 12) % 12) + 1;
  return `${shiftedYear}-${String(shiftedMonth).padStart(2, "0")}`;
}

/** Primeiro e último dia do mês, em ISO. */
export function monthBounds(isoMonth: string): { from: string; to: string } {
  const [year, month] = parseMonth(isoMonth);
  const lastDay = new Date(Date.UTC(year, month, 0)).getUTCDate();
  return {
    from: `${isoMonth}-01`,
    to: `${isoMonth}-${String(lastDay).padStart(2, "0")}`,
  };
}
