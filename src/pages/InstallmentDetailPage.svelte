<script lang="ts">
  import {
    createMutation,
    createQuery,
    useQueryClient,
  } from "@tanstack/svelte-query";

  import { deleteInstallmentPlan, getInstallmentPlan } from "../lib/api";
  import { confirmAction } from "../lib/confirm";
  import { normalizeError } from "../lib/errors";
  import {
    formatCents,
    formatDate,
    installmentStatusLabel,
    paymentLabel,
    periodLabel,
  } from "../lib/format";
  import { notify } from "../lib/toast.svelte";

  let { planId }: { planId: string } = $props();

  const queryClient = useQueryClient();

  const planQuery = createQuery(() => ({
    queryKey: ["installment-plans", "detail", planId],
    queryFn: () => getInstallmentPlan(planId),
  }));

  const deleteMutation = createMutation(() => ({
    mutationFn: deleteInstallmentPlan,
    onSuccess: () => {
      void queryClient.invalidateQueries();
      notify("Parcelamento excluído.");
      window.location.hash = "#/parcelamentos";
    },
  }));

  const plan = $derived(planQuery.data);
  const queryError = $derived(
    planQuery.error ? normalizeError(planQuery.error) : null,
  );
  const actionError = $derived(
    deleteMutation.error ? normalizeError(deleteMutation.error).message : null,
  );

  async function confirmDelete(): Promise<void> {
    if (!plan) {
      return;
    }
    const confirmed = await confirmAction(
      `Excluir o parcelamento “${plan.description || plan.category_name}”? Só é permitido quando não há pagamentos alocados. As parcelas e os reconhecimentos serão removidos.`,
      "Excluir parcelamento",
      "Excluir",
    );
    if (confirmed) {
      deleteMutation.mutate(plan.id);
    }
  }
</script>

<section class="page-intro">
  <div>
    <a class="text-link text-link--back" href="#/parcelamentos">
      ← Parcelamentos
    </a>
    <span class="eyebrow">Parcelamento</span>
    <h2>{plan?.description || plan?.category_name || "Parcelamento"}</h2>
    {#if plan}
      <p>
        {plan.category_name} · compra em {formatDate(plan.purchased_on)} ·{" "}
        {periodLabel(plan.period)} · {paymentLabel(plan.payment_method)}
      </p>
    {/if}
  </div>
  <div class="page-actions">
    {#if plan}
      <a class="button" href="#/cartoes/{plan.card_account_id}">Ver cartão</a>
      <button
        type="button"
        class="button button--danger"
        onclick={confirmDelete}
        disabled={deleteMutation.isPending || plan.paid_cents > 0}
      >
        Excluir
      </button>
    {/if}
  </div>
</section>

{#if planQuery.isPending}
  <section class="panel" aria-busy="true">
    <div class="state-panel">
      <span class="state-kicker">Lendo</span>
      <h2>Consultando o parcelamento.</h2>
      <div class="loading-rule" aria-hidden="true"></div>
    </div>
  </section>
{:else if queryError}
  <section class="panel">
    <div class="state-panel state-panel--error" role="alert">
      <span class="state-kicker">{queryError.code}</span>
      <h2>Não foi possível carregar o parcelamento.</h2>
      <p>{queryError.message}</p>
      <div class="state-actions">
        <button
          type="button"
          class="button button--primary"
          onclick={() => planQuery.refetch()}
        >
          Tentar novamente
        </button>
      </div>
    </div>
  </section>
{:else if plan}
  <section class="summary-strip" aria-label="Resumo do parcelamento">
    <div>
      <span class="data-label">Total</span>
      <strong class="hero-value">{formatCents(plan.total_cents)}</strong>
    </div>
    <div>
      <span class="data-label">Pago</span>
      <strong class="summary-value">{formatCents(plan.paid_cents)}</strong>
    </div>
    <div>
      <span class="data-label">Em aberto</span>
      <strong class="summary-value">
        {formatCents(plan.outstanding_cents)}
      </strong>
    </div>
    <div class="summary-note">
      <span class="mono">
        {plan.installment_count}x · cartão
        <a class="text-link" href="#/cartoes/{plan.card_account_id}">
          {plan.card_account_name}
        </a>
      </span>
    </div>
  </section>

  {#if plan.paid_cents > 0}
    <div class="inline-result inline-result--warn" role="status">
      <span class="mono">BLOQUEADO</span>
      <span>
        Este parcelamento possui pagamentos. Remova os pagamentos no cartão
        antes de excluir.
      </span>
    </div>
  {/if}

  <section class="panel" aria-labelledby="installments-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">Cronograma</span>
        <h3 class="panel-title" id="installments-title">Parcelas</h3>
      </div>
      <span class="panel-meta">{plan.installment_count} parcelas</span>
    </header>

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

    {#if actionError}
      <div class="inline-result inline-result--warn" role="alert">
        <span class="mono">ERRO</span>
        <span>{actionError}</span>
      </div>
    {/if}
  </section>
{/if}
