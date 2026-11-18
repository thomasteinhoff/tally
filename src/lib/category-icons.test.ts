import { describe, expect, it } from "vitest";

import {
  CATEGORY_ICONS,
  categoryIconLabel,
  categoryIconPath,
  isCategoryIconKey,
} from "./category-icons";

describe("categoryIcons", () => {
  it("mantém um catálogo curado com chaves válidas e únicas", () => {
    const keys = CATEGORY_ICONS.map((icon) => icon.key);
    expect(new Set(keys).size).toBe(keys.length);
    expect(keys).toContain("generic");
    expect(keys.length).toBeGreaterThanOrEqual(40);
    expect(
      CATEGORY_ICONS.every(
        (icon) => icon.path.length > 0 && icon.label.length > 0,
      ),
    ).toBe(true);
    expect(
      keys.every((key) => key.length <= 32 && /^[a-z0-9_]+$/.test(key)),
    ).toBe(true);
  });

  it("preserva as chaves da versão anterior e cobre despesas e receitas", () => {
    const keys = CATEGORY_ICONS.map((icon) => icon.key);
    const legacy = [
      "generic", "cart", "home", "car", "bus", "fuel", "food", "coffee",
      "heart", "pill", "book", "game", "film", "music", "plane", "gift",
      "paw", "bolt", "droplet", "wifi", "briefcase", "piggy", "trend",
      "star",
    ];
    const addedExpenses = [
      "basket", "restaurant", "clothes", "gym", "camera", "plant", "tools",
      "cleaning", "furniture", "phone", "software", "mail", "insurance",
      "tax", "hotel", "parking", "bicycle", "donation",
    ];
    const addedIncome = [
      "salary", "bonus", "freelance", "business", "sales", "interest",
      "coins", "refund",
    ];

    for (const key of [...legacy, ...addedExpenses, ...addedIncome]) {
      expect(keys).toContain(key);
      if (key !== "generic") {
        expect(categoryIconPath(key)).not.toBe(categoryIconPath("generic"));
      }
    }
  });

  it("cai no genérico para chaves vazias ou desconhecidas", () => {
    expect(categoryIconPath("coffee")).not.toBe(categoryIconPath("generic"));
    expect(categoryIconPath("")).toBe(categoryIconPath("generic"));
    expect(categoryIconPath(null)).toBe(categoryIconPath("generic"));
    expect(categoryIconPath("inexistente")).toBe(categoryIconPath("generic"));
    expect(categoryIconLabel("inexistente")).toBe("Genérico");
    expect(isCategoryIconKey("coffee")).toBe(true);
    expect(isCategoryIconKey("inexistente")).toBe(false);
  });
});
