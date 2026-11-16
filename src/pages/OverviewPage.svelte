<script lang="ts">
  import { createQuery } from "@tanstack/svelte-query";

  import type { ReportFilter } from "../lib/api/bindings";
  import {
    getCashFlowReport,
    getCommitmentsReport,
    getDashboard,
    getIncomeReport,
    getNetWorthReport,
    getSpendingReport,
    listAccounts,
    listCards,
    listCategories,
    listMovements,
  } from "../lib/api";
  import { buildCategoryBars } from "../lib/category-chart";
  import CategoryBars from "../lib/components/CategoryBars.svelte";
  import { normalizeError } from "../lib/errors";
  import {
    accountTypeLabel,
    formatCents,
    formatDate,
    formatMonth,
    kindLabel,
    paymentLabel,
    periodLabel,
  } from "../lib/format";

  /* Situação atual — nunca depende dos filtros de análise. */
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

  /* Análises — filtros próprios, abaixo do resumo. */
  let from = $state("");
  let to = $state("");
  let period = $state("");
  let categoryId = $state("");
  let accountId = $state("");
  let paymentMethod = $state("");

  const filter = $derived<ReportFilter>({
    from: from || null,
    to: to || null,
    period: period || null,
    category_id: categoryId || null,
    account_id: accountId || null,
    payment_method: paymentMethod || null,
  });
  const asOf = $derived(to || null);
  const filterActive = $derived(
    Boolean(from || to || period || categoryId || accountId || paymentMethod),
  );

  const categoriesQuery = createQuery(() => ({
    queryKey: ["categories", true],
    queryFn: () => listCategories(true),
  }));

  const reportAccountsQuery = createQuery(() => ({
    queryKey: ["accounts", true],
    queryFn: () => listAccounts(true),
  }));

  const spendingQuery = createQuery(() => ({
    queryKey: ["reports", "spending", filter],
    queryFn: () => getSpendingReport(filter),
  }));

  const incomeQuery = createQuery(() => ({
    queryKey: ["reports", "income", filter],
    queryFn: () => getIncomeReport(filter),
  }));

  const cashFlowQuery = createQuery(() => ({
    queryKey: ["reports", "cash-flow", filter],
    queryFn: () => getCashFlowReport(filter),
  }));

  /* Análise histórica só existe quando uma data final foi escolhida;
     sem data, a posição atual do topo já cobre o indicador. */
  const netWorthQuery = createQuery(() => ({
    queryKey: ["reports", "net-worth", asOf],
    queryFn: () => getNetWorthReport(asOf),
    enabled: Boolean(asOf),
  }));

  const commitmentsQuery = createQuery(() => ({
    queryKey: ["reports", "commitments", asOf],
    queryFn: () => getCommitmentsReport(asOf, 12),
  }));

  const dashboard = $derived(dashboardQuery.data);
  const movements = $derived(movementsQuery.data ?? []);
  const accounts = $derived(accountsQuery.data ?? []);
  const cashAccounts = $derived(
    accounts.filter((account) => account.account_type !== "credit_card"),
  );
  const cards = $derived(cardsQuery.data ?? []);
  const activeCards = $derived(cards.filter((card) => !card.archived_at));
  const firstRun = $derived(accountsQuery.isSuccess && accounts.length === 0);
  const recentMovements = $derived(movements.slice(0, 10));
  const portfolioLoading = $derived(
    accountsQuery.isPending || cardsQuery.isPending,
  );
  const accountsError = $derived(
    accountsQuery.error ? normalizeError(accountsQuery.error) : null,
  );
  const cardsError = $derived(
    cardsQuery.error ? normalizeError(cardsQuery.error) : null,
  );

  const categories = $derived(categoriesQuery.data ?? []);
  const reportAccounts = $derived(reportAccountsQuery.data ?? []);
  const spending = $derived(spendingQuery.data);
  const income = $derived(incomeQuery.data);
  const cashFlow = $derived(cashFlowQuery.data);
  const netWorth = $derived(netWorthQuery.data);
  const commitments = $derived(commitmentsQuery.data);

  const spendingBars = $derived(buildCategoryBars(spending?.categories ?? []));
  const incomeBars = $derived(buildCategoryBars(income?.categories ?? []));

  const analysisRange = $derived(
    spending ? `${formatDate(spending.from)} – ${formatDate(spending.to)}` : "—",
  );
  const analysisScope = $derived(
    from || to ? "intervalo selecionado" : "mês corrente",
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
  const analysisError = $derived(
    spendingQuery.error
      ? normalizeError(spendingQuery.error)
      : incomeQuery.error
        ? normalizeError(incomeQuery.error)
        : cashFlowQuery.error
          ? normalizeError(cashFlowQuery.error)
          : netWorthQuery.error
            ? normalizeError(netWorthQuery.error)
            : commitmentsQuery.error
              ? normalizeError(commitmentsQuery.error)
              : null,
  );

  const periodOptions = [
    ["", "Todos"],
    ["madrugada", "Madrugada"],
    ["manha", "Manhã"],
    ["tarde", "Tarde"],
    ["noite", "Noite"],
    ["custom", "Personalizado"],
  ];

  const paymentOptions = [
    "pix",
    "debit",
    "credit",
    "subscription_credit",
    "cash",
  ];

  function clearFilters(): void {
    from = "";
    to = "";
    period = "";
    categoryId = "";
    accountId = "";
    paymentMethod = "";
  }

  function refresh(): void {
    void dashboardQuery.refetch();
    void movementsQuery.refetch();
    void accountsQuery.refetch();
    void cardsQuery.refetch();
    void spendingQuery.refetch();
    void incomeQuery.refetch();
    void cashFlowQuery.refetch();
    void commitmentsQuery.refetch();
    if (asOf) {
      void netWorthQuery.refetch();
    }
  }
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Visão do mês</span>
    <h2>O retrato do presente.</h2>
    <p>
      Patrimônio, contas e cartões no topo. Abaixo, o resumo do mês e as
      análises de competência e caixa. Todo número é calculado no Rust a partir
      dos lançamentos publicados.
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
      <span class="hero-label">Patrimônio líquido</span>
      <strong
        class="hero-figure"
        class:amount-negative={dashboard.net_worth_cents < 0}
      >
        {formatCents(dashboard.net_worth_cents)}
      </strong>
      <span class="hero-note">ativos − passivos</span>
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
          <span>Não muda com o período de análise.</span>
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

      <div class="data-grid data-grid--auto">
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
          <span>Detalhamento mês a mês nas análises abaixo.</span>
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

<section class="analyses" aria-labelledby="analyses-title">
  <header class="section-head">
    <div>
      <span class="eyebrow">Análises</span>
      <h3 id="analyses-title">Competência e caixa.</h3>
    </div>
    <p>
      Despesas e receitas usam o período do lançamento; parcelas contam no
      vencimento. Fluxo de caixa mostra o dinheiro que entrou e saiu das contas,
      sem transferências nem saldos iniciais.
    </p>
  </header>

  <section class="panel" aria-labelledby="report-filters-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">Filtros das análises</span>
        <h3 class="panel-title" id="report-filters-title">Período de análise</h3>
      </div>
      <span class="panel-meta">
        {analysisScope} · {analysisRange}
      </span>
    </header>

    <p class="filter-scope">
      <span class="mono">escopo</span>
      <span>
        Estes filtros valem apenas para as análises abaixo. Patrimônio, saldos
        das contas e limites dos cartões permanecem atuais.
      </span>
    </p>

    <div class="filters">
      <label class="field">
        <span>
          De
          <small>{from ? formatDate(from) : "dd/mm/aaaa"}</small>
        </span>
        <input type="date" lang="pt-BR" bind:value={from} />
      </label>
      <label class="field">
        <span>
          Até
          <small>{to ? formatDate(to) : "dd/mm/aaaa"}</small>
        </span>
        <input type="date" lang="pt-BR" bind:value={to} />
      </label>
      <label class="field">
        <span>Período do dia</span>
        <select bind:value={period}>
          {#each periodOptions as [value, label] (value)}
            <option value={value}>{label}</option>
          {/each}
        </select>
      </label>
      <label class="field">
        <span>Categoria</span>
        <select bind:value={categoryId}>
          <option value="">Todas</option>
          {#each categories as category (category.id)}
            <option value={category.id}>
              {kindLabel(category.kind)} · {category.name}
            </option>
          {/each}
        </select>
      </label>
      <label class="field">
        <span>Conta</span>
        <select bind:value={accountId}>
          <option value="">Todas</option>
          {#each reportAccounts as account (account.id)}
            <option value={account.id}>
              {account.name}{account.archived_at ? " (arquivada)" : ""}
            </option>
          {/each}
        </select>
      </label>
      <label class="field">
        <span>Forma de pagamento</span>
        <select bind:value={paymentMethod}>
          <option value="">Todas</option>
          {#each paymentOptions as method (method)}
            <option value={method}>{paymentLabel(method)}</option>
          {/each}
        </select>
      </label>
    </div>

    <footer class="panel-footer">
      <div class="panel-footnote">
        <span class="mono">sem datas</span>
        <span>
          A análise usa o mês corrente. Conta e forma de pagamento afetam
          gastos, receitas e caixa; patrimônio e compromissos usam a data final.
        </span>
      </div>
      <div class="actions">
        <button
          type="button"
          class="button"
          onclick={clearFilters}
          disabled={!filterActive}
        >
          Limpar filtros
        </button>
      </div>
    </footer>
  </section>

  {#if analysisError}
    <section class="panel">
      <div class="state-panel state-panel--error" role="alert">
        <span class="state-kicker">{analysisError.code}</span>
        <h2>Não foi possível carregar as análises.</h2>
        <p>{analysisError.message}</p>
        {#if analysisError.details}
          <details class="details">
            <summary>Detalhes técnicos</summary>
            <pre>{analysisError.details}</pre>
          </details>
        {/if}
        <div class="state-actions">
          <button type="button" class="button button--primary" onclick={refresh}>
            Tentar novamente
          </button>
        </div>
      </div>
    </section>
  {:else}
    <div class="overview-grid">
      <section class="panel" aria-labelledby="spending-title">
        <header class="panel-header">
          <div class="panel-heading">
            <span class="panel-kicker">Saídas</span>
            <h3 class="panel-title" id="spending-title">Gastos por categoria</h3>
          </div>
        </header>

        {#if spendingQuery.isPending}
          <div class="state-panel" aria-busy="true">
            <span class="state-kicker">Lendo</span>
            <h2>Calculando gastos.</h2>
            <div class="loading-rule" aria-hidden="true"></div>
          </div>
        {:else if !spending || spending.categories.length === 0}
          <div class="empty-panel">
            <p>Nenhuma despesa no período selecionado.</p>
          </div>
        {:else}
          <CategoryBars
            bars={spendingBars}
            totalCents={spending.total_cents}
            entryCount={spending.entry_count}
            tone="expense"
          />

          <details class="chart-details">
            <summary>
              Detalhamento completo · {spending.categories.length}
              {spending.categories.length === 1 ? "categoria" : "categorias"}
            </summary>
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
                  {#each spending.categories as item (item.category_id ?? "none")}
                    <tr>
                      <td>{item.category_name}</td>
                      <td class="mono align-right">{item.entry_count}</td>
                      <td class="mono align-right">
                        {formatCents(item.total_cents)}
                      </td>
                    </tr>
                  {/each}
                </tbody>
                <tfoot>
                  <tr>
                    <td>Total</td>
                    <td class="mono align-right">{spending.entry_count}</td>
                    <td class="mono align-right">
                      {formatCents(spending.total_cents)}
                    </td>
                  </tr>
                </tfoot>
              </table>
            </div>
          </details>
        {/if}
      </section>

      <section class="panel" aria-labelledby="income-title">
        <header class="panel-header">
          <div class="panel-heading">
            <span class="panel-kicker">Entradas</span>
            <h3 class="panel-title" id="income-title">Receitas por categoria</h3>
          </div>
        </header>

        {#if incomeQuery.isPending}
          <div class="state-panel" aria-busy="true">
            <span class="state-kicker">Lendo</span>
            <h2>Calculando receitas.</h2>
            <div class="loading-rule" aria-hidden="true"></div>
          </div>
        {:else if !income || income.categories.length === 0}
          <div class="empty-panel">
            <p>Nenhuma receita no período selecionado.</p>
          </div>
        {:else}
          <CategoryBars
            bars={incomeBars}
            totalCents={income.total_cents}
            entryCount={income.entry_count}
            tone="income"
          />

          <details class="chart-details">
            <summary>
              Detalhamento completo · {income.categories.length}
              {income.categories.length === 1 ? "categoria" : "categorias"}
            </summary>
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
                  {#each income.categories as item (item.category_id ?? "none")}
                    <tr>
                      <td>{item.category_name}</td>
                      <td class="mono align-right">{item.entry_count}</td>
                      <td class="mono align-right">
                        {formatCents(item.total_cents)}
                      </td>
                    </tr>
                  {/each}
                </tbody>
                <tfoot>
                  <tr>
                    <td>Total</td>
                    <td class="mono align-right">{income.entry_count}</td>
                    <td class="mono align-right">
                      {formatCents(income.total_cents)}
                    </td>
                  </tr>
                </tfoot>
              </table>
            </div>
          </details>
        {/if}
      </section>
    </div>

    <section class="panel" aria-labelledby="cash-flow-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">Caixa</span>
          <h3 class="panel-title" id="cash-flow-title">Fluxo de caixa</h3>
        </div>
        <span class="panel-meta">
          {cashFlow ? formatDate(cashFlow.from) : "—"} · {cashFlow
            ? formatDate(cashFlow.to)
            : "—"}
        </span>
      </header>

      {#if cashFlowQuery.isPending}
        <div class="state-panel" aria-busy="true">
          <span class="state-kicker">Lendo</span>
          <h2>Calculando o caixa.</h2>
          <div class="loading-rule" aria-hidden="true"></div>
        </div>
      {:else if !cashFlow || cashFlow.months.length === 0}
        <div class="empty-panel">
          <p>Nenhuma entrada ou saída de caixa no período.</p>
        </div>
      {:else}
        <div class="data-grid">
          <div class="data-cell">
            <div class="data-label">Entradas</div>
            <div class="data-value amount-positive">
              {formatCents(cashFlow.inflow_cents)}
            </div>
          </div>
          <div class="data-cell">
            <div class="data-label">Saídas</div>
            <div class="data-value amount-negative">
              {formatCents(cashFlow.outflow_cents)}
            </div>
          </div>
          <div class="data-cell">
            <div class="data-label">Líquido</div>
            <div
              class="data-value"
              class:amount-negative={cashFlow.net_cents < 0}
            >
              {formatCents(cashFlow.net_cents)}
            </div>
          </div>
          <div class="data-cell">
            <div class="data-label">Período</div>
            <div class="data-value">{periodLabel(cashFlow.period)}</div>
            <div class="data-note">transferências não entram no caixa</div>
          </div>
        </div>

        <div class="table-wrap">
          <table class="data-table">
            <thead>
              <tr>
                <th scope="col">Mês</th>
                <th scope="col" class="align-right">Entradas</th>
                <th scope="col" class="align-right">Saídas</th>
                <th scope="col" class="align-right">Líquido</th>
              </tr>
            </thead>
            <tbody>
              {#each cashFlow.months as month (month.month)}
                <tr>
                  <td class="mono">{formatMonth(month.month)}</td>
                  <td class="mono align-right amount-positive">
                    {formatCents(month.inflow_cents)}
                  </td>
                  <td class="mono align-right amount-negative">
                    {formatCents(month.outflow_cents)}
                  </td>
                  <td class="mono align-right">{formatCents(month.net_cents)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        <header class="panel-header panel-header--divided">
          <div class="panel-heading">
            <span class="panel-kicker">Origem</span>
            <h3 class="panel-title">Por categoria e tipo</h3>
          </div>
        </header>
        <div class="table-wrap">
          <table class="data-table">
            <thead>
              <tr>
                <th scope="col">Origem</th>
                <th scope="col">Tipo</th>
                <th scope="col" class="align-right">Entradas</th>
                <th scope="col" class="align-right">Saídas</th>
                <th scope="col" class="align-right">Líquido</th>
              </tr>
            </thead>
            <tbody>
              {#each cashFlow.groups as group (group.kind + group.label)}
                <tr>
                  <td>{group.label}</td>
                  <td><span class="tag">{kindLabel(group.kind)}</span></td>
                  <td class="mono align-right amount-positive">
                    {formatCents(group.inflow_cents)}
                  </td>
                  <td class="mono align-right amount-negative">
                    {formatCents(group.outflow_cents)}
                  </td>
                  <td class="mono align-right">{formatCents(group.net_cents)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>

    {#if asOf}
      <section class="panel" aria-labelledby="net-worth-title">
        <header class="panel-header">
          <div class="panel-heading">
            <span class="panel-kicker">Histórico</span>
            <h3 class="panel-title" id="net-worth-title">
              Patrimônio em {netWorth ? formatDate(netWorth.as_of) : formatDate(asOf)}
            </h3>
          </div>
          <span class="panel-meta">posição na data final do filtro</span>
        </header>

        {#if netWorthQuery.isPending}
          <div class="state-panel" aria-busy="true">
            <span class="state-kicker">Lendo</span>
            <h2>Calculando o patrimônio.</h2>
            <div class="loading-rule" aria-hidden="true"></div>
          </div>
        {:else if !netWorth}
          <div class="empty-panel"><p>Patrimônio indisponível.</p></div>
        {:else}
          <div class="data-grid data-grid--auto">
            <div class="data-cell">
              <div class="data-label">Patrimônio líquido</div>
              <div
                class="data-value"
                class:amount-negative={netWorth.net_worth_cents < 0}
              >
                {formatCents(netWorth.net_worth_cents)}
              </div>
            </div>
            <div class="data-cell">
              <div class="data-label">Ativos</div>
              <div class="data-value">{formatCents(netWorth.assets_cents)}</div>
              <div class="data-note">
                inclui {formatCents(netWorth.deferred_expense_cents)} a
                reconhecer
              </div>
            </div>
            <div class="data-cell">
              <div class="data-label">Passivos</div>
              <div class="data-value">
                {formatCents(netWorth.liabilities_cents)}
              </div>
            </div>
          </div>

          {#if netWorth.accounts.length > 0}
            <div class="table-wrap">
              <table class="data-table">
                <thead>
                  <tr>
                    <th scope="col">Conta</th>
                    <th scope="col">Tipo</th>
                    <th scope="col" class="align-right">Saldo na data</th>
                  </tr>
                </thead>
                <tbody>
                  {#each netWorth.accounts as account (account.account_id)}
                    <tr>
                      <td>{account.name}</td>
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
        {/if}
      </section>
    {/if}

    <section class="panel" aria-labelledby="commitments-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">Futuro</span>
          <h3 class="panel-title" id="commitments-title">
            Compromissos em detalhe
          </h3>
        </div>
        <span class="panel-meta">mês a mês · totais no resumo</span>
      </header>

      {#if commitmentsQuery.isPending}
        <div class="state-panel" aria-busy="true">
          <span class="state-kicker">Lendo</span>
          <h2>Lendo parcelas futuras.</h2>
          <div class="loading-rule" aria-hidden="true"></div>
        </div>
      {:else if !commitments}
        <div class="empty-panel"><p>Compromissos indisponíveis.</p></div>
      {:else if commitments.months.length === 0}
        <div class="empty-panel">
          <p>Nenhuma parcela em aberto no horizonte.</p>
        </div>
      {:else}
        {#each commitments.months as month (month.month)}
          <header class="panel-header panel-header--divided">
            <div class="panel-heading">
              <span class="panel-kicker">{formatMonth(month.month)}</span>
              <h3 class="panel-title">
                {formatCents(month.total_cents)}
              </h3>
            </div>
          </header>
          <div class="table-wrap">
            <table class="data-table">
              <thead>
                <tr>
                  <th scope="col">Compra</th>
                  <th scope="col">Cartão</th>
                  <th scope="col">Parcela</th>
                  <th scope="col" class="align-right">Em aberto</th>
                </tr>
              </thead>
              <tbody>
                {#each month.items as item (item.installment_id)}
                  <tr>
                    <td>
                      <div class="cell-stack">
                        <span>{item.description || item.category_name}</span>
                        <small>{item.category_name}</small>
                      </div>
                    </td>
                    <td>{item.card_account_name}</td>
                    <td class="mono">
                      {item.seq}/{item.installment_count} · {formatDate(
                        item.due_on,
                      )}
                    </td>
                    <td class="mono align-right">
                      {formatCents(item.outstanding_cents)}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/each}
      {/if}
    </section>
  {/if}
</section>
