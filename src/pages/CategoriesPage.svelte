<script lang="ts">
  import {
    createMutation,
    createQuery,
    useQueryClient,
  } from "@tanstack/svelte-query";

  import type { CategoryDto } from "../lib/api/bindings";
  import {
    createCategory,
    deleteCategory,
    listCategories,
    setCategoryArchived,
    updateCategory,
  } from "../lib/api";
  import CategoryFormModal from "../lib/components/CategoryFormModal.svelte";
  import CategoryIcon from "../lib/components/CategoryIcon.svelte";
  import { confirmAction } from "../lib/confirm";
  import { normalizeError } from "../lib/errors";
  import { kindLabel } from "../lib/format";
  import type { CategoryFormRequest } from "../lib/form-types";
  import { notify } from "../lib/toast.svelte";

  const queryClient = useQueryClient();

  let includeArchived = $state(false);
  let kindFilter = $state("");
  let composerOpen = $state(false);
  let editing = $state<CategoryDto | null>(null);

  const categoriesQuery = createQuery(() => ({
    queryKey: ["categories", includeArchived],
    queryFn: () => listCategories(includeArchived),
  }));

  const saveMutation = createMutation(() => ({
    mutationFn: (request: CategoryFormRequest) => saveRequest(request),
    onSuccess: () => {
      void queryClient.invalidateQueries();
      composerOpen = false;
      editing = null;
      notify("Categoria salva.");
    },
  }));

  const archiveMutation = createMutation(() => ({
    mutationFn: (input: { id: string; archived: boolean }) =>
      setCategoryArchived(input.id, input.archived),
    onSuccess: (_category, input) => {
      void queryClient.invalidateQueries();
      notify(input.archived ? "Categoria arquivada." : "Categoria restaurada.");
    },
  }));

  const deleteMutation = createMutation(() => ({
    mutationFn: deleteCategory,
    onSuccess: () => {
      void queryClient.invalidateQueries();
      notify("Categoria excluída.");
    },
  }));

  const categories = $derived(categoriesQuery.data ?? []);
  const visibleCategories = $derived(
    kindFilter
      ? categories.filter((category) => category.kind === kindFilter)
      : categories,
  );
  const activeCount = $derived(
    categories.filter((category) => !category.archived_at).length,
  );
  const queryError = $derived(
    categoriesQuery.error ? normalizeError(categoriesQuery.error) : null,
  );
  const saveError = $derived(
    saveMutation.error ? normalizeError(saveMutation.error).message : null,
  );

  function openNew(): void {
    editing = null;
    composerOpen = true;
    saveMutation.reset();
  }

  function openEdit(category: CategoryDto): void {
    editing = category;
    composerOpen = true;
    saveMutation.reset();
  }

  function closeComposer(): void {
    if (saveMutation.isPending) {
      return;
    }
    composerOpen = false;
    editing = null;
    saveMutation.reset();
  }

  async function saveRequest(request: CategoryFormRequest): Promise<CategoryDto> {
    if (request.mode === "edit") {
      if (!request.id) {
        throw new Error("Categoria sem identificador.");
      }
      return updateCategory(request.id, request.name, request.icon);
    }

    return createCategory({
      name: request.name,
      kind: request.kind,
      icon: request.icon,
    });
  }

  async function confirmArchive(category: CategoryDto): Promise<void> {
    const archived = !category.archived_at;
    const confirmed = await confirmAction(
      archived
        ? `Arquivar “${category.name}”? O histórico permanece e a categoria deixa de aparecer em novos lançamentos.`
        : `Restaurar “${category.name}” para uso em novas movimentações?`,
      archived ? "Arquivar categoria" : "Restaurar categoria",
      archived ? "Arquivar" : "Restaurar",
    );
    if (confirmed) {
      archiveMutation.mutate({ id: category.id, archived });
    }
  }

  async function confirmDelete(category: CategoryDto): Promise<void> {
    const confirmed = await confirmAction(
      `Excluir “${category.name}”? Só é permitido quando não há histórico. Categorias usadas devem ser arquivadas.`,
      "Excluir categoria",
      "Excluir",
    );
    if (confirmed) {
      deleteMutation.mutate(category.id);
    }
  }
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Classificação</span>
    <h2>Categorias</h2>
    <p>
      Categorias de despesa e receita. O tipo é imutável e o histórico não é
      reescrito ao arquivar ou renomear.
    </p>
  </div>
  <button type="button" class="button button--primary" onclick={openNew}>
    Nova categoria
  </button>
</section>

