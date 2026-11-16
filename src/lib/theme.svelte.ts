import {
  DEFAULT_THEME_PREFERENCE,
  parseStoredTheme,
  resolveTheme,
  THEME_STORAGE_KEY,
  type ThemePreference,
} from "./theme";

const darkQuery = window.matchMedia("(prefers-color-scheme: dark)");

function readStoredPreference(): ThemePreference {
  try {
    return parseStoredTheme(localStorage.getItem(THEME_STORAGE_KEY));
  } catch {
    return DEFAULT_THEME_PREFERENCE;
  }
}

export const themeState = $state({
  preference: readStoredPreference(),
  systemDark: darkQuery.matches,
});

function apply(): void {
  document.documentElement.dataset.theme = resolveTheme(
    themeState.preference,
    themeState.systemDark,
  );
}

export function setThemePreference(preference: ThemePreference): void {
  themeState.preference = preference;
  try {
    localStorage.setItem(THEME_STORAGE_KEY, preference);
  } catch {
    // Storage indisponível: o tema vale para esta sessão, sem persistir.
  }
  apply();
}

/**
 * Aplica o tema antes do primeiro render e acompanha a preferência do
 * Windows enquanto "Seguir o sistema" estiver ativo.
 */
export function startTheme(): void {
  apply();
  darkQuery.addEventListener("change", (event) => {
    themeState.systemDark = event.matches;
    apply();
  });
}
