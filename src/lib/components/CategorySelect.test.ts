import { fireEvent, render, screen } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

import type { CategoryDto } from "../api/bindings";
import CategorySelect from "./CategorySelect.svelte";

const categories: CategoryDto[] = [
  {
    id: "c1",
    name: "Mercado",
    kind: "expense",
    icon: "basket",
    archived_at: null,
    created_at: "2026-01-01T00:00:00.000Z",
  },
  {
    id: "c2",
    name: "Salário",
    kind: "income",
    icon: "salary",
    archived_at: null,
    created_at: "2026-01-01T00:00:00.000Z",
  },
  {
    id: "c3",
    name: "Outros",
    kind: "expense",
    icon: "generic",
    archived_at: null,
    created_at: "2026-01-01T00:00:00.000Z",
  },
];

function trigger(): HTMLElement {
  return screen.getByRole("combobox", { name: "Categoria" });
}

describe("CategorySelect", () => {
  it("mostra o ícone da selecionada e abre a lista com ícones nas opções", async () => {
    render(CategorySelect, { props: { categories, value: "c2" } });

    expect(trigger()).toHaveTextContent("Salário");
    expect(trigger()).toHaveAttribute("aria-expanded", "false");
    expect(trigger().querySelector("svg path")).toHaveAttribute("d");

    await fireEvent.click(trigger());

    expect(screen.getByRole("listbox")).toBeInTheDocument();
    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(3);
    for (const option of options) {
      expect(option.querySelector("svg path")).toHaveAttribute("d");
    }
    expect(screen.getByRole("option", { name: "Salário" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(trigger()).toHaveAttribute(
      "aria-activedescendant",
      expect.stringContaining("option-1"),
    );
  });

  it("navega com setas e seleciona com Enter", async () => {
    render(CategorySelect, { props: { categories, value: "c2" } });

    await fireEvent.keyDown(trigger(), { key: "ArrowDown" });
    expect(trigger()).toHaveAttribute("aria-expanded", "true");
    await fireEvent.keyDown(trigger(), { key: "ArrowDown" });
    expect(trigger()).toHaveAttribute(
      "aria-activedescendant",
      expect.stringContaining("option-2"),
    );

    await fireEvent.keyDown(trigger(), { key: "Enter" });
    expect(trigger()).toHaveAttribute("aria-expanded", "false");
    expect(trigger()).toHaveTextContent("Outros");
  });

  it("dá a volta na lista com Home/End e fecha com Escape sem fechar o modal", async () => {
    render(CategorySelect, { props: { categories, value: "" } });

    await fireEvent.click(trigger());
    await fireEvent.keyDown(trigger(), { key: "End" });
    expect(trigger()).toHaveAttribute(
      "aria-activedescendant",
      expect.stringContaining("option-2"),
    );
    await fireEvent.keyDown(trigger(), { key: "ArrowDown" });
    expect(trigger()).toHaveAttribute(
      "aria-activedescendant",
      expect.stringContaining("option-0"),
    );
    await fireEvent.keyDown(trigger(), { key: "Home" });
    expect(trigger()).toHaveAttribute(
      "aria-activedescendant",
      expect.stringContaining("option-0"),
    );

    const onWindowEscape = vi.fn();
    window.addEventListener("keydown", onWindowEscape);
    await fireEvent.keyDown(trigger(), { key: "Escape" });
    expect(trigger()).toHaveAttribute("aria-expanded", "false");
    expect(onWindowEscape).not.toHaveBeenCalled();
    window.removeEventListener("keydown", onWindowEscape);
  });

  it("fecha ao clicar fora e mantém o placeholder quando não há seleção", async () => {
    render(CategorySelect, { props: { categories } });

    expect(trigger()).toHaveTextContent("Escolha a categoria");
    await fireEvent.click(trigger());
    expect(trigger()).toHaveAttribute("aria-expanded", "true");
    await fireEvent.pointerDown(document.body);
    expect(trigger()).toHaveAttribute("aria-expanded", "false");
  });
});
