<script lang="ts">
  import {
    createMutation,
    createQuery,
    useQueryClient,
  } from "@tanstack/svelte-query";

  import {
    deleteCardPayment,
    deleteInstallmentPlan,
    getCard,
    listAccounts,
    listCardPayments,
    listCategories,
    listInstallmentPlans,
  } from "../lib/api";
  import MovementFormModal from "../lib/components/MovementFormModal.svelte";
  import { confirmAction } from "../lib/confirm";
  import { normalizeError } from "../lib/errors";
  import {
    formatCents,
    formatDate,
    formatLimit,
    installmentStatusLabel,
    periodLabel,
  } from "../lib/format";
  import type { MovementFormRequest } from "../lib/movement-form";
  import { submitMovementForm } from "../lib/movement-submit";
  import { notify } from "../lib/toast.svelte";

  let { cardId }: { cardId: string } = $props();

  const queryClient = useQueryClient();

  let composer = $state<"purchase" | "payment" | null>(null);

  const cardQuery = createQuery(() => ({
    queryKey: ["cards", cardId],
    queryFn: () => getCard(cardId),
  }));

  const plansQuery = createQuery(() => ({
    queryKey: ["installment-plans", cardId],
    queryFn: () => listInstallmentPlans(cardId),
  }));

  const paymentsQuery = createQuery(() => ({
    queryKey: ["card-payments", cardId],
    queryFn: () => listCardPayments(cardId),
  }));

  const accountsQuery = createQuery(() => ({
    queryKey: ["accounts", false],
    queryFn: () => listAccounts(false),
  }));

  const categoriesQuery = createQuery(() => ({
    queryKey: ["categories", false],
    queryFn: () => listCategories(false),
  }));

  const purchaseMutation = createMutation(() => ({
    mutationFn: (request: MovementFormRequest) => submitMovementForm(request),
    onSuccess: (_result, request) => {
      void queryClient.invalidateQueries();
      composer = null;
      notify(
        request.form === "card_payment"
          ? "Pagamento registrado."
          : "Compra lançada.",
      );
    },
  }));

  const deletePlanMutation = createMutation(() => ({
    mutationFn: deleteInstallmentPlan,
    onSuccess: () => {
      void queryClient.invalidateQueries();
      notify("Parcelamento excluído.");
    },
  }));

  const deletePaymentMutation = createMutation(() => ({
    mutationFn: deleteCardPayment,
    onSuccess: () => {
      void queryClient.invalidateQueries();
      notify("Pagamento excluído.");
    },
  }));

  const card = $derived(cardQuery.data);
  const plans = $derived(plansQuery.data ?? []);
  const payments = $derived(paymentsQuery.data ?? []);
  const accounts = $derived(accountsQuery.data ?? []);
  const categories = $derived(categoriesQuery.data ?? []);
  const hasUnallocated = $derived(
    payments.some((payment) => payment.unallocated_cents > 0),
  );

  const cards = $derived(card ? [card] : []);
  const loading = $derived(
    cardQuery.isPending || plansQuery.isPending || paymentsQuery.isPending,
  );
  const queryError = $derived(
    cardQuery.error
      ? normalizeError(cardQuery.error)
      : plansQuery.error
        ? normalizeError(plansQuery.error)
        : paymentsQuery.error
          ? normalizeError(paymentsQuery.error)
          : null,
  );
  const actionError = $derived(
    deletePlanMutation.error
      ? normalizeError(deletePlanMutation.error).message
      : deletePaymentMutation.error
        ? normalizeError(deletePaymentMutation.error).message
        : null,
  );

  function closeComposer(): void {
    if (purchaseMutation.isPending) {
      return;
    }
    composer = null;
    purchaseMutation.reset();
  }

  async function confirmDeletePlan(
    planId: string,
    description: string,
  ): Promise<void> {
    const confirmed = await confirmAction(
      `Excluir o parcelamento “${description}”? Só é permitido quando não há pagamentos alocados. As parcelas e os reconhecimentos serão removidos.`,
      "Excluir parcelamento",
      "Excluir",
    );
    if (confirmed) {
      deletePlanMutation.mutate(planId);
    }
  }

  async function confirmDeletePayment(paymentId: string): Promise<void> {
    const confirmed = await confirmAction(
      "Excluir este pagamento? O caixa será restaurado e as parcelas alocadas reabrem.",
      "Excluir pagamento",
      "Excluir",
    );
    if (confirmed) {
      deletePaymentMutation.mutate(paymentId);
    }
  }
</script>

