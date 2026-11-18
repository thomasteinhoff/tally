import { describe, expect, it } from "vitest";

import { currentMonthIso, monthBounds, shiftMonth } from "./month";

describe("month", () => {
  it("produz o mês corrente em ISO", () => {
    expect(currentMonthIso()).toMatch(/^\d{4}-(0[1-9]|1[0-2])$/);
  });

  it("desloca meses com virada de ano", () => {
    expect(shiftMonth("2025-12", 1)).toBe("2026-01");
    expect(shiftMonth("2026-01", -1)).toBe("2025-12");
    expect(shiftMonth("2024-03", -13)).toBe("2023-02");
    expect(shiftMonth("2024-03", 22)).toBe("2026-01");
    expect(shiftMonth("2026-06", 0)).toBe("2026-06");
  });

  it("calcula os limites do mês, incluindo fevereiro bissexto", () => {
    expect(monthBounds("2026-01")).toEqual({
      from: "2026-01-01",
      to: "2026-01-31",
    });
    expect(monthBounds("2024-02")).toEqual({
      from: "2024-02-01",
      to: "2024-02-29",
    });
    expect(monthBounds("2025-02")).toEqual({
      from: "2025-02-01",
      to: "2025-02-28",
    });
    expect(monthBounds("2026-12")).toEqual({
      from: "2026-12-01",
      to: "2026-12-31",
    });
  });

  it("rejeita meses fora do formato", () => {
    expect(() => monthBounds("2026-13")).toThrow(RangeError);
    expect(() => monthBounds("26-01")).toThrow(RangeError);
    expect(() => shiftMonth("2026-1", 1)).toThrow(RangeError);
    expect(() => shiftMonth("2026-01", 1.5)).toThrow(RangeError);
  });
});
