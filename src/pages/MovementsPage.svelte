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
    deleteMovement,
    listAccounts,
    listCards,
    listCategories,
    listMovements,
  } from "../lib/api";
  import CategoryIcon from "../lib/components/CategoryIcon.svelte";
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
  import type { MovementFormRequest } from "../lib/movement-form";
  import { submitMovementForm } from "../lib/movement-submit";
  import { notify } from "../lib/toast.svelte";
  import { uiState } from "../lib/ui.svelte";

  const queryClient = useQueryClient();

  let filters = $state({
    accountId: "",
    categoryId: "",
    kind: "",
    period: "",
    paymentMethod: "",
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
    payment_method: filters.paymentMethod || null,
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

  const cardsQuery = createQuery(() => ({
    queryKey: ["cards"],
    queryFn: listCards,
  }));

  const saveMutation = createMutation(() => ({
    mutationFn: (request: MovementFormRequest) => submitMovementForm(request),
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
  const categoryIcons = $derived(
    new Map(categories.map((category) => [category.id, category.icon])),
  );
  const cards = $derived(
    (cardsQuery.data ?? []).filter((card) => !card.archived_at),
  );
  const movements = $derived(movementsQuery.data ?? []);
  const editableKinds = new Set([
    "expense",
    "income",
    "transfer",
    "adjustment",
  ]);

  function cardHref(movement: MovementDto): string | null {
    if (movement.kind === "card_purchase") {
      return `#/cartoes/${movement.primary_account_id}`;
    }
    if (movement.kind === "card_payment" && movement.counterparty_account_id) {
      return `#/cartoes/${movement.counterparty_account_id}`;
    }
    return null;
  }

  function cardPurchaseSummary(movement: MovementDto): string | null {
    if (movement.kind !== "card_purchase" || !movement.installment_count) {
      return null;
    }
    const count = movement.installment_count;
    if (count === 1) {
      return "À vista · total da compra";
    }
    const total = Math.abs(movement.amount_cents);
    const first = movement.first_installment_cents;
    if (first === null) {
      return `${count}x · total da compra`;
    }
    return first * count === total
      ? `${count}x de ${formatCents(first)} · total da compra`
      : `${count}x a partir de ${formatCents(first)} · total da compra`;
  }

  function primaryAccountHref(movement: MovementDto): string {
    return movement.kind === "card_purchase"
      ? `#/cartoes/${movement.primary_account_id}`
      : `#/contas/${movement.primary_account_id}`;
  }

  function counterpartyAccountHref(movement: MovementDto): string {
    return movement.kind === "card_payment"
      ? `#/cartoes/${movement.counterparty_account_id}`
      : `#/contas/${movement.counterparty_account_id}`;
  }

  $effect(() => {
    if (uiState.openNewMovementRequested) {
      uiState.openNewMovementRequested = false;
      openNew();
    }
  });

  function openNew(): void {
    editing = null;
    startedTime = currentTime();
    saveMutation.reset();
    composerOpen = true;
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
      paymentMethod: "",
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
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Livro</span>
    <h2>Movimentações</h2>
    <p>
      Despesas, receitas, transferências, ajustes, compras e pagamentos de
      cartão em um único fluxo, publicados em partidas dobradas.
    </p>
  </div>
  <button type="button" class="button button--primary" onclick={openNew}>
    Nova movimentação
  </button>
</section>

<div class="inline-result">
  <span class="mono">CARTÃO</span>
  <span>
    Compras no cartão aparecem no livro pelo valor total da compra. A despesa
    reconhecida é apenas a parcela do período.
  </span>
</div>

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
      <span>Forma de pagamento</span>
      <select bind:value={filters.paymentMethod}>
        <option value="">Todas</option>
        <option value="pix">Pix</option>
        <option value="debit">Débito</option>
        <option value="credit">Crédito</option>
        <option value="subscription_credit">Crédito recorrente</option>
        <option value="cash">Dinheiro</option>
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
                  {#if cardPurchaseSummary(movement)}
                    <small class="mono">{cardPurchaseSummary(movement)}</small>
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
                  <a class="text-link" href={primaryAccountHref(movement)}>
                    {movement.primary_account_name}
                  </a>
                  {#if movement.counterparty_account_id && movement.counterparty_account_name}
                    <small>
                      → <a
                        class="text-link"
                        href={counterpartyAccountHref(movement)}
                      >
                        {movement.counterparty_account_name}
                      </a>
                    </small>
                  {/if}
                </div>
              </td>
              <td>
                {#if movement.category_id && categoryIcons.has(movement.category_id)}
                  <span class="category-cell">
                    <CategoryIcon
                      icon={categoryIcons.get(movement.category_id)}
                    />
                    {movement.category_name ?? "—"}
                  </span>
                {:else}
                  {movement.category_name ?? "—"}
                {/if}
              </td>
              <td>{paymentLabel(movement.payment_method)}</td>
              <td>{periodLabel(movement.period)}</td>
              <td
                class="mono align-right"
                class:amount-negative={movement.amount_cents < 0 &&
                  movement.kind !== "transfer"}
                class:amount-positive={movement.amount_cents > 0 &&
                  movement.kind !== "transfer"}
                class:amount-neutral={movement.kind === "transfer"}
              >
                {formatCents(movement.amount_cents)}
                {#if movement.kind === "card_purchase"}
                  <br /><small>total da compra</small>
                {/if}
              </td>
              <td class="align-right">
                <div class="row-actions">
                  {#if editableKinds.has(movement.kind)}
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
                  {:else if cardHref(movement)}
                    {#if movement.installment_plan_id}
                      <a
                        class="button button--small"
                        href="#/parcelamentos/{movement.installment_plan_id}"
                      >
                        Parcelamento
                      </a>
                    {/if}
                    <a class="button button--small" href={cardHref(movement)}>
                      Cartão
                    </a>
                  {/if}
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
      {cards}
      movement={editing}
      {startedTime}
      saving={saveMutation.isPending}
      error={saveError}
      onsave={(request) => saveMutation.mutate(request)}
      onclose={closeComposer}
    />
  {/key}
{/if}