<section class="page-intro">
  <div>
    <a class="text-link text-link--back" href="#/cartoes">← Cartões</a>
    <span class="eyebrow">Cartão</span>
    <h2>{card?.name ?? "Cartão"}</h2>
    <p>
      Dívida total inclui dívida inicial e todas as parcelas, inclusive futuras.
      Parcelas viram despesa na data de vencimento.
    </p>
  </div>
  <div class="page-actions">
    <button
      type="button"
      class="button"
      onclick={() => {
        purchaseMutation.reset();
        composer = "payment";
      }}
      disabled={!card || card.debt_cents <= 0 || Boolean(card.archived_at)}
    >
      Registrar pagamento
    </button>
    <button
      type="button"
      class="button button--primary"
      onclick={() => {
        purchaseMutation.reset();
        composer = "purchase";
      }}
      disabled={!card || Boolean(card.archived_at)}
    >
      Nova compra
    </button>
  </div>
</section>

{#if loading}
  <section class="panel" aria-busy="true">
    <div class="state-panel">
      <span class="state-kicker">Lendo</span>
      <h2>Consultando o cartão.</h2>
      <div class="loading-rule" aria-hidden="true"></div>
    </div>
  </section>
{:else if queryError}
  <section class="panel">
    <div class="state-panel state-panel--error" role="alert">
      <span class="state-kicker">{queryError.code}</span>
      <h2>Não foi possível carregar o cartão.</h2>
      <p>{queryError.message}</p>
      <div class="state-actions">
        <button
          type="button"
          class="button button--primary"
          onclick={() => {
            void cardQuery.refetch();
            void plansQuery.refetch();
            void paymentsQuery.refetch();
          }}
        >
          Tentar novamente
        </button>
      </div>
    </div>
  </section>
{:else if card}
  <section class="summary-strip" aria-label="Resumo do cartão">
    <div>
      <span class="data-label">Dívida total</span>
      <strong class="hero-value">{formatCents(card.debt_cents)}</strong>
    </div>
    <div>
      <span class="data-label">Disponível / limite</span>
      <strong
        class="summary-value"
        class:amount-negative={card.available_limit_cents !== null &&
          card.available_limit_cents < 0}
      >
        {formatLimit(card.available_limit_cents, card.credit_limit_cents)}
      </strong>
    </div>
    <div>
      <span class="data-label">Crédito</span>
      <strong class="summary-value amount-positive">
        {formatCents(card.credit_cents)}
      </strong>
    </div>
    <div class="summary-note">
      <span class="mono">
        vencido {formatCents(card.overdue_cents)} · mês{" "}
        {formatCents(card.current_month_cents)} · futuro{" "}
        {formatCents(card.future_cents)}
      </span>
    </div>
  </section>

  {#if card.over_limit}
    <div class="inline-result inline-result--warn" role="status">
      <span class="mono">LIMITE</span>
      <span>O cartão está acima do limite informado.</span>
    </div>
  {/if}

  <section class="panel" aria-labelledby="plans-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">Compromissos</span>
        <h3 class="panel-title" id="plans-title">Parcelamentos</h3>
      </div>
      <span class="panel-meta">
        {plans.length} {plans.length === 1 ? "compra" : "compras"}
      </span>
    </header>

    {#if plans.length === 0}
      <div class="empty-panel">
        <p>Nenhuma compra parcelada neste cartão.</p>
        <button
          type="button"
          class="button"
          onclick={() => {
            purchaseMutation.reset();
            composer = "purchase";
          }}
        >
          Lançar compra
        </button>
      </div>
    {:else}
      {#each plans as plan (plan.id)}
        <article class="plan-block">
          <header class="plan-header">
            <div class="cell-stack">
              <a class="text-link" href="#/parcelamentos/{plan.id}">
                {plan.description || plan.category_name}
              </a>
              <small class="mono">
                {plan.category_name} · {plan.installment_count}x · compra em
                {formatDate(plan.purchased_on)} · {periodLabel(plan.period)}
              </small>
            </div>
            <div class="plan-numbers">
              <span>Total {formatCents(plan.total_cents)}</span>
              <span>Pago {formatCents(plan.paid_cents)}</span>
              <span>Em aberto {formatCents(plan.outstanding_cents)}</span>
            </div>
            <button
              type="button"
              class="button button--small button--danger"
              onclick={() =>
                confirmDeletePlan(
                  plan.id,
                  plan.description || plan.category_name,
                )}
              disabled={deletePlanMutation.isPending || plan.paid_cents > 0}
            >
              Excluir
            </button>
          </header>

          {#if plan.paid_cents > 0}
            <div class="inline-result inline-result--warn" role="status">
              <span class="mono">BLOQUEADO</span>
              <span>
                Este parcelamento possui pagamentos. Remova os pagamentos antes
                de excluir.
              </span>
            </div>
          {/if}

          <div class="table-wrap">
            <table class="data-table">
              <thead>
                <tr>
                  <th scope="col">#</th>
                  <th scope="col">Vencimento</th>
                  <th scope="col">Período</th>
                  <th scope="col">Situação</th>
                  <th scope="col" class="align-right">Valor</th>
                  <th scope="col" class="align-right">Pago</th>
                  <th scope="col" class="align-right">Em aberto</th>
                </tr>
              </thead>
              <tbody>
                {#each plan.installments as installment (installment.id)}
                  <tr>
                    <td class="mono">{installment.seq}</td>
                    <td>{formatDate(installment.due_on)}</td>
                    <td>{periodLabel(plan.period)}</td>
                    <td>
                      <span
                        class="tag"
                        class:tag--active={installment.status === "paid"}
                        class:tag--expense={installment.status === "overdue"}
                        class:tag--partial={installment.status === "partial"}
                        class:tag--archived={installment.status === "open"}
                      >
                        {installmentStatusLabel(installment.status)}
                      </span>
                    </td>
                    <td class="mono align-right">
                      {formatCents(installment.amount_cents)}
                    </td>
                    <td class="mono align-right">
                      {formatCents(installment.paid_cents)}
                    </td>
                    <td class="mono align-right">
                      {formatCents(
                        installment.amount_cents - installment.paid_cents,
                      )}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </article>
      {/each}
    {/if}
  </section>

  <section class="panel" aria-labelledby="payments-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">Caixa</span>
        <h3 class="panel-title" id="payments-title">Pagamentos</h3>
      </div>
      <span class="panel-meta">
        {payments.length} {payments.length === 1 ? "pagamento" : "pagamentos"}
      </span>
    </header>

    {#if payments.length === 0}
      <div class="empty-panel">
        <p>Nenhum pagamento registrado.</p>
      </div>
    {:else}
      <div class="table-wrap">
        <table class="data-table">
          <thead>
            <tr>
              <th scope="col">Data</th>
              <th scope="col">Descrição</th>
              <th scope="col">Origem</th>
              <th scope="col">Alocações</th>
              <th scope="col" class="align-right">Valor</th>
              <th scope="col" class="align-right">Não alocado</th>
              <th scope="col" class="align-right">Ações</th>
            </tr>
          </thead>
          <tbody>
            {#each payments as payment (payment.id)}
              <tr>
                <td>{formatDate(payment.occurred_on)}</td>
                <td>{payment.description || "Pagamento"}</td>
                <td>{payment.bank_account_name}</td>
                <td class="mono">
                  {payment.allocations.length}
                  {payment.allocations.length === 1 ? "parcela" : "parcelas"}
                </td>
                <td class="mono align-right">
                  {formatCents(payment.amount_cents)}
                </td>
                <td class="mono align-right">
                  {payment.unallocated_cents > 0
                    ? formatCents(payment.unallocated_cents)
                    : "—"}
                </td>
                <td class="align-right">
                  <button
                    type="button"
                    class="button button--small button--danger"
                    onclick={() => confirmDeletePayment(payment.id)}
                    disabled={deletePaymentMutation.isPending}
                  >
                    Excluir
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}

    {#if hasUnallocated}
      <div class="inline-result" role="status">
        <span class="mono">CRÉDITO</span>
        <span>
          A parte não alocada do pagamento fica como crédito no cartão e não vira
          despesa.
        </span>
      </div>
    {/if}

    {#if actionError}
      <div class="inline-result inline-result--warn" role="alert">
        <span class="mono">ERRO</span>
        <span>{actionError}</span>
      </div>
    {/if}
  </section>
{/if}

{#if composer === "purchase" || composer === "payment"}
  <MovementFormModal
    {accounts}
    {categories}
    cards={cards}
    defaultCardId={cardId}
    defaultKind={composer === "purchase" ? "card_purchase" : "card_payment"}
    allowedKinds={["card_purchase", "card_payment"]}
    saving={purchaseMutation.isPending}
    error={purchaseMutation.error
      ? normalizeError(purchaseMutation.error).message
      : null}
    onsave={(request) => purchaseMutation.mutate(request)}
    onclose={closeComposer}
  />
{/if}
