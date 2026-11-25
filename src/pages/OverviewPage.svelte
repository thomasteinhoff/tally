<script lang="ts">
  import { createQuery } from "@tanstack/svelte-query";

  import {
    getDashboard,
    listAccounts,
    listCards,
    listMovements,
  } from "../lib/api";
  import { normalizeError } from "../lib/errors";
  import {
    accountTypeLabel,
    formatCents,
    formatDate,
    kindLabel,
    periodLabel,
  } from "../lib/format";

  /* Situação atual — nunca depende do mês do Quadro de análises. */
  const dashboardQuery = createQuery(() => ({
    queryKey: ["dashboard"],
    queryFn: getDashboard,
  }));

  const movementsQuery = createQuery(() => ({
    queryKey: ["movements", "overview"],
    queryFn: () => listMovements(),
  }));

  const accountsQuery = createQuery(() => ({
    queryKey: ["accounts", false],
    queryFn: () => listAccounts(false),
  }));

  const cardsQuery = createQuery(() => ({
    queryKey: ["cards"],
    queryFn: listCards,
  }));

  const dashboard = $derived(dashboardQuery.data);
  const movements = $derived(movementsQuery.data ?? []);
  const accounts = $derived(accountsQuery.data ?? []);
  const cashAccounts = $derived(
    accounts.filter(
      (account) =>
        account.account_type !== "credit_card" && !account.is_savings,
    ),
  );
  const savingsAccounts = $derived(
    accounts.filter((account) => account.is_savings && !account.archived_at),
  );
  const cards = $derived(cardsQuery.data ?? []);
  const activeCards = $derived(cards.filter((card) => !card.archived_at));
  const firstRun = $derived(accountsQuery.isSuccess && accounts.length === 0);
  const recentMovements = $derived(movements.slice(0, 5));
  const portfolioLoading = $derived(
    accountsQuery.isPending || cardsQuery.isPending,
  );
  const accountsError = $derived(
    accountsQuery.error ? normalizeError(accountsQuery.error) : null,
  );
  const cardsError = $derived(
    cardsQuery.error ? normalizeError(cardsQuery.error) : null,
  );

  const summaryError = $derived(
    dashboardQuery.error
      ? normalizeError(dashboardQuery.error)
      : movementsQuery.error
        ? normalizeError(movementsQuery.error)
        : null,
  );
  const summaryLoading = $derived(
    dashboardQuery.isPending || movementsQuery.isPending,
  );

  function refresh(): void {
    void dashboardQuery.refetch();
    void movementsQuery.refetch();
    void accountsQuery.refetch();
    void cardsQuery.refetch();
  }
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Visão do mês</span>
    <h2>O retrato do presente.</h2>
    <p>
      Patrimônio, contas e cartões no topo. Abaixo, o resumo do mês, os
      compromissos e os últimos lançamentos. Gráficos e comparações ficam no
      Quadro de análises.
    </p>
  </div>
  <div class="page-actions">
    <button type="button" class="button" onclick={refresh}>Atualizar</button>
    <a class="button button--primary" href="#/movimentacoes">Movimentar</a>
  </div>
</section>

