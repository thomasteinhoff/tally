import { describe, expect, it } from "vitest";

import { buildCategoryBars, formatPercent } from "./category-chart";

const category = (id: string, total_cents: number) => ({
  category_id: id,
  category_name: id,
  total_cents,
  entry_count: 1,
});

describe("buildCategoryBars", () => {
  it("ordena do maior para o menor", () => {
    const bars = buildCategoryBars([
      category("mercado", 1000),
      category("aluguel", 5000),
      category("lazer", 3000),
    ]);

    expect(bars.map((bar) => bar.label)).toEqual(["aluguel", "lazer", "mercado"]);
  });

  it("agrupa a cauda em Outras sem perder valor", () => {
    const bars = buildCategoryBars(
      [
        category("a", 500),
        category("b", 300),
        category("c", 150),
        category("d", 50),
      ],
      2,
    );

    expect(bars).toHaveLength(3);
    expect(bars[2]).toMatchObject({
      label: "Outras (2)",
      totalCents: 200,
      grouped: true,
    });
    expect(bars.reduce((sum, bar) => sum + bar.totalCents, 0)).toBe(1000);
    expect(bars.reduce((sum, bar) => sum + bar.percent, 0)).toBeCloseTo(100);
  });

  it("retorna vazio sem categorias e não divide por zero", () => {
    expect(buildCategoryBars([])).toEqual([]);
    const zero = buildCategoryBars([category("a", 0)]);
    expect(zero[0].percent).toBe(0);
  });

  it("rejeita limite inválido", () => {
    expect(() => buildCategoryBars([], 0)).toThrow(RangeError);
  });
});

describe("formatPercent", () => {
  it("usa vírgula e uma casa decimal", () => {
    expect(formatPercent(12.44)).toBe("12,4%");
    expect(formatPercent(100)).toBe("100,0%");
  });
});
