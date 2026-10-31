export type Route =
  | { name: "overview" }
  | { name: "movements" }
  | { name: "accounts" }
  | { name: "statement"; accountId: string }
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
    case "categories":
      return "#/categorias";
    case "settings":
      return "#/configuracoes";
    case "unavailable":
      return `#${route.path}`;
  }
}
