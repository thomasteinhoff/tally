<script lang="ts">
  import {
    createMutation,
    createQuery,
    useQueryClient,
  } from "@tanstack/svelte-query";

  import type { CardSummaryDto } from "../lib/api/bindings";
  import {
    createAccount,
    createCardPayment,
    createCardPurchase,
    listAccounts,
    listCards,
    listCategories,
  } from "../lib/api";
  import AccountFormModal from "../lib/components/AccountFormModal.svelte";
  import CardPaymentModal from "../lib/components/CardPaymentModal.svelte";
  import CardPurchaseModal from "../lib/components/CardPurchaseModal.svelte";
  import { normalizeError } from "../lib/errors";
  import type { AccountFormRequest } from "../lib/form-types";
  import { formatCents } from "../lib/format";
  import { notify } from "../lib/toast.svelte";

  const queryClient = useQueryClient();

  let composer = $state<"card" | "purchase" | "payment" | null>(null);
  let activeCardId = $state("");

  const cardsQuery = createQuery(() => ({
    queryKey: ["cards"],
    queryFn: listCards,
  }));

  const accountsQuery = createQuery(() => ({
    queryKey: ["accounts", false],
    queryFn: () => listAccounts(false),
  }));

  const categoriesQuery = createQuery(() => ({
    queryKey: ["categories", false],
    queryFn: () => listCategories(false),
  }));

  const cardMutation = createMutation(() => ({
    mutationFn: (request: AccountFormRequest) =>
      createAccount({
        name: request.name,
        account_type: request.accountType,
        opening_balance_cents: request.openingBalanceCents,
        credit_limit_cents: request.creditLimitCents,
      }),
    onSuccess: () => {
      void queryClient.invalidateQueries();
      composer = null;
      notify("Cartão criado.");
    },
  }));

  const purchaseMutation = createMutation(() => ({
    mutationFn: createCardPurchase,
    onSuccess: () => {
      void queryClient.invalidateQueries();
      composer = null;
      notify("Compra lançada.");
    },
  }));

  const paymentMutation = createMutation(() => ({
    mutationFn: createCardPayment,
    onSuccess: () => {
      void queryClient.invalidateQueries();
      composer = null;
      notify("Pagamento registrado.");
    },
  }));

  const cards = $derived(cardsQuery.data ?? []);
  const activeCards = $derived(cards.filter((card) => !card.archived_at));
  const totalDebtCents = $derived(
    activeCards.reduce((total, card) => total + card.debt_cents, 0),
  );
  const overLimitCount = $derived(
    activeCards.filter((card) => card.over_limit).length,
  );
  const accounts = $derived(accountsQuery.data ?? []);
  const categories = $derived(categoriesQuery.data ?? []);

  const queryError = $derived(
    cardsQuery.error ? normalizeError(cardsQuery.error) : null,
  );

  function openPurchase(card?: CardSummaryDto): void {
    activeCardId = card?.id ?? "";
    purchaseMutation.reset();
    composer = "purchase";
  }

  function openPayment(card?: CardSummaryDto): void {
    activeCardId = card?.id ?? "";
    paymentMutation.reset();
    composer = "payment";
  }

  function closeComposer(): void {
    if (
      cardMutation.isPending ||
      purchaseMutation.isPending ||
      paymentMutation.isPending
    ) {
      return;
    }
    composer = null;
    cardMutation.reset();
    purchaseMutation.reset();
    paymentMutation.reset();
  }
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Passivo</span>
    <h2>Cartões</h2>
    <p>
      Cartão é dívida, não despesa. Compras aumentam a dívida total; parcelas
      viram despesa na data de vencimento; pagamentos reduzem dívida e caixa.
    </p>
  </div>
  <div class="page-actions">
    <button
      type="button"
      class="button"
      onclick={() => {
        cardMutation.reset();
        composer = "card";
      }}
    >
      Novo cartão
    </button>
    <button
      type="button"
      class="button button--primary"
      onclick={() => openPurchase()}
      disabled={activeCards.length === 0}
    >
      Nova compra
    </button>
  </div>
</section>

<section class="summary-strip" aria-label="Resumo dos cartões ativos">
  <div>
    <span class="data-label">Dívida total</span>
    <strong class="summary-value">{formatCents(totalDebtCents)}</strong>
  </div>
  <div>
    <span class="data-label">Cartões ativos</span>
    <strong class="summary-value">{activeCards.length}</strong>
  </div>
  <div>
    <span class="data-label">Acima do limite</span>
    <strong class="summary-value">{overLimitCount}</strong>
  </div>
  <div class="summary-note">
    <span class="mono">dívida inclui parcelas futuras</span>
  </div>
</section>

