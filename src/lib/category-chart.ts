import type { CategoryTotalDto } from "./api/bindings";

export type CategoryBarDatum = {
  key: string;
  label: string;
  totalCents: number;
  /** Percentual sobre o total das categorias, sem arredondar. */
  percent: number;
  grouped: boolean;
};

/**
 * Ordena do maior para o menor e agrupa a cauda em "Outras" quando passa do
 * limite. O percentual usa o total completo, então as barras sempre somam 100%.
 */
export function buildCategoryBars(
  categories: CategoryTotalDto[],
  limit = 8,
): CategoryBarDatum[] {
  if (!Number.isInteger(limit) || limit < 1) {
    throw new RangeError("buildCategoryBars exige limite inteiro >= 1.");
  }

  const sorted = [...categories].sort((a, b) => b.total_cents - a.total_cents);
  const total = sorted.reduce((sum, item) => sum + item.total_cents, 0);
  const bar = (
    key: string,
    label: string,
    totalCents: number,
    grouped: boolean,
  ): CategoryBarDatum => ({
    key,
    label,
    totalCents,
    percent: total > 0 ? (totalCents / total) * 100 : 0,
    grouped,
  });

  const bars = sorted
    .slice(0, limit)
    .map((item) =>
      bar(
        item.category_id ?? "__sem_categoria__",
        item.category_name,
        item.total_cents,
        false,
      ),
    );

  const tail = sorted.slice(limit);
  if (tail.length > 0) {
    bars.push(
      bar(
        "__outras__",
        `Outras (${tail.length})`,
        tail.reduce((sum, item) => sum + item.total_cents, 0),
        true,
      ),
    );
  }

  return bars;
}

/** Percentual pt-BR com uma casa: `12,4%`. */
export function formatPercent(percent: number): string {
  return `${percent.toFixed(1).replace(".", ",")}%`;
}
