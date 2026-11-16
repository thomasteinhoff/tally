import { describe, expect, it } from "vitest";

import {
  isThemePreference,
  parseStoredTheme,
  resolveTheme,
} from "./theme";

describe("tema", () => {
  it("resolve sistema a partir da preferência do Windows", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
  });

  it("preferência explícita ignora o sistema", () => {
    expect(resolveTheme("light", true)).toBe("light");
    expect(resolveTheme("dark", false)).toBe("dark");
  });

  it("descarta valores inválidos do storage", () => {
    expect(parseStoredTheme(null)).toBe("system");
    expect(parseStoredTheme("sepia")).toBe("system");
    expect(parseStoredTheme("dark")).toBe("dark");
  });

  it("reconhece apenas as três preferências", () => {
    expect(isThemePreference("light")).toBe(true);
    expect(isThemePreference("Dark")).toBe(false);
    expect(isThemePreference(undefined)).toBe(false);
  });
});
