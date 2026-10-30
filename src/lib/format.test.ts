import { describe, expect, it } from "vitest";

import { formatCents } from "./format";

describe("formatCents", () => {
  it("formata centavos inteiros em BRL", () => {
    expect(formatCents(0)).toBe("R$ 0,00");
    expect(formatCents(1)).toBe("R$ 0,01");
    expect(formatCents(123_456)).toBe("R$ 1.234,56");
    expect(formatCents(-1_999)).toBe("-R$ 19,99");
  });

  it("rejeita valores que não são centavos inteiros", () => {
    expect(() => formatCents(1.5)).toThrow(RangeError);
    expect(() => formatCents(Number.NaN)).toThrow(RangeError);
    expect(() => formatCents(Number.POSITIVE_INFINITY)).toThrow(RangeError);
  });
});
