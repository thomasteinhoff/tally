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