<section class="panel" aria-labelledby="cards-title">
  <header class="panel-header">
    <div class="panel-heading">
      <span class="panel-kicker">Cadastro</span>
      <h3 class="panel-title" id="cards-title">Cartões</h3>
    </div>
    <span class="panel-meta">{cards.length} registrados</span>
  </header>

  {#if cardsQuery.isPending}
    <div class="state-panel" aria-busy="true">
      <span class="state-kicker">Lendo</span>
      <h2>Consultando cartões.</h2>
      <div class="loading-rule" aria-hidden="true"></div>
    </div>
  {:else if queryError}
    <div class="state-panel state-panel--error" role="alert">
      <span class="state-kicker">{queryError.code}</span>
      <h2>Não foi possível carregar os cartões.</h2>
      <p>{queryError.message}</p>
      <div class="state-actions">
        <button
          type="button"
          class="button button--primary"
          onclick={() => cardsQuery.refetch()}
        >
          Tentar novamente
        </button>
      </div>
    </div>
  {:else if cards.length === 0}
    <div class="empty-panel">
      <p>Nenhum cartão cadastrado.</p>
      <button
        type="button"
        class="button"
        onclick={() => {
          cardMutation.reset();
          composer = "card";
        }}
      >
        Criar o primeiro
      </button>
    </div>
  {:else}
    <div class="table-wrap">
      <table class="data-table">
        <thead>
          <tr>
            <th scope="col">Cartão</th>
            <th scope="col">Situação</th>
            <th scope="col" class="align-right">Dívida</th>
            <th scope="col" class="align-right">Limite</th>
            <th scope="col" class="align-right">Disponível</th>
            <th scope="col" class="align-right">Vencido</th>
            <th scope="col" class="align-right">Mês atual</th>
            <th scope="col" class="align-right">Futuro</th>
            <th scope="col" class="align-right">Ações</th>
          </tr>
        </thead>
        <tbody>
          {#each cards as card (card.id)}
            <tr class:row-muted={Boolean(card.archived_at)}>
              <td>
                <div class="cell-stack">
                  <a class="text-link" href="#/cartoes/{card.id}">{card.name}</a>
                  <small class="mono">{card.id.slice(0, 8)}</small>
                </div>
              </td>
              <td>
                {#if card.archived_at}
                  <span class="tag tag--archived">Arquivado</span>
                {:else if card.over_limit}
                  <span class="tag tag--expense">Acima do limite</span>
                {:else}
                  <span class="tag tag--active">Ativo</span>
                {/if}
              </td>
              <td class="mono align-right">{formatCents(card.debt_cents)}</td>
              <td class="mono align-right">
                {card.credit_limit_cents === null
                  ? "—"
                  : formatCents(card.credit_limit_cents)}
              </td>
              <td
                class="mono align-right"
                class:amount-negative={card.available_limit_cents !== null &&
                  card.available_limit_cents < 0}
              >
                {card.available_limit_cents === null
                  ? "—"
                  : formatCents(card.available_limit_cents)}
              </td>
              <td
                class="mono align-right"
                class:amount-negative={card.overdue_cents > 0}
              >
                {formatCents(card.overdue_cents)}
              </td>
              <td class="mono align-right">
                {formatCents(card.current_month_cents)}
              </td>
              <td class="mono align-right">{formatCents(card.future_cents)}</td>
              <td class="align-right">
                <div class="row-actions">
                  <a class="button button--small" href="#/cartoes/{card.id}">
                    Abrir
                  </a>
                  <button
                    type="button"
                    class="button button--small"
                    onclick={() => openPurchase(card)}
                  >
                    Comprar
                  </button>
                  <button
                    type="button"
                    class="button button--small"
                    onclick={() => openPayment(card)}
                  >
                    Pagar
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

</section>

<section class="panel" aria-label="Cadastro de cartão">
  <div class="panel-footer">
    <span class="panel-footnote">
      O limite é informativo e não bloqueia compras. Cartões também aparecem em
      <a class="text-link" href="#/contas">Contas</a>; as compras parceladas, em
      <a class="text-link" href="#/parcelamentos">Parcelamentos</a>.
    </span>
  </div>
</section>

{#if composer === "card"}
  <AccountFormModal
    presetType="credit_card"
    saving={cardMutation.isPending}
    error={cardMutation.error
      ? normalizeError(cardMutation.error).message
      : null}
    onsave={(request) => cardMutation.mutate(request)}
    onclose={closeComposer}
  />
{:else if composer === "purchase"}
  <CardPurchaseModal
    cards={activeCards}
    {categories}
    defaultCardId={activeCardId}
    saving={purchaseMutation.isPending}
    error={purchaseMutation.error
      ? normalizeError(purchaseMutation.error).message
      : null}
    onsave={(input) => purchaseMutation.mutate(input)}
    onclose={closeComposer}
  />
{:else if composer === "payment"}
  <CardPaymentModal
    cards={activeCards}
    {accounts}
    defaultCardId={activeCardId}
    saving={paymentMutation.isPending}
    error={paymentMutation.error
      ? normalizeError(paymentMutation.error).message
      : null}
    onsave={(input) => paymentMutation.mutate(input)}
    onclose={closeComposer}
  />
{/if}
