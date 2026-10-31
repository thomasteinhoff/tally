<script lang="ts">
  import {
    createMutation,
    createQuery,
    useQueryClient,
  } from "@tanstack/svelte-query";

  import type {
    MovementDto,
    MovementFilter,
  } from "../lib/api/bindings";
  import {
    createAdjustment,
    createExpense,
    createIncome,
    createTransfer,
    deleteMovement,
    listAccounts,
    listCategories,
    listMovements,
    updateMovement,
  } from "../lib/api";
  import MovementFormModal from "../lib/components/MovementFormModal.svelte";
  import { confirmAction } from "../lib/confirm";
  import { normalizeError } from "../lib/errors";
  import {
    currentTime,
    formatCents,
    formatDate,
    kindLabel,
    paymentLabel,
    periodLabel,
  } from "../lib/format";
  import type { SaveMovementRequest } from "../lib/movement-form";
  import { notify } from "../lib/toast.svelte";
  import { uiState } from "../lib/ui.svelte";

  const queryClient = useQueryClient();

  let filters = $state({
    accountId: "",
    categoryId: "",
    kind: "",
    period: "",
    from: "",
    to: "",
  });
  let composerOpen = $state(false);
  let editing = $state<MovementDto | null>(null);
  let startedTime = $state("");
  let deleteError = $state<string | null>(null);

  const filter = $derived<MovementFilter>({
    account_id: filters.accountId || null,
    category_id: filters.categoryId || null,
    kind: filters.kind || null,
    period: filters.period || null,
    from: filters.from || null,
    to: filters.to || null,
  });

  const accountsQuery = createQuery(() => ({
    queryKey: ["accounts", true],
    queryFn: () => listAccounts(true),
  }));

  const categoriesQuery = createQuery(() => ({
    queryKey: ["categories", true],
    queryFn: () => listCategories(true),
  }));

  const movementsQuery = createQuery(() => ({
    queryKey: ["movements", filter],
    queryFn: () => listMovements(filter),
  }));

  const saveMutation = createMutation(() => ({
    mutationFn: (request: SaveMovementRequest) => saveRequest(request),
    onSuccess: () => {
      void queryClient.invalidateQueries();
      composerOpen = false;
      editing = null;
      notify("Movimentação salva.");
    },
  }));

  const deleteMutation = createMutation(() => ({
    mutationFn: deleteMovement,
    onSuccess: () => {
      void queryClient.invalidateQueries();
      deleteError = null;
      notify("Movimentação excluída.");
    },
  }));

  const saveError = $derived(
    saveMutation.error ? normalizeError(saveMutation.error).message : null,
  );
  const queryError = $derived(
    movementsQuery.error ? normalizeError(movementsQuery.error) : null,
  );

  const accounts = $derived(accountsQuery.data ?? []);
  const categories = $derived(categoriesQuery.data ?? []);
  const movements = $derived(movementsQuery.data ?? []);

  $effect(() => {
    if (uiState.openNewMovementRequested) {
      uiState.openNewMovementRequested = false;
      openNew();
    }
  });

  function openNew(): void {
    editing = null;
    startedTime = currentTime();
    composerOpen = true;
    saveMutation.reset();
  }

  function openEdit(movement: MovementDto): void {
    editing = movement;
    startedTime = "";
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

  function clearFilters(): void {
    filters = {
      accountId: "",
      categoryId: "",
      kind: "",
      period: "",
      from: "",
      to: "",
    };
  }

  async function confirmDelete(movement: MovementDto): Promise<void> {
    const label =
      movement.description || kindLabel(movement.kind).toLowerCase();
    const confirmed = await confirmAction(
      `Excluir “${label}” em ${formatDate(movement.occurred_on)}? A exclusão remove os lançamentos contábeis e recalcula o saldo da conta. Esta ação não pode ser desfeita.`,
      "Excluir movimentação",
      "Excluir",
    );
    if (confirmed) {
      deleteError = null;
      deleteMutation.mutate(movement.id);
    }
  }

  async function saveRequest(request: SaveMovementRequest): Promise<MovementDto> {
    const { draft } = request;
    const shared = {
      account_id: draft.primaryAccountId,
      category_id: draft.categoryId || null,
      amount_cents: draft.amountCents,
      occurred_on: draft.occurredOn,
      occurred_time: draft.occurredTime || null,
      manual_period: draft.manualPeriod || null,
      description: draft.description.trim() || null,
    };

    if (request.mode === "edit") {
      if (!request.id) {
        throw new Error("Movimentação sem identificador.");
      }

      return updateMovement(request.id, {
        kind: draft.kind,
        primary_account_id: draft.primaryAccountId,
        counterparty_account_id: draft.counterpartyAccountId || null,
        category_id: draft.categoryId || null,
        amount_cents: draft.amountCents,
        occurred_on: draft.occurredOn,
        occurred_time: draft.occurredTime || null,
        manual_period: draft.manualPeriod || null,
        description: draft.description.trim() || null,
        payment_method: draft.paymentMethod || null,
      });
    }

    switch (draft.kind) {
      case "expense":
        if (!shared.category_id || !draft.paymentMethod) {
          throw new Error("Categoria e método são obrigatórios.");
        }
        return createExpense({
          ...shared,
          category_id: shared.category_id,
          payment_method: draft.paymentMethod,
        });
      case "income":
        if (!shared.category_id || !draft.paymentMethod) {
          throw new Error("Categoria e método são obrigatórios.");
        }
        return createIncome({
          ...shared,
          category_id: shared.category_id,
          payment_method: draft.paymentMethod,
        });
      case "transfer":
        return createTransfer({
          from_account_id: draft.primaryAccountId,
          to_account_id: draft.counterpartyAccountId,
          amount_cents: draft.amountCents,
          occurred_on: draft.occurredOn,
          occurred_time: draft.occurredTime || null,
          manual_period: draft.manualPeriod || null,
          description: draft.description.trim() || null,
        });
      case "adjustment":
        return createAdjustment({
          account_id: draft.primaryAccountId,
          amount_cents: draft.amountCents,
          occurred_on: draft.occurredOn,
          description: draft.description.trim() || null,
        });
    }
  }
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Livro</span>
    <h2>Movimentações</h2>
    <p>
      Despesas, receitas, transferências e ajustes publicados em partidas
      dobradas. Saldos e períodos vêm do Rust.
    </p>
  </div>
  <button type="button" class="button button--primary" onclick={openNew}>
    Nova movimentação
  </button>
</section>

<section class="panel" aria-labelledby="movement-filter-title">
  <header class="panel-header">
    <div class="panel-heading">
      <span class="panel-kicker">Filtros</span>
      <h3 class="panel-title" id="movement-filter-title">Filtrar livro</h3>
    </div>
    <button type="button" class="button button--quiet" onclick={clearFilters}>
      Limpar filtros
    </button>
  </header>

  <div class="filters">
    <label class="field">
      <span>Conta</span>
      <select bind:value={filters.accountId}>
        <option value="">Todas</option>
        {#each accounts as account (account.id)}
          <option value={account.id}>{account.name}</option>
        {/each}
      </select>
    </label>

    <label class="field">
      <span>Categoria</span>
      <select bind:value={filters.categoryId}>
        <option value="">Todas</option>
        {#each categories as category (category.id)}
          <option value={category.id}>{category.name}</option>
        {/each}
      </select>
    </label>

    <label class="field">
      <span>Tipo</span>
      <select bind:value={filters.kind}>
        <option value="">Todos</option>
        <option value="expense">Despesa</option>
        <option value="income">Receita</option>
        <option value="transfer">Transferência</option>
        <option value="adjustment">Ajuste</option>
      </select>
    </label>

    <label class="field">
      <span>Período</span>
      <select bind:value={filters.period}>
        <option value="">Todos</option>
        <option value="madrugada">Madrugada</option>
        <option value="manha">Manhã</option>
        <option value="tarde">Tarde</option>
        <option value="noite">Noite</option>
        <option value="custom">Personalizado</option>
      </select>
    </label>

    <label class="field">
      <span>De</span>
      <input type="date" bind:value={filters.from} />
    </label>

    <label class="field">
      <span>Até</span>
      <input type="date" bind:value={filters.to} />
    </label>
  </div>
</section>

<section class="panel" aria-labelledby="movement-list-title">
  <header class="panel-header">
    <div class="panel-heading">
      <span class="panel-kicker">Registro</span>
      <h3 class="panel-title" id="movement-list-title">Movimentações</h3>
    </div>
    <span class="panel-meta">
      {movements.length}
      {movements.length === 1 ? " lançamento" : " lançamentos"}
    </span>
  </header>

  {#if movementsQuery.isPending}
    <div class="state-panel" aria-busy="true">
      <span class="state-kicker">Lendo</span>
      <h2>Consultando o livro.</h2>
      <div class="loading-rule" aria-hidden="true"></div>
    </div>
  {:else if queryError}
    <div class="state-panel state-panel--error" role="alert">
      <span class="state-kicker">{queryError.code}</span>
      <h2>Não foi possível carregar as movimentações.</h2>
      <p>{queryError.message}</p>
      <div class="state-actions">
        <button
          type="button"
          class="button button--primary"
          onclick={() => movementsQuery.refetch()}
        >
          Tentar novamente
        </button>
      </div>
    </div>
  {:else if movements.length === 0}
    <div class="empty-panel">
      <p>Nenhuma movimentação com estes filtros.</p>
      <button type="button" class="button" onclick={openNew}>
        Lançar a primeira
      </button>
    </div>
  {:else}
    <div class="table-wrap">
      <table class="data-table">
        <thead>
          <tr>
            <th scope="col">Data</th>
            <th scope="col">Descrição</th>
            <th scope="col">Tipo</th>
            <th scope="col">Conta</th>
            <th scope="col">Categoria</th>
            <th scope="col">Método</th>
            <th scope="col">Período</th>
            <th scope="col" class="align-right">Valor</th>
            <th scope="col" class="align-right">Ações</th>
          </tr>
        </thead>
        <tbody>
          {#each movements as movement (movement.id)}
            <tr>
              <td>
                <div class="cell-stack">
                  <span>{formatDate(movement.occurred_on)}</span>
                  <small class="mono">{movement.occurred_time ?? "manual"}</small>
                </div>
              </td>
              <td>
                <div class="cell-stack">
                  <span
                    >{movement.description ||
                      kindLabel(movement.kind)}</span
                  >
                  {#if movement.description}
                    <small class="mono">{kindLabel(movement.kind)}</small>
                  {/if}
                </div>
              </td>
              <td>
                <span class="tag tag--{movement.kind}"
                  >{kindLabel(movement.kind)}</span
                >
              </td>
              <td>
                <div class="cell-stack">
                  <span>{movement.primary_account_name}</span>
                  {#if movement.counterparty_account_name}
                    <small>→ {movement.counterparty_account_name}</small>
                  {/if}
                </div>
              </td>
              <td>{movement.category_name ?? "—"}</td>
              <td>{paymentLabel(movement.payment_method)}</td>
              <td>{periodLabel(movement.period)}</td>
              <td
                class="mono align-right"
                class:amount-negative={movement.amount_cents < 0}
                class:amount-positive={movement.amount_cents > 0}
              >
                {formatCents(movement.amount_cents)}
              </td>
              <td class="align-right">
                <div class="row-actions">
                  <button
                    type="button"
                    class="button button--small"
                    onclick={() => openEdit(movement)}
                  >
                    Editar
                  </button>
                  <button
                    type="button"
                    class="button button--small button--danger"
                    onclick={() => void confirmDelete(movement)}
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

  {#if deleteError || deleteMutation.error}
    <div class="inline-result inline-result--warn" role="alert">
      <span class="mono">ERRO</span>
      <span>
        {deleteError ??
          (deleteMutation.error
            ? normalizeError(deleteMutation.error).message
            : "")}
      </span>
    </div>
  {/if}
</section>

{#if composerOpen}
  {#key editing?.id ?? "new"}
    <MovementFormModal
      {accounts}
      {categories}
      movement={editing}
      {startedTime}
      saving={saveMutation.isPending}
      error={saveError}
      onsave={(request) => saveMutation.mutate(request)}
      onclose={closeComposer}
    />
  {/key}
{/if}
