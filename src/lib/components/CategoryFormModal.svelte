<script lang="ts">
  import { untrack } from "svelte";

  import type { CategoryDto } from "../api/bindings";
  import { CATEGORY_ICONS } from "../category-icons";
  import type { CategoryFormRequest } from "../form-types";
  import CategoryIcon from "./CategoryIcon.svelte";
  import Modal from "./Modal.svelte";

  let {
    category = null,
    saving,
    error,
    onsave,
    onclose,
  }: {
    category?: CategoryDto | null;
    saving: boolean;
    error: string | null;
    onsave: (request: CategoryFormRequest) => void;
    onclose: () => void;
  } = $props();

  const initial = untrack(() => ({
    editing: category !== null,
    name: category?.name ?? "",
    kind: category?.kind ?? "expense",
    icon: category?.icon ?? "generic",
  }));
  const editing = initial.editing;
  let name = $state(initial.name);
  let kind = $state(initial.kind);
  let icon = $state(initial.icon);
  let localError = $state<string | null>(null);

  function submit(): void {
    localError = null;
    if (!name.trim()) {
      localError = "Informe o nome da categoria.";
      return;
    }

    onsave({
      mode: editing ? "edit" : "create",
      id: category?.id ?? null,
      name: name.trim(),
      kind,
      icon,
    });
  }
</script>

<Modal
  title={editing ? "Editar categoria" : "Nova categoria"}
  {onclose}
>
  <form
    class="form"
    onsubmit={(event) => {
      event.preventDefault();
      submit();
    }}
  >
    <div class="form-grid">
      <label class="field field--wide">
        <span>Nome</span>
        <input
          bind:value={name}
          maxlength="80"
          placeholder="Ex.: Mercado"
          required
        />
      </label>

      <label class="field">
        <span>Tipo</span>
        <select bind:value={kind} disabled={editing}>
          <option value="expense">Despesa</option>
          <option value="income">Receita</option>
        </select>
      </label>

      <div class="field field--wide">
        <span>Ícone</span>
        <div class="icon-picker" role="group" aria-label="Ícone da categoria">
          {#each CATEGORY_ICONS as option (option.key)}
            <button
              type="button"
              class="icon-picker-item"
              class:is-active={icon === option.key}
              aria-label={option.label}
              aria-pressed={icon === option.key}
              title={option.label}
              onclick={() => (icon = option.key)}
            >
              <CategoryIcon icon={option.key} size={20} />
            </button>
          {/each}
        </div>
      </div>
    </div>

    <p class="form-hint">
      {editing
        ? "O tipo da categoria é imutável. O histórico continua vinculado."
        : "Categorias organizam despesas e receitas; não alteram saldos por si."}
    </p>

    {#if error || localError}
      <div class="inline-result inline-result--warn" role="alert">
        <span class="mono">{error ? "ERRO" : "VALIDAR"}</span>
        <span>{error ?? localError}</span>
      </div>
    {/if}

    <footer class="form-actions">
      <button type="button" class="button" onclick={onclose} disabled={saving}>
        Cancelar
      </button>
      <button type="submit" class="button button--primary" disabled={saving}>
        {saving ? "Salvando…" : editing ? "Salvar alterações" : "Criar categoria"}
      </button>
    </footer>
  </form>
</Modal>
