export type Route =
  | { name: "overview" }
  | { name: "movements" }
  | { name: "accounts" }
  | { name: "statement"; accountId: string }
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
      case "configuracoes":
        return { name: "settings" };
      default:
        return { name: "unavailable", path };
    }
  }

  if (segments.length === 2 && segments[0] === "contas") {
    return { name: "statement", accountId: segments[1] };
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
    case "movements":
      return "#/movimentacoes";
    case "accounts":
      return "#/contas";
    case "statement":
      return `#/contas/${encodeURIComponent(route.accountId)}`;
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
