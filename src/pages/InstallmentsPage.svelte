<script lang="ts">
  import { createQuery } from "@tanstack/svelte-query";

  import { listInstallmentPlans } from "../lib/api";
  import { normalizeError } from "../lib/errors";
  import { formatCents, formatDate } from "../lib/format";

  const plansQuery = createQuery(() => ({
    queryKey: ["installment-plans", null],
    queryFn: () => listInstallmentPlans(null),
  }));

  const plans = $derived(plansQuery.data ?? []);
  const totalOutstandingCents = $derived(
    plans.reduce((total, plan) => total + plan.outstanding_cents, 0),
  );
  const openInstallments = $derived(
    plans.reduce(
      (total, plan) =>
        total +
        plan.installments.filter((installment) => installment.status !== "paid")
          .length,
      0,
    ),
  );
  const overdueInstallments = $derived(
    plans.reduce(
      (total, plan) =>
        total +
        plan.installments.filter(
          (installment) => installment.status === "overdue",
        ).length,
      0,
    ),
  );
  const queryError = $derived(
    plansQuery.error ? normalizeError(plansQuery.error) : null,
  );

  function paidCount(plan: (typeof plans)[number]): number {
    return plan.installments.filter(
      (installment) => installment.status === "paid",
    ).length;
  }
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Compromissos</span>
    <h2>Parcelamentos</h2>
    <p>
      Cada compra parcelada reconhece uma parcela por vencimento. A dívida total
      já inclui as parcelas futuras; elas não são despesa do mês da compra.
    </p>
  </div>
  <div class="page-actions">
    <a class="button" href="#/cartoes">Cartões</a>
  </div>
</section>

<section class="summary-strip" aria-label="Resumo dos parcelamentos">
  <div>
    <span class="data-label">Em aberto</span>
    <strong class="summary-value">{formatCents(totalOutstandingCents)}</strong>
  </div>
  <div>
    <span class="data-label">Parcelamentos</span>
    <strong class="summary-value">{plans.length}</strong>
  </div>
  <div>
    <span class="data-label">Parcelas em aberto</span>
    <strong class="summary-value">{openInstallments}</strong>
  </div>
  <div>
    <span class="data-label">Vencidas</span>
    <strong class="summary-value">{overdueInstallments}</strong>
  </div>
</section>

<section class="panel" aria-labelledby="plans-title">
  <header class="panel-header">
    <div class="panel-heading">
      <span class="panel-kicker">Compras</span>
      <h3 class="panel-title" id="plans-title">Parcelamentos</h3>
    </div>
    <span class="panel-meta">
      {plans.length} {plans.length === 1 ? "compra" : "compras"}
    </span>
  </header>

  {#if plansQuery.isPending}
    <div class="state-panel" aria-busy="true">
      <span class="state-kicker">Lendo</span>
      <h2>Consultando parcelamentos.</h2>
      <div class="loading-rule" aria-hidden="true"></div>
    </div>
  {:else if queryError}
    <div class="state-panel state-panel--error" role="alert">
      <span class="state-kicker">{queryError.code}</span>
      <h2>Não foi possível carregar os parcelamentos.</h2>
      <p>{queryError.message}</p>
      <div class="state-actions">
        <button
          type="button"
          class="button button--primary"
          onclick={() => plansQuery.refetch()}
        >
          Tentar novamente
        </button>
      </div>
    </div>
  {:else if plans.length === 0}
    <div class="empty-panel">
      <p>Nenhuma compra parcelada registrada.</p>
      <a class="button" href="#/cartoes">Ir para cartões</a>
    </div>
  {:else}
    <div class="table-wrap">
      <table class="data-table">
        <thead>
          <tr>
            <th scope="col">Compra</th>
            <th scope="col">Cartão</th>
            <th scope="col">Parcelas</th>
            <th scope="col">Primeira</th>
            <th scope="col" class="align-right">Total</th>
            <th scope="col" class="align-right">Pago</th>
            <th scope="col" class="align-right">Em aberto</th>
          </tr>
        </thead>
        <tbody>
          {#each plans as plan (plan.id)}
            <tr>
              <td>
                <div class="cell-stack">
                  <a class="text-link" href="#/parcelamentos/{plan.id}">
                    {plan.description || plan.category_name}
                  </a>
                  <small class="mono">{plan.category_name}</small>
                </div>
              </td>
              <td>
                <a class="text-link" href="#/cartoes/{plan.card_account_id}">
                  {plan.card_account_name}
                </a>
              </td>
              <td class="mono">
                {paidCount(plan)}/{plan.installment_count}
              </td>
              <td>{formatDate(plan.first_due_on)}</td>
              <td class="mono align-right">{formatCents(plan.total_cents)}</td>
              <td class="mono align-right">{formatCents(plan.paid_cents)}</td>
              <td class="mono align-right">
                {formatCents(plan.outstanding_cents)}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</section>