{#if summaryLoading}
  <section class="panel" aria-busy="true">
    <div class="state-panel">
      <span class="state-kicker">Lendo</span>
      <h2>Consultando o ledger.</h2>
      <div class="loading-rule" aria-hidden="true"></div>
    </div>
  </section>
{:else if summaryError}
  <section class="panel">
    <div class="state-panel state-panel--error" role="alert">
      <span class="state-kicker">{summaryError.code}</span>
      <h2>Não foi possível carregar o resumo.</h2>
      <p>{summaryError.message}</p>
      {#if summaryError.details}
        <details class="details">
          <summary>Detalhes técnicos</summary>
          <pre>{summaryError.details}</pre>
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
  {#if firstRun}
    <section class="panel" aria-labelledby="first-run-title">
      <header class="panel-header">
        <div class="panel-heading">
          <h3 class="panel-title" id="first-run-title">Primeiros passos</h3>
        </div>
        <span class="panel-meta">3 etapas</span>
      </header>

      <div class="data-grid data-grid--auto">
        <div class="data-cell">
          <div class="data-label">01 · conta</div>
          <div class="data-value data-value--text">
            Crie sua primeira conta ou cartão em
            <a class="text-link" href="#/contas">Contas e cartões</a>.
          </div>
          <div class="data-note">
            O saldo inicial vira o ponto de partida do patrimônio.
          </div>
        </div>
        <div class="data-cell">
          <div class="data-label">02 · lançamento</div>
          <div class="data-value data-value--text">
            Registre despesas, receitas e transferências em
            <a class="text-link" href="#/movimentacoes">Movimentações</a>.
          </div>
          <div class="data-note">Ctrl+N abre o formulário em qualquer tela.</div>
        </div>
        <div class="data-cell">
          <div class="data-label">03 · segurança</div>
          <div class="data-value data-value--text">
            Crie um backup em
            <a class="text-link" href="#/configuracoes">Configurações</a>.
          </div>
          <div class="data-note">
            Tudo fica no seu computador, sem internet.
          </div>
        </div>
      </div>
    </section>
  {/if}

  <section class="ledger-hero" aria-label="Resumo financeiro">
    <div class="hero-main">
      <span class="hero-label">Patrimônio em contas</span>
      <strong
        class="hero-figure"
        class:amount-negative={dashboard.net_worth_cents < 0}
      >
        {formatCents(dashboard.net_worth_cents)}
      </strong>
      <span class="hero-note">ativos − passivos, sem cofrinhos</span>
    </div>

    <dl class="hero-stats">
      <div class="hero-stat">
        <dt>Ativos</dt>
        <dd class="mono">{formatCents(dashboard.assets_cents)}</dd>
      </div>
      <div class="hero-stat">
        <dt>Passivos</dt>
        <dd class="mono">{formatCents(dashboard.liabilities_cents)}</dd>
      </div>
      <div class="hero-stat">
        <dt>Cofrinhos</dt>
        <dd class="mono">{formatCents(dashboard.savings_cents)}</dd>
      </div>
      <div class="hero-stat">
        <dt>Total com cofrinhos</dt>
        <dd class="mono">
          {formatCents(dashboard.total_with_savings_cents)}
        </dd>
      </div>
      <div class="hero-stat">
        <dt>Resultado do mês</dt>
        <dd
          class="mono"
          class:amount-negative={dashboard.month_result_cents < 0}
        >
          {formatCents(dashboard.month_result_cents)}
        </dd>
      </div>
    </dl>
  </section>

  <div class="overview-grid">
    <section class="panel" aria-labelledby="overview-accounts-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">Disponível</span>
          <h3 class="panel-title" id="overview-accounts-title">Contas</h3>
        </div>
        <a class="text-link" href="#/contas">gerenciar</a>
      </header>

      {#if portfolioLoading}
        <div class="panel-body">
          <span class="panel-meta">Lendo contas…</span>
        </div>
      {:else if accountsError}
        <div class="panel-body" role="alert">
          <span class="amount-negative">{accountsError.message}</span>
        </div>
      {:else if cashAccounts.length === 0}
        <div class="empty-panel">
          <p>Nenhuma conta cadastrada.</p>
          <a class="button" href="#/contas">Criar conta</a>
        </div>
      {:else}
        <div class="table-wrap">
          <table class="data-table">
            <thead>
              <tr>
                <th scope="col">Conta</th>
                <th scope="col">Tipo</th>
                <th scope="col" class="align-right">Saldo</th>
              </tr>
            </thead>
            <tbody>
              {#each cashAccounts as account (account.id)}
                <tr>
                  <td>
                    <a class="text-link" href="#/contas/{account.id}">
                      {account.name}
                    </a>
                  </td>
                  <td>{accountTypeLabel(account.account_type)}</td>
                  <td
                    class="mono align-right"
                    class:amount-negative={account.balance_cents < 0}
                  >
                    {formatCents(account.balance_cents)}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      <footer class="panel-footer">
        <div class="panel-footnote">
          <span class="mono">saldo atual</span>
          <span>Não muda com o mês do Quadro de análises.</span>
        </div>
      </footer>
    </section>

    <section class="panel" aria-labelledby="overview-cards-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">Crédito</span>
          <h3 class="panel-title" id="overview-cards-title">Cartões</h3>
        </div>
        <a class="text-link" href="#/cartoes">gerenciar</a>
      </header>

      {#if portfolioLoading}
        <div class="panel-body">
          <span class="panel-meta">Lendo cartões…</span>
        </div>
      {:else if cardsError}
        <div class="panel-body" role="alert">
          <span class="amount-negative">{cardsError.message}</span>
        </div>
      {:else if activeCards.length === 0}
        <div class="empty-panel">
          <p>Nenhum cartão ativo.</p>
          <a class="button" href="#/cartoes">Criar cartão</a>
        </div>
      {:else}
        <div class="table-wrap">
          <table class="data-table">
            <thead>
              <tr>
                <th scope="col">Cartão</th>
                <th scope="col" class="align-right">Limite</th>
                <th scope="col" class="align-right">Disponível</th>
                <th scope="col" class="align-right">Dívida</th>
              </tr>
            </thead>
            <tbody>
              {#each activeCards as card (card.id)}
                <tr>
                  <td>
                    <a class="text-link" href="#/cartoes/{card.id}">
                      {card.name}
                    </a>
                  </td>
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
                  <td class="mono align-right">{formatCents(card.debt_cents)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      <footer class="panel-footer">
        <div class="panel-footnote">
          <span class="mono">limite</span>
          <span>Informativo; não soma ao patrimônio.</span>
        </div>
      </footer>
    </section>
  </div>

  <section class="panel" aria-labelledby="overview-savings-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">Rendimento</span>
        <h3 class="panel-title" id="overview-savings-title">Cofrinhos</h3>
      </div>
      <a class="text-link" href="#/contas">gerenciar</a>
    </header>

    {#if savingsAccounts.length === 0}
      <div class="empty-panel">
        <p>Nenhum cofrinho ativo.</p>
        <a class="button" href="#/contas">Criar cofrinho</a>
      </div>
    {:else}
      <div class="table-wrap">
        <table class="data-table">
          <thead>
            <tr>
              <th scope="col">Cofrinho</th>
              <th scope="col">Modalidade</th>
              <th scope="col" class="align-right">Saldo</th>
            </tr>
          </thead>
          <tbody>
            {#each savingsAccounts as account (account.id)}
              <tr>
                <td>
                  <a class="text-link" href="#/cofrinhos/{account.id}">
                    {account.name}
                  </a>
                </td>
                <td>
                  {account.yield_mode === "business"
                    ? "Dias úteis"
                    : "Todos os dias"}
                  · {account.cdi_percent_bps !== null
                    ? `${account.cdi_percent_bps / 100}% do CDI`
                    : "—"}
                </td>
                <td class="mono align-right">{formatCents(account.balance_cents)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}

    <footer class="panel-footer">
      <div class="panel-footnote">
        <span class="mono">fora do fluxo de caixa</span>
        <span>Aplicações e resgates são transferências; rendimentos entram no total com cofrinhos.</span>
      </div>
    </footer>
  </section>

  <div class="overview-grid">
    <section class="panel" aria-labelledby="overview-month-title">
      <header class="panel-header">
        <div class="panel-heading">
          <h3 class="panel-title" id="overview-month-title">Resumo do mês</h3>
        </div>
        <span class="panel-meta">
          {formatDate(dashboard.month_from)} – {formatDate(dashboard.month_to)}
        </span>
      </header>

      <div class="data-grid data-grid--summary">
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
          <div class="data-label">Rendimento dos cofrinhos</div>
          <div
            class="data-value"
            class:amount-negative={dashboard.month_savings_yield_cents < 0}
          >
            {formatCents(dashboard.month_savings_yield_cents)}
          </div>
          <div class="data-note">
            líquido de estornos · não é receita nem fluxo de caixa
          </div>
        </div>
        <div class="data-cell">
          <div class="data-label">Dívida no cartão</div>
          <div class="data-value">
            {formatCents(dashboard.card_debt_cents)}
          </div>
          <div class="data-note">
            crédito a favor {formatCents(dashboard.card_credit_cents)}
          </div>
        </div>
      </div>
    </section>

    <section class="panel" aria-labelledby="overview-commitments-title">
      <header class="panel-header">
        <div class="panel-heading">
          <h3 class="panel-title" id="overview-commitments-title">
            Compromissos
          </h3>
        </div>
        <span class="panel-meta">próximos 12 meses</span>
      </header>

      <div class="data-grid data-grid--auto">
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
          <div class="data-label">Comprometido no futuro</div>
          <div class="data-value">
            {formatCents(dashboard.future_commitments_cents)}
          </div>
        </div>
      </div>

      <footer class="panel-footer">
        <div class="panel-footnote">
          <span class="mono">parcelas</span>
          <span>Detalhamento completo em Parcelamentos.</span>
        </div>
      </footer>
    </section>
  </div>

  <section class="panel" aria-labelledby="overview-movements-title">
    <header class="panel-header">
      <div class="panel-heading">
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
              class:amount-negative={movement.amount_cents < 0 &&
                movement.kind !== "transfer"}
              class:amount-positive={movement.amount_cents > 0 &&
                movement.kind !== "transfer"}
              class:amount-neutral={movement.kind === "transfer"}
            >
              {formatCents(movement.amount_cents)}
            </span>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}
