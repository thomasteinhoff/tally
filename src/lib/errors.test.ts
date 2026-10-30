import { describe, expect, it } from "vitest";

import { isAppErrorDto, normalizeError } from "./errors";

describe("isAppErrorDto", () => {
  it("aceita a forma pública de erro", () => {
    expect(
      isAppErrorDto({
        code: "DATABASE",
        message: "falha no banco",
        details: "detalhe técnico",
      }),
    ).toBe(true);

    expect(
      isAppErrorDto({
        code: "DATABASE",
        message: "falha no banco",
        details: null,
      }),
    ).toBe(true);
  });

  it("rejeita valores malformados", () => {
    expect(isAppErrorDto(null)).toBe(false);
    expect(isAppErrorDto(undefined)).toBe(false);
    expect(isAppErrorDto({ code: 1, message: "falha", details: null })).toBe(
      false,
    );
    expect(isAppErrorDto({ code: "DATABASE", message: "falha" })).toBe(false);
  });
});

describe("normalizeError", () => {
  it("preserva um AppErrorDto válido", () => {
    const error = {
      code: "BOOTSTRAP",
      message: "o banco não está disponível",
      details: "arquivo bloqueado",
    };

    expect(normalizeError(error)).toEqual(error);
  });

  it("embrulha Error sem inventar sucesso", () => {
    expect(normalizeError(new Error("falha de IPC"))).toEqual({
      code: "UNEXPECTED",
      message: "Ocorreu um erro inesperado.",
      details: "falha de IPC",
    });
  });

  it("trata valores desconhecidos", () => {
    expect(normalizeError("sem detalhe")).toEqual({
      code: "UNEXPECTED",
      message: "Ocorreu um erro inesperado.",
      details: "sem detalhe",
    });

    expect(normalizeError(null)).toEqual({
      code: "UNEXPECTED",
      message: "Ocorreu um erro inesperado.",
      details: null,
    });
  });
});
