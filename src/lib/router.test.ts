import { describe, expect, it } from "vitest";

import { parseRoute, routeHref } from "./router";

describe("parseRoute", () => {
  it("mapeia as rotas conhecidas", () => {
    expect(parseRoute("")).toEqual({ name: "overview" });
    expect(parseRoute("#/")).toEqual({ name: "overview" });
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
    expect(parseRoute("#/relatorios")).toEqual({ name: "overview" });
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
