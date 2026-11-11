<script lang="ts">
  import { createQuery } from "@tanstack/svelte-query";

  import { getDashboard, listMovements } from "../lib/api";
  import { normalizeError } from "../lib/errors";
  import {
    formatCents,
    formatDate,
    formatMonth,
    kindLabel,
    periodLabel,
  } from "../lib/format";

  const dashboardQuery = createQuery(() => ({
    queryKey: ["dashboard"],
    queryFn: getDashboard,
  }));

  const movementsQuery = createQuery(() => ({
    queryKey: ["movements", "overview"],
    queryFn: () => listMovements(),
  }));

  const dashboard = $derived(dashboardQuery.data);
  const movements = $derived(movementsQuery.data ?? []);
  const recentMovements = $derived(movements.slice(0, 8));
  const queryError = $derived(
    dashboardQuery.error
      ? normalizeError(dashboardQuery.error)
      : movementsQuery.error
        ? normalizeError(movementsQuery.error)
        : null,
  );
  const loading = $derived(
    dashboardQuery.isPending || movementsQuery.isPending,
  );

  function refresh(): void {
    void dashboardQuery.refetch();
    void movementsQuery.refetch();
  }
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Visão geral</span>
    <h2>O retrato do presente.</h2>
    <p>
      Patrimônio, resultado do mês e compromissos calculados no Rust a partir dos
      lançamentos publicados. Nenhum número desta tela é autoridade no frontend.
    </p>
  </div>
  <div class="page-actions">
    <a class="button" href="#/relatorios">Relatórios</a>
    <a class="button button--primary" href="#/movimentacoes">Movimentar</a>
  </div>
</section>

