/**
 * Preferência de tema — lógica pura, sem DOM.
 *
 * O tema é uma preferência de interface: mora no frontend, não no banco,
 * não é dado financeiro e não passa pelo Rust.
 */

export const THEME_PREFERENCES = ["light", "dark", "system"] as const;

export type ThemePreference = (typeof THEME_PREFERENCES)[number];
export type ResolvedTheme = "light" | "dark";

export const THEME_STORAGE_KEY = "tally.theme";
export const DEFAULT_THEME_PREFERENCE: ThemePreference = "system";

export const THEME_LABELS: Record<ThemePreference, string> = {
  light: "Tema claro",
  dark: "Tema escuro",
  system: "Seguir o sistema",
};

export const THEME_SHORT_LABELS: Record<ThemePreference, string> = {
  light: "Claro",
  dark: "Escuro",
  system: "Sistema",
};

export function isThemePreference(value: unknown): value is ThemePreference {
  return (
    value === "light" || value === "dark" || value === "system"
  );
}

/** Valor arbitrário vindo do storage nunca vira tema inválido. */
export function parseStoredTheme(raw: string | null): ThemePreference {
  return isThemePreference(raw) ? raw : DEFAULT_THEME_PREFERENCE;
}

export function resolveTheme(
  preference: ThemePreference,
  systemPrefersDark: boolean,
): ResolvedTheme {
  if (preference === "system") {
    return systemPrefersDark ? "dark" : "light";
  }
  return preference;
}