<section class="summary-strip" aria-label="Resumo de categorias">
  <div>
    <span class="data-label">Categorias ativas</span>
    <strong class="summary-value">{activeCount}</strong>
  </div>
  <div>
    <span class="data-label">Despesas</span>
    <strong class="summary-value">
      {categories.filter((category) => category.kind === "expense" && !category.archived_at)
        .length}
    </strong>
  </div>
  <div>
    <span class="data-label">Receitas</span>
    <strong class="summary-value">
      {categories.filter((category) => category.kind === "income" && !category.archived_at)
        .length}
    </strong>
  </div>
</section>

<section class="panel" aria-labelledby="categories-title">
  <header class="panel-header">
    <div class="panel-heading">
      <span class="panel-kicker">Cadastro</span>
      <h3 class="panel-title" id="categories-title">Categorias</h3>
    </div>

    <div class="header-tools">
      <label class="field field--inline">
        <span>Tipo</span>
        <select bind:value={kindFilter}>
          <option value="">Todos</option>
          <option value="expense">Despesa</option>
          <option value="income">Receita</option>
        </select>
      </label>
      <label class="toggle">
        <input type="checkbox" bind:checked={includeArchived} />
        <span>
          Mostrar arquivadas
          <span
            class="info-mark"
            role="img"
            aria-label="Categorias arquivadas não aparecem em novos lançamentos. O histórico continua intacto."
            title="Categorias arquivadas não aparecem em novos lançamentos. O histórico continua intacto."
            >i</span
          >
        </span>
      </label>
    </div>
  </header>

  {#if categoriesQuery.isPending}
    <div class="state-panel" aria-busy="true">
      <span class="state-kicker">Lendo</span>
      <h2>Consultando categorias.</h2>
      <div class="loading-rule" aria-hidden="true"></div>
    </div>
  {:else if queryError}
    <div class="state-panel state-panel--error" role="alert">
      <span class="state-kicker">{queryError.code}</span>
      <h2>Não foi possível carregar as categorias.</h2>
      <p>{queryError.message}</p>
      <div class="state-actions">
        <button
          type="button"
          class="button button--primary"
          onclick={() => categoriesQuery.refetch()}
        >
          Tentar novamente
        </button>
      </div>
    </div>
  {:else if visibleCategories.length === 0}
    <div class="empty-panel">
      <p>Nenhuma categoria com estes filtros.</p>
      <button type="button" class="button" onclick={openNew}>
        Criar a primeira
      </button>
    </div>
  {:else}
    <div class="table-wrap">
      <table class="data-table">
        <thead>
          <tr>
            <th scope="col">Categoria</th>
            <th scope="col">Tipo</th>
            <th scope="col">Situação</th>
            <th scope="col" class="align-right">Ações</th>
          </tr>
        </thead>
        <tbody>
          {#each visibleCategories as category (category.id)}
            <tr class:row-muted={Boolean(category.archived_at)}>
              <td>
                <span class="category-cell">
                  <CategoryIcon icon={category.icon} />
                  {category.name}
                </span>
              </td>
              <td>
                <span class="tag tag--{category.kind}"
                  >{kindLabel(category.kind)}</span
                >
              </td>
              <td>
                {#if category.archived_at}
                  <span class="tag tag--archived">Arquivada</span>
                {:else}
                  <span class="tag tag--active">Ativa</span>
                {/if}
              </td>
              <td class="align-right">
                <div class="row-actions">
                  <button
                    type="button"
                    class="button button--small"
                    onclick={() => openEdit(category)}
                  >
                    Editar
                  </button>
                  <button
                    type="button"
                    class="button button--small"
                    onclick={() => void confirmArchive(category)}
                    disabled={archiveMutation.isPending}
                  >
                    {category.archived_at ? "Restaurar" : "Arquivar"}
                  </button>
                  <button
                    type="button"
                    class="button button--small button--danger"
                    onclick={() => void confirmDelete(category)}
                    disabled={deleteMutation.isPending}
                  >
                    Excluir
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

  {#if archiveMutation.error || deleteMutation.error}
    <div class="inline-result inline-result--warn" role="alert">
      <span class="mono">ERRO</span>
      <span>
        {normalizeError(archiveMutation.error ?? deleteMutation.error).message}
      </span>
    </div>
  {/if}
</section>

{#if composerOpen}
  <CategoryFormModal
    category={editing}
    saving={saveMutation.isPending}
    error={saveError}
    onsave={(request) => saveMutation.mutate(request)}
    onclose={closeComposer}
  />
{/if}
