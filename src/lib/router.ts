export type Route =
  | { name: "overview" }
  | { name: "analysis" }
  | { name: "movements" }
  | { name: "accounts" }
  | { name: "statement"; accountId: string }
  | { name: "savings"; accountId: string }
  | { name: "cards" }
  | { name: "card"; cardId: string }
  | { name: "installments" }
  | { name: "installment"; planId: string }
  | { name: "categories" }
  | { name: "settings" }
  | { name: "unavailable"; path: string };

export function parseRoute(hash: string): Route {
  const withoutHash = hash.startsWith("#") ? hash.slice(1) : hash;
  const path = (withoutHash.split("?")[0] ?? "").trim();
  const segments = path.split("/").filter(Boolean).map(decodeURIComponent);

  if (segments.length === 0) {
    return { name: "overview" };
  }

  if (segments.length === 1) {
    switch (segments[0]) {
      case "quadro":
        return { name: "analysis" };
      case "movimentacoes":
        return { name: "movements" };
      case "contas":
        return { name: "accounts" };
      case "cartoes":
        return { name: "cards" };
      case "parcelamentos":
        return { name: "installments" };
      case "categorias":
        return { name: "categories" };
      case "relatorios":
        // Caminho legado do Quadro de Análises; mantém links antigos.
        return { name: "analysis" };
      case "configuracoes":
      case "sobre":
        // Sobre foi unificado em Configurações.
        return { name: "settings" };
      default:
        return { name: "unavailable", path };
    }
  }

  if (segments.length === 2 && segments[0] === "contas") {
    return { name: "statement", accountId: segments[1] };
  }

  if (segments.length === 2 && segments[0] === "cofrinhos") {
    return { name: "savings", accountId: segments[1] };
  }

  if (segments.length === 2 && segments[0] === "cartoes") {
    return { name: "card", cardId: segments[1] };
  }

  if (segments.length === 2 && segments[0] === "parcelamentos") {
    return { name: "installment", planId: segments[1] };
  }

  return { name: "unavailable", path };
}

export function routeHref(route: Route): string {
  switch (route.name) {
    case "overview":
      return "#/";
    case "analysis":
      return "#/quadro";
    case "movements":
      return "#/movimentacoes";
    case "accounts":
      return "#/contas";
    case "statement":
      return `#/contas/${encodeURIComponent(route.accountId)}`;
    case "savings":
      return `#/cofrinhos/${encodeURIComponent(route.accountId)}`;
    case "cards":
      return "#/cartoes";
    case "card":
      return `#/cartoes/${encodeURIComponent(route.cardId)}`;
    case "installments":
      return "#/parcelamentos";
    case "installment":
      return `#/parcelamentos/${encodeURIComponent(route.planId)}`;
    case "categories":
      return "#/categorias";
    case "settings":
      return "#/configuracoes";
    case "unavailable":
      return `#${route.path}`;
  }
}

/**
 * Navegação principal. A ordem é intencional: Operação (fluxo diário) antes
 * de Sistema (cadastros).
 */
export type NavSection =
  | "overview"
  | "analysis"
  | "movements"
  | "accounts"
  | "installments"
  | "categories"
  | "settings";

export type NavItem = {
  section: NavSection;
  label: string;
  href: string;
};

export const NAV_GROUPS: ReadonlyArray<{
  label: string;
  items: ReadonlyArray<NavItem>;
}> = [
  {
    label: "Operação",
    items: [
      { section: "overview", label: "Visão geral", href: "#/" },
      { section: "analysis", label: "Quadro de análises", href: "#/quadro" },
      { section: "movements", label: "Movimentações", href: "#/movimentacoes" },
      { section: "accounts", label: "Contas e cartões", href: "#/contas" },
      { section: "installments", label: "Parcelamentos", href: "#/parcelamentos" },
    ],
  },
  {
    label: "Sistema",
    items: [
      { section: "categories", label: "Categorias", href: "#/categorias" },
      { section: "settings", label: "Configurações", href: "#/configuracoes" },
    ],
  },
];

/** Rotas filhas mantêm o item pai ativo. */
export function isSectionActive(route: Route, section: NavSection): boolean {
  if (section === "accounts") {
    return (
      route.name === "accounts" ||
      route.name === "statement" ||
      route.name === "savings" ||
      route.name === "cards" ||
      route.name === "card"
    );
  }
  if (section === "installments") {
    return route.name === "installments" || route.name === "installment";
  }
  return route.name === section;
}