{#if loading}
  <section class="panel" aria-busy="true">
    <div class="state-panel">
      <span class="state-kicker">Lendo</span>
      <h2>Consultando o ledger.</h2>
      <div class="loading-rule" aria-hidden="true"></div>
    </div>
  </section>
{:else if queryError}
  <section class="panel">
    <div class="state-panel state-panel--error" role="alert">
      <span class="state-kicker">{queryError.code}</span>
      <h2>Não foi possível carregar a visão geral.</h2>
      <p>{queryError.message}</p>
      {#if queryError.details}
        <details class="details">
          <summary>Detalhes técnicos</summary>
          <pre>{queryError.details}</pre>
        </details>
      {/if}
      <div class="state-actions">
        <button type="button" class="button button--primary" onclick={refresh}>
          Tentar novamente
        </button>
      </div>
    </div>
  </section>
{:else if dashboard}
  <section class="summary-strip summary-strip--hero" aria-label="Resumo financeiro">
    <div>
      <span class="data-label">Patrimônio líquido</span>
      <strong
        class="hero-value"
        class:amount-negative={dashboard.net_worth_cents < 0}
      >
        {formatCents(dashboard.net_worth_cents)}
      </strong>
    </div>
    <div>
      <span class="data-label">Resultado do mês</span>
      <strong
        class="summary-value"
        class:amount-negative={dashboard.month_result_cents < 0}
      >
        {formatCents(dashboard.month_result_cents)}
      </strong>
    </div>
    <div>
      <span class="data-label">Dívida no cartão</span>
      <strong class="summary-value">
        {formatCents(dashboard.card_debt_cents)}
      </strong>
    </div>
    <div class="summary-note">
      <span class="mono">
        {formatMonth(dashboard.month_from.slice(0, 7))} · até
        {formatDate(dashboard.as_of)}
      </span>
      <span>ativos − passivos</span>
    </div>
  </section>

  <div class="overview-grid">
    <section class="panel" aria-labelledby="overview-month-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">01 · mês</span>
          <h3 class="panel-title" id="overview-month-title">Resultado</h3>
        </div>
        <span class="panel-meta">
          {formatDate(dashboard.month_from)} · {formatDate(dashboard.month_to)}
        </span>
      </header>

      <div class="data-grid">
        <div class="data-cell">
          <div class="data-label">Receitas</div>
          <div class="data-value amount-positive">
            {formatCents(dashboard.month_income_cents)}
          </div>
        </div>
        <div class="data-cell">
          <div class="data-label">Despesas</div>
          <div class="data-value amount-negative">
            {formatCents(dashboard.month_expense_cents)}
          </div>
        </div>
        <div class="data-cell">
          <div class="data-label">Resultado</div>
          <div
            class="data-value"
            class:amount-negative={dashboard.month_result_cents < 0}
          >
            {formatCents(dashboard.month_result_cents)}
          </div>
        </div>
        <div class="data-cell">
          <div class="data-label">Crédito no cartão</div>
          <div class="data-value">
            {formatCents(dashboard.card_credit_cents)}
          </div>
          <div class="data-note">saldo a favor</div>
        </div>
      </div>
    </section>

    <section class="panel" aria-labelledby="overview-commitments-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">02 · futuro</span>
          <h3 class="panel-title" id="overview-commitments-title">
            Compromissos
          </h3>
        </div>
        <a class="text-link" href="#/relatorios">ver relatório</a>
      </header>

      <div class="data-grid">
        <div class="data-cell">
          <div class="data-label">Parcelas vencidas</div>
          <div
            class="data-value"
            class:amount-negative={dashboard.overdue_installments_cents > 0}
          >
            {formatCents(dashboard.overdue_installments_cents)}
          </div>
        </div>
        <div class="data-cell">
          <div class="data-label">Próximos 12 meses</div>
          <div class="data-value">
            {formatCents(dashboard.future_commitments_cents)}
          </div>
        </div>
        <div class="data-cell">
          <div class="data-label">Ativos</div>
          <div class="data-value">{formatCents(dashboard.assets_cents)}</div>
        </div>
        <div class="data-cell">
          <div class="data-label">Passivos</div>
          <div class="data-value">
            {formatCents(dashboard.liabilities_cents)}
          </div>
        </div>
      </div>
    </section>
  </div>

  <div class="overview-grid">
    <section class="panel" aria-labelledby="overview-top-expenses-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">03 · saídas</span>
          <h3 class="panel-title" id="overview-top-expenses-title">
            Maiores despesas do mês
          </h3>
        </div>
        <a class="text-link" href="#/relatorios">ver todas</a>
      </header>

      {#if dashboard.top_expenses.length === 0}
        <div class="empty-panel">
          <p>Nenhuma despesa registrada neste mês.</p>
        </div>
      {:else}
        <div class="table-wrap">
          <table class="data-table">
            <thead>
              <tr>
                <th scope="col">Categoria</th>
                <th scope="col" class="align-right">Lançamentos</th>
                <th scope="col" class="align-right">Total</th>
              </tr>
            </thead>
            <tbody>
              {#each dashboard.top_expenses as item (item.category_id ?? "none")}
                <tr>
                  <td>{item.category_name}</td>
                  <td class="mono align-right">{item.entry_count}</td>
                  <td class="mono align-right">
                    {formatCents(item.total_cents)}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>

    <section class="panel" aria-labelledby="overview-movements-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">04 · livro</span>
          <h3 class="panel-title" id="overview-movements-title">
            Últimos lançamentos
          </h3>
        </div>
        <a class="text-link" href="#/movimentacoes">ver livro</a>
      </header>

      {#if recentMovements.length === 0}
        <div class="empty-panel">
          <p>Nenhuma movimentação. O saldo inicial não aparece no livro.</p>
          <a class="button" href="#/movimentacoes">Nova movimentação</a>
        </div>
      {:else}
        <ul class="movement-list">
          {#each recentMovements as movement (movement.id)}
            <li>
              <div class="movement-list-main">
                <span>{movement.description || kindLabel(movement.kind)}</span>
                <small>
                  {formatDate(movement.occurred_on)} ·
                  {periodLabel(movement.period)} ·
                  {movement.primary_account_name}
                </small>
              </div>
              <span
                class="mono"
                class:amount-negative={movement.amount_cents < 0}
                class:amount-positive={movement.amount_cents > 0}
              >
                {formatCents(movement.amount_cents)}
              </span>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>
{/if}
