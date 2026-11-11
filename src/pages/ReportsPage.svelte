<script lang="ts">
  import { createQuery } from "@tanstack/svelte-query";

  import type { ReportFilter } from "../lib/api/bindings";
  import {
    getCashFlowReport,
    getCommitmentsReport,
    getIncomeReport,
    getNetWorthReport,
    getSpendingReport,
    listAccounts,
    listCategories,
  } from "../lib/api";
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

  const accountsQuery = createQuery(() => ({
    queryKey: ["accounts", true],
    queryFn: () => listAccounts(true),
  }));

  const categoriesQuery = createQuery(() => ({
    queryKey: ["categories", true],
    queryFn: () => listCategories(true),
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

  const netWorthQuery = createQuery(() => ({
    queryKey: ["reports", "net-worth", asOf],
    queryFn: () => getNetWorthReport(asOf),
  }));

  const commitmentsQuery = createQuery(() => ({
    queryKey: ["reports", "commitments", asOf],
    queryFn: () => getCommitmentsReport(asOf, 12),
  }));

  const accounts = $derived(accountsQuery.data ?? []);
  const categories = $derived(categoriesQuery.data ?? []);
  const spending = $derived(spendingQuery.data);
  const income = $derived(incomeQuery.data);
  const cashFlow = $derived(cashFlowQuery.data);
  const netWorth = $derived(netWorthQuery.data);
  const commitments = $derived(commitmentsQuery.data);

  const resultCents = $derived(
    (income?.total_cents ?? 0) - (spending?.total_cents ?? 0),
  );

  const queryError = $derived(
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
    void spendingQuery.refetch();
    void incomeQuery.refetch();
    void cashFlowQuery.refetch();
    void netWorthQuery.refetch();
    void commitmentsQuery.refetch();
  }
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Relatórios</span>
    <h2>Competência e caixa.</h2>
    <p>
      Despesas e receitas usam o período do lançamento; parcelas contam no
      vencimento. Fluxo de caixa mostra o dinheiro que entrou e saiu das contas,
      sem transferências nem saldos iniciais.
    </p>
  </div>
  <div class="page-actions">
    <button type="button" class="button" onclick={refresh}>Atualizar</button>
  </div>
</section>

<section class="panel" aria-labelledby="report-filters-title">
  <header class="panel-header">
    <div class="panel-heading">
      <span class="panel-kicker">Filtros</span>
      <h3 class="panel-title" id="report-filters-title">Período</h3>
    </div>
    <span class="panel-meta">
      {spending ? formatDate(spending.from) : "—"} · {spending
        ? formatDate(spending.to)
        : "—"}
    </span>
  </header>
  <div class="filters">
    <label class="field">
      <span>De</span>
      <input type="date" bind:value={from} />
    </label>
    <label class="field">
      <span>Até</span>
      <input type="date" bind:value={to} />
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
        {#each accounts as account (account.id)}
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
      <span class="mono">vazio</span>
      <span>Sem datas, o relatório usa o mês corrente.</span>
    </div>
    <div class="panel-footnote">
      <span class="mono">escopo</span>
      <span>
        Conta e forma de pagamento afetam gastos, receitas e fluxo de caixa;
        patrimônio e compromissos usam apenas a data.
      </span>
    </div>
    <div class="actions">
      <button type="button" class="button" onclick={clearFilters}>
        Limpar
      </button>
    </div>
  </footer>
</section>

{#if queryError}
  <section class="panel">
    <div class="state-panel state-panel--error" role="alert">
      <span class="state-kicker">{queryError.code}</span>
      <h2>Não foi possível carregar os relatórios.</h2>
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
{:else}
  <section class="summary-strip summary-strip--hero" aria-label="Resumo do período">
    <div>
      <span class="data-label">Resultado</span>
      <strong class="hero-value" class:amount-negative={resultCents < 0}>
        {formatCents(resultCents)}
      </strong>
    </div>
    <div>
      <span class="data-label">Despesas</span>
      <strong class="summary-value">
        {formatCents(spending?.total_cents ?? 0)}
      </strong>
    </div>
    <div>
      <span class="data-label">Receitas</span>
      <strong class="summary-value">
        {formatCents(income?.total_cents ?? 0)}
      </strong>
    </div>
    <div class="summary-note">
      <span class="mono">receitas − despesas</span>
    </div>
  </section>

  <div class="overview-grid">
    <section class="panel" aria-labelledby="spending-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">Saídas</span>
          <h3 class="panel-title" id="spending-title">Gastos por categoria</h3>
        </div>
        <span class="panel-meta">
          {spending?.entry_count ?? 0} lançamentos
        </span>
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
          </table>
        </div>
      {/if}
    </section>

    <section class="panel" aria-labelledby="income-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">Entradas</span>
          <h3 class="panel-title" id="income-title">Receitas por categoria</h3>
        </div>
        <span class="panel-meta">
          {income?.entry_count ?? 0} lançamentos
        </span>
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
          </table>
        </div>
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

      <header class="panel-header">
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

  <div class="overview-grid">
    <section class="panel" aria-labelledby="net-worth-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">Patrimônio</span>
          <h3 class="panel-title" id="net-worth-title">Patrimônio líquido</h3>
        </div>
        <span class="panel-meta">
          {netWorth ? formatDate(netWorth.as_of) : "—"}
        </span>
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
        <div class="data-grid">
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
              inclui {formatCents(netWorth.deferred_expense_cents)} a reconhecer
            </div>
          </div>
          <div class="data-cell">
            <div class="data-label">Passivos</div>
            <div class="data-value">
              {formatCents(netWorth.liabilities_cents)}
            </div>
          </div>
          <div class="data-cell">
            <div class="data-label">Contas</div>
            <div class="data-value">{netWorth.accounts.length}</div>
            <div class="data-note">saldos não zerados</div>
          </div>
        </div>

        {#if netWorth.accounts.length > 0}
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

    <section class="panel" aria-labelledby="commitments-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">Futuro</span>
          <h3 class="panel-title" id="commitments-title">Compromissos</h3>
        </div>
        <span class="panel-meta">próximos 12 meses</span>
      </header>

      {#if commitmentsQuery.isPending}
        <div class="state-panel" aria-busy="true">
          <span class="state-kicker">Lendo</span>
          <h2>Lendo parcelas futuras.</h2>
          <div class="loading-rule" aria-hidden="true"></div>
        </div>
      {:else if !commitments}
        <div class="empty-panel"><p>Compromissos indisponíveis.</p></div>
      {:else}
        <div class="data-grid">
          <div class="data-cell">
            <div class="data-label">Em aberto</div>
            <div class="data-value">
              {formatCents(commitments.total_cents)}
            </div>
            <div class="data-note">
              vencimentos após {formatDate(commitments.start_after)}
            </div>
          </div>
          <div class="data-cell">
            <div class="data-label">Vencidas</div>
            <div
              class="data-value"
              class:amount-negative={commitments.overdue_cents > 0}
            >
              {formatCents(commitments.overdue_cents)}
            </div>
            <div class="data-note">
              até {formatDate(commitments.as_of)}
            </div>
          </div>
        </div>

        {#if commitments.months.length === 0}
          <div class="empty-panel">
            <p>Nenhuma parcela em aberto no horizonte.</p>
          </div>
        {:else}
          {#each commitments.months as month (month.month)}
            <header class="panel-header">
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
      {/if}
    </section>
  </div>
{/if}
