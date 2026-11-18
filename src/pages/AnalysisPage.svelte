<script lang="ts">
  import { createQuery } from "@tanstack/svelte-query";

  import type { ReportFilter } from "../lib/api/bindings";
  import {
    getCashFlowReport,
    getIncomeReport,
    getSpendingReport,
    listCategories,
    listInstallmentPlans,
    listMovements,
  } from "../lib/api";
  import { buildCategoryBars } from "../lib/category-chart";
  import CategoryChart from "../lib/components/CategoryChart.svelte";
  import CategoryIcon from "../lib/components/CategoryIcon.svelte";
  import { normalizeError } from "../lib/errors";
  import {
    formatCents,
    formatDate,
    formatMonth,
    kindLabel,
  } from "../lib/format";
  import { currentMonthIso, monthBounds, shiftMonth } from "../lib/month";

  let month = $state(currentMonthIso());
  let spendingMode = $state<"bars" | "donut">("bars");
  let incomeMode = $state<"bars" | "donut">("bars");

  /* O input pode ficar vazio enquanto o usuário edita; o mês efetivo cai no
     corrente para não quebrar as queries. */
  const effectiveMonth = $derived(
    /^\d{4}-(0[1-9]|1[0-2])$/.test(month) ? month : currentMonthIso(),
  );
  const bounds = $derived(monthBounds(effectiveMonth));

  const filter = $derived<ReportFilter>({
    from: bounds.from,
    to: bounds.to,
    period: null,
    category_id: null,
    account_id: null,
    payment_method: null,
  });

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

  const categoriesQuery = createQuery(() => ({
    queryKey: ["categories", true],
    queryFn: () => listCategories(true),
  }));

  /* As parcelas de cartão são despesas reconhecidas no vencimento e não
     aparecem em list_movements; por isso entram via planos. */
  const expensesQuery = createQuery(() => ({
    queryKey: ["movements", "largest-expenses", bounds.from, bounds.to],
    queryFn: () =>
      listMovements({
        account_id: null,
        category_id: null,
        kind: "expense",
        period: null,
        payment_method: null,
        from: bounds.from,
        to: bounds.to,
      }),
  }));

  const plansQuery = createQuery(() => ({
    queryKey: ["installment-plans", null],
    queryFn: () => listInstallmentPlans(null),
  }));

  const categories = $derived(categoriesQuery.data ?? []);
  const icons = $derived(
    Object.fromEntries(categories.map((category) => [category.id, category.icon])),
  );
  const spending = $derived(spendingQuery.data);
  const income = $derived(incomeQuery.data);
  const cashFlow = $derived(cashFlowQuery.data);

  const spendingBars = $derived(buildCategoryBars(spending?.categories ?? []));
  const incomeBars = $derived(buildCategoryBars(income?.categories ?? []));

  type LargestExpense = {
    key: string;
    label: string;
    categoryName: string | null;
    sourceName: string;
    date: string;
    amountCents: number;
    icon: string;
  };

  const largestExpenses = $derived.by(() => {
    const items: LargestExpense[] = [];

    for (const movement of expensesQuery.data ?? []) {
      items.push({
        key: movement.id,
        label:
          movement.description ||
          movement.category_name ||
          kindLabel(movement.kind),
        categoryName: movement.category_name,
        sourceName: movement.primary_account_name,
        date: movement.occurred_on,
        amountCents: Math.abs(movement.amount_cents),
        icon: icons[movement.category_id ?? ""] ?? "generic",
      });
    }

    for (const plan of plansQuery.data ?? []) {
      for (const installment of plan.installments) {
        if (
          installment.due_on < bounds.from ||
          installment.due_on > bounds.to
        ) {
          continue;
        }
        items.push({
          key: installment.id,
          label: plan.description || plan.category_name,
          categoryName: plan.category_name,
          sourceName: plan.card_account_name,
          date: installment.due_on,
          amountCents: installment.amount_cents,
          icon: icons[plan.category_id] ?? "generic",
        });
      }
    }

    return items
      .sort((left, right) => right.amountCents - left.amountCents)
      .slice(0, 5);
  });

  const analysisError = $derived(
    spendingQuery.error
      ? normalizeError(spendingQuery.error)
      : incomeQuery.error
        ? normalizeError(incomeQuery.error)
        : cashFlowQuery.error
          ? normalizeError(cashFlowQuery.error)
          : expensesQuery.error
            ? normalizeError(expensesQuery.error)
            : plansQuery.error
              ? normalizeError(plansQuery.error)
              : null,
  );

  function stepMonth(delta: number): void {
    month = shiftMonth(effectiveMonth, delta);
  }

  function refresh(): void {
    void spendingQuery.refetch();
    void incomeQuery.refetch();
    void cashFlowQuery.refetch();
    void expensesQuery.refetch();
    void plansQuery.refetch();
  }
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Quadro de análises</span>
    <h2>O mês em perspectiva.</h2>
    <p>
      Gráficos e comparações do mês selecionado. Patrimônio, saldos e limites
      continuam atuais na Visão geral. Todo número é calculado no Rust a partir
      dos lançamentos publicados.
    </p>
  </div>
  <div class="page-actions month-toolbar">
    <button
      type="button"
      class="button button--small"
      aria-label="Mês anterior"
      onclick={() => stepMonth(-1)}
    >
      Anterior
    </button>
    <label class="field month-field">
      <span>Mês</span>
      <input type="month" bind:value={month} />
    </label>
    <button
      type="button"
      class="button button--small"
      aria-label="Mês seguinte"
      onclick={() => stepMonth(1)}
    >
      Próximo
    </button>
  </div>
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
  <div class="analysis-grid">
    <section class="panel" aria-labelledby="spending-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">Saídas</span>
          <h3 class="panel-title" id="spending-title">Gastos por categoria</h3>
        </div>
        <div
          class="chart-toggle"
          role="group"
          aria-label="Visualização de gastos"
        >
          <button
            type="button"
            class:is-active={spendingMode === "bars"}
            aria-pressed={spendingMode === "bars"}
            onclick={() => (spendingMode = "bars")}
          >
            Barras
          </button>
          <button
            type="button"
            class:is-active={spendingMode === "donut"}
            aria-pressed={spendingMode === "donut"}
            onclick={() => (spendingMode = "donut")}
          >
            Rosquinha
          </button>
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
          <p>Nenhuma despesa no mês selecionado.</p>
        </div>
      {:else}
        <CategoryChart
          bars={spendingBars}
          totalCents={spending.total_cents}
          entryCount={spending.entry_count}
          tone="expense"
          mode={spendingMode}
          {icons}
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
        <div
          class="chart-toggle"
          role="group"
          aria-label="Visualização de receitas"
        >
          <button
            type="button"
            class:is-active={incomeMode === "bars"}
            aria-pressed={incomeMode === "bars"}
            onclick={() => (incomeMode = "bars")}
          >
            Barras
          </button>
          <button
            type="button"
            class:is-active={incomeMode === "donut"}
            aria-pressed={incomeMode === "donut"}
            onclick={() => (incomeMode = "donut")}
          >
            Rosquinha
          </button>
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
          <p>Nenhuma receita no mês selecionado.</p>
        </div>
      {:else}
        <CategoryChart
          bars={incomeBars}
          totalCents={income.total_cents}
          entryCount={income.entry_count}
          tone="income"
          mode={incomeMode}
          {icons}
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
      <span class="panel-meta">{formatMonth(effectiveMonth)}</span>
    </header>

    {#if cashFlowQuery.isPending}
      <div class="state-panel" aria-busy="true">
        <span class="state-kicker">Lendo</span>
        <h2>Calculando o caixa.</h2>
        <div class="loading-rule" aria-hidden="true"></div>
      </div>
    {:else if !cashFlow}
      <div class="empty-panel">
        <p>Fluxo de caixa indisponível.</p>
      </div>
    {:else}
      <div class="data-grid data-grid--auto">
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
          <div class="data-note">transferências não entram no caixa</div>
        </div>
      </div>

      {#if cashFlow.groups.length > 0}
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
    {/if}
  </section>

  <section class="panel" aria-labelledby="largest-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">Destaques</span>
        <h3 class="panel-title" id="largest-title">Maiores despesas do mês</h3>
      </div>
      <span class="panel-meta">top 5 · {formatDate(bounds.from)} – {formatDate(bounds.to)}</span>
    </header>

    {#if expensesQuery.isPending || plansQuery.isPending}
      <div class="state-panel" aria-busy="true">
        <span class="state-kicker">Lendo</span>
        <h2>Comparando despesas.</h2>
        <div class="loading-rule" aria-hidden="true"></div>
      </div>
    {:else if largestExpenses.length === 0}
      <div class="empty-panel">
        <p>Nenhuma despesa no mês selecionado.</p>
      </div>
    {:else}
      <ul class="movement-list">
        {#each largestExpenses as expense (expense.key)}
          <li>
            <div class="movement-list-main">
              <span class="expense-label">
                <CategoryIcon icon={expense.icon} size={16} />
                <span>{expense.label}</span>
              </span>
              <small>
                {formatDate(expense.date)} ·
                {expense.categoryName ?? "Sem categoria"} ·
                {expense.sourceName}
              </small>
            </div>
            <span class="mono">{formatCents(expense.amountCents)}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}
