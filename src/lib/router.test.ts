import { describe, expect, it } from "vitest";

import {
  isSectionActive,
  NAV_GROUPS,
  parseRoute,
  routeHref,
} from "./router";

describe("parseRoute", () => {
  it("mapeia as rotas conhecidas", () => {
    expect(parseRoute("")).toEqual({ name: "overview" });
    expect(parseRoute("#/")).toEqual({ name: "overview" });
    expect(parseRoute("#/quadro")).toEqual({ name: "analysis" });
    expect(parseRoute("#/movimentacoes")).toEqual({ name: "movements" });
    expect(parseRoute("#/contas")).toEqual({ name: "accounts" });
    expect(parseRoute("#/contas/abc%20123")).toEqual({
      name: "statement",
      accountId: "abc 123",
    });
    expect(parseRoute("#/cartoes")).toEqual({ name: "cards" });
    expect(parseRoute("#/cartoes/abc%20123")).toEqual({
      name: "card",
      cardId: "abc 123",
    });
    expect(parseRoute("#/parcelamentos")).toEqual({ name: "installments" });
    expect(parseRoute("#/parcelamentos/abc%20123")).toEqual({
      name: "installment",
      planId: "abc 123",
    });
    expect(parseRoute("#/categorias")).toEqual({ name: "categories" });
    expect(parseRoute("#/relatorios")).toEqual({ name: "analysis" });
    expect(parseRoute("#/configuracoes")).toEqual({ name: "settings" });
    expect(parseRoute("#/sobre")).toEqual({ name: "settings" });
  });

  it("preserva query string fora da rota e rejeita caminhos desconhecidos", () => {
    expect(parseRoute("#/movimentacoes?foo=1")).toEqual({
      name: "movements",
    });
    expect(parseRoute("#/desconhecido")).toEqual({
      name: "unavailable",
      path: "/desconhecido",
    });
    expect(parseRoute("#/contas/a/b")).toEqual({
      name: "unavailable",
      path: "/contas/a/b",
    });
  });
});

describe("routeHref", () => {
  it("gera o hash canônico de cada rota", () => {
    expect(routeHref({ name: "overview" })).toBe("#/");
    expect(routeHref({ name: "analysis" })).toBe("#/quadro");
    expect(routeHref({ name: "movements" })).toBe("#/movimentacoes");
    expect(routeHref({ name: "accounts" })).toBe("#/contas");
    expect(routeHref({ name: "statement", accountId: "a b" })).toBe(
      "#/contas/a%20b",
    );
    expect(routeHref({ name: "cards" })).toBe("#/cartoes");
    expect(routeHref({ name: "card", cardId: "a b" })).toBe(
      "#/cartoes/a%20b",
    );
    expect(routeHref({ name: "installments" })).toBe("#/parcelamentos");
    expect(routeHref({ name: "installment", planId: "a b" })).toBe(
      "#/parcelamentos/a%20b",
    );
    expect(routeHref({ name: "categories" })).toBe("#/categorias");
    expect(routeHref({ name: "settings" })).toBe("#/configuracoes");
    expect(routeHref({ name: "unavailable", path: "/x" })).toBe("#/x");
  });
});

describe("navegação", () => {
  it("mantém os sete itens na ordem definida", () => {
    expect(NAV_GROUPS.map((group) => group.label)).toEqual([
      "Operação",
      "Sistema",
    ]);
    expect(NAV_GROUPS.flatMap((group) => group.items.map((item) => item.section)))
      .toEqual([
        "overview",
        "analysis",
        "movements",
        "accounts",
        "installments",
        "categories",
        "settings",
      ]);
    expect(
      NAV_GROUPS.flatMap((group) => group.items.map((item) => item.href)),
    ).toEqual([
      "#/",
      "#/quadro",
      "#/movimentacoes",
      "#/contas",
      "#/parcelamentos",
      "#/categorias",
      "#/configuracoes",
    ]);
  });

  it("marca o item pai para rotas filhas", () => {
    expect(isSectionActive({ name: "analysis" }, "analysis")).toBe(true);
    expect(isSectionActive({ name: "statement", accountId: "a" }, "accounts"))
      .toBe(true);
    expect(isSectionActive({ name: "card", cardId: "c" }, "accounts")).toBe(
      true,
    );
    expect(
      isSectionActive({ name: "installment", planId: "p" }, "installments"),
    ).toBe(true);
    expect(isSectionActive({ name: "categories" }, "movements")).toBe(false);
  });
});
