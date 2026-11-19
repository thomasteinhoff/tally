import { describe, expect, it } from "vitest";

import {
  formatAmountInput,
  formatCents,
  formatDate,
  formatDays,
  formatLimit,
  formatMonth,
  movementAmountForInput,
  normalizeTimeInput,
  parseAmountToCents,
  periodLabel,
  periodRangeLabel,
} from "./format";

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

describe("parseAmountToCents", () => {
  it("interpreta formatos brasileiros sem ponto flutuante", () => {
    expect(parseAmountToCents("0,00")).toBe(0);
    expect(parseAmountToCents("1,2")).toBe(120);
    expect(parseAmountToCents("1.234,56")).toBe(123_456);
    expect(parseAmountToCents("1234,56")).toBe(123_456);
    expect(parseAmountToCents("1234.56")).toBe(123_456);
    expect(parseAmountToCents("1.234")).toBe(123_400);
    expect(parseAmountToCents(" R$ 1.234,56 ")).toBe(123_456);
    expect(parseAmountToCents("-1.234,56")).toBe(-123_456);
    expect(parseAmountToCents("+12")).toBe(1_200);
  });

  it("recusa entradas inválidas", () => {
    expect(parseAmountToCents("")).toBeNull();
    expect(parseAmountToCents("abc")).toBeNull();
    expect(parseAmountToCents("1,234")).toBeNull();
    expect(parseAmountToCents("1,2,3")).toBeNull();
    expect(parseAmountToCents("-")).toBeNull();
  });
});

describe("normalizeTimeInput", () => {
  it("normaliza horários 24h digitados", () => {
    expect(normalizeTimeInput("")).toBe("");
    expect(normalizeTimeInput("9")).toBe("09:00");
    expect(normalizeTimeInput("930")).toBe("09:30");
    expect(normalizeTimeInput("0930")).toBe("09:30");
    expect(normalizeTimeInput("9:5")).toBe("09:05");
    expect(normalizeTimeInput("15:30")).toBe("15:30");
    expect(normalizeTimeInput("23:59")).toBe("23:59");
  });

  it("recusa horários inválidos", () => {
    expect(normalizeTimeInput("24:00")).toBeNull();
    expect(normalizeTimeInput("12:60")).toBeNull();
    expect(normalizeTimeInput("abc")).toBeNull();
  });
});

describe("formatação de apoio", () => {
  it("converte centavos para campo de formulário", () => {
    expect(formatAmountInput(0)).toBe("0,00");
    expect(formatAmountInput(123_456)).toBe("1.234,56");
    expect(formatAmountInput(-1_234)).toBe("-12,34");
  });

  it("formata datas ISO e períodos do domínio", () => {
    expect(formatDate("2026-10-31")).toBe("31/10/2026");
    expect(formatDate("31/10/2026")).toBe("31/10/2026");
    expect(formatMonth("2026-10")).toBe("10/2026");
    expect(formatMonth("2026-10-31")).toBe("2026-10-31");
    expect(formatDays(0)).toBe("0 dias ativos");
    expect(formatDays(1)).toBe("1 dia ativo");
    expect(formatDays(4)).toBe("4 dias ativos");
    expect(periodLabel("madrugada")).toBe("Madrugada");
    expect(periodLabel(null)).toBe("—");
  });

  it("descreve as quatro faixas com seus intervalos", () => {
    expect(periodRangeLabel("madrugada")).toBe("Madrugada (00:00–04:59)");
    expect(periodRangeLabel("manha")).toBe("Manhã (05:00–11:59)");
    expect(periodRangeLabel("tarde")).toBe("Tarde (12:00–18:59)");
    expect(periodRangeLabel("noite")).toBe("Noite (19:00–23:59)");
    expect(periodRangeLabel("custom")).toBe("Personalizado");
    expect(periodRangeLabel(null)).toBe("—");
  });

  it("combina disponível e limite do cartão", () => {
    expect(formatLimit(140_000, 200_000)).toBe(
      "R$ 1.400,00 / R$ 2.000,00",
    );
    expect(formatLimit(-1_000, 200_000)).toBe("-R$ 10,00 / R$ 2.000,00");
    expect(formatLimit(null, 200_000)).toBe("— / R$ 2.000,00");
    expect(formatLimit(140_000, null)).toBe("—");
  });

  it("usa magnitude no formulário exceto para ajustes assinados", () => {
    expect(movementAmountForInput("expense", -25_000)).toBe(25_000);
    expect(movementAmountForInput("transfer", -7_000)).toBe(7_000);
    expect(movementAmountForInput("income", 12_000)).toBe(12_000);
    expect(movementAmountForInput("adjustment", -9_000)).toBe(-9_000);
  });
});
