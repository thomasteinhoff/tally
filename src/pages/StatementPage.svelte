<script lang="ts">
  import {
    createQuery,
    useQueryClient,
  } from "@tanstack/svelte-query";

  import { getAccountStatement, listCategories } from "../lib/api";
  import { normalizeError } from "../lib/errors";
  import {
    formatCents,
    formatDate,
    kindLabel,
    periodLabel,
  } from "../lib/format";
  import {
    filterStatementLines,
    hasStatementLineFilters,
  } from "../lib/statement-filters";

  let { accountId }: { accountId: string } = $props();

  const queryClient = useQueryClient();

  let from = $state("");
  let to = $state("");
  let categoryId = $state("");
  let kind = $state("");
  let period = $state("");

  const statementQuery = createQuery(() => ({
    queryKey: ["statement", accountId, from, to],
    queryFn: () =>
      getAccountStatement(accountId, from || null, to || null),
  }));

  const categoriesQuery = createQuery(() => ({
    queryKey: ["categories", true],
    queryFn: () => listCategories(true),
  }));

  const statement = $derived(statementQuery.data);
  const categories = $derived(categoriesQuery.data ?? []);
  const queryError = $derived(
    statementQuery.error ? normalizeError(statementQuery.error) : null,
  );

  const lineFilters = $derived({ categoryId, kind, period });
  const filteredLines = $derived(
    statement ? filterStatementLines(statement.lines, lineFilters) : [],
  );
  // O extrato é publicado em ordem cronológica para calcular o saldo corrente;
  // na tela mostramos do lançamento mais recente para o mais antigo.
  const displayedLines = $derived([...filteredLines].reverse());
  const lineFiltersActive = $derived(
    hasStatementLineFilters(lineFilters),
  );
  const hasPeriodSelection = $derived(Boolean(from || to));
  const periodRangeLabel = $derived(
    from && to
      ? `${formatDate(from)} a ${formatDate(to)}`
      : from
        ? `a partir de ${formatDate(from)}`
        : to
          ? `até ${formatDate(to)}`
          : "histórico completo",
  );

  function clearFilters(): void {
    from = "";
    to = "";
    categoryId = "";
    kind = "";
    period = "";
  }
</script>

<section class="page-intro">
  <div>
    <a class="text-link text-link--back" href="#/contas">← Contas</a>
    <span class="eyebrow">Extrato</span>
    <h2>{statement?.account.name ?? "Conta"}</h2>
    <p>
      Lançamentos publicados com o saldo corrente calculado pelo Rust. A lista
      aparece da data mais recente para a mais antiga; o saldo inicial segue
      como linha própria.
    </p>
  </div>
  <button
    type="button"
    class="button"
    onclick={() => {
      void queryClient.invalidateQueries();
    }}
  >
    Atualizar
  </button>
</section>

<section class="panel" aria-labelledby="statement-filter-title">
  <header class="panel-header">
    <div class="panel-heading">
      <span class="panel-kicker">Filtros</span>
      <h3 class="panel-title" id="statement-filter-title">
        Filtrar extrato
        <span
          class="info-mark"
          role="img"
          aria-label="Categoria, tipo e período ocultam linhas. As datas também recalculam os saldos do período selecionado."
          title="Categoria, tipo e período ocultam linhas. As datas também recalculam os saldos do período selecionado."
          >i</span
        >
      </h3>
    </div>
    <button
      type="button"
      class="button button--quiet"
      onclick={clearFilters}
    >
      Limpar filtros
    </button>
  </header>

  <div class="filters">
    <label class="field">
      <span>Categoria</span>
      <select bind:value={categoryId}>
        <option value="">Todas</option>
        {#each categories as category (category.id)}
          <option value={category.id}>
            {category.name} · {kindLabel(category.kind)}{category.archived_at
              ? " (arquivada)"
              : ""}
          </option>
        {/each}
      </select>
    </label>

    <label class="field">
      <span>Tipo</span>
      <select bind:value={kind}>
        <option value="">Todos</option>
        <option value="expense">Despesa</option>
        <option value="income">Receita</option>
        <option value="transfer">Transferência</option>
        <option value="adjustment">Ajuste</option>
        <option value="opening_balance">Saldo inicial</option>
      </select>
    </label>

    <label class="field">
      <span>
        Período
        <span
          class="info-mark"
          role="img"
          aria-label="O período do dia vem do horário quando informado ou da escolha manual."
          title="O período do dia vem do horário quando informado ou da escolha manual."
          >i</span
        >
      </span>
      <select bind:value={period}>
        <option value="">Todos</option>
        <option value="madrugada">Madrugada</option>
        <option value="manha">Manhã</option>
        <option value="tarde">Tarde</option>
        <option value="noite">Noite</option>
        <option value="custom">Personalizado</option>
      </select>
    </label>

    <label class="field">
      <span>De</span>
      <input type="date" bind:value={from} />
    </label>

    <label class="field">
      <span>Até</span>
      <input type="date" bind:value={to} />
    </label>
  </div>
</section>

{#if statementQuery.isPending}
  <section class="panel" aria-busy="true">
    <div class="state-panel">
      <span class="state-kicker">Lendo</span>
      <h2>Montando o extrato.</h2>
      <p>O saldo corrente é calculado a partir dos postings publicados.</p>
      <div class="loading-rule" aria-hidden="true"></div>
    </div>
  </section>
{:else if queryError}
  <section class="panel">
    <div class="state-panel state-panel--error" role="alert">
      <span class="state-kicker">{queryError.code}</span>
      <h2>Não foi possível montar o extrato.</h2>
      <p>{queryError.message}</p>
      <div class="state-actions">
        <button
          type="button"
          class="button button--primary"
          onclick={() => statementQuery.refetch()}
        >
          Tentar novamente
        </button>
      </div>
    </div>
  </section>
{:else if statement}
  {#if hasPeriodSelection}
    <section
      class="summary-strip"
      aria-label="Resumo do período selecionado"
    >
      <div>
        <span class="data-label">
          Saldo no início do período
          <span
            class="info-mark"
            role="img"
            aria-label="Saldo da conta antes da data inicial do período selecionado."
            title="Saldo da conta antes da data inicial do período selecionado."
            >i</span
          >
        </span>
        <strong class="summary-value">
          {formatCents(statement.opening_balance_cents)}
        </strong>
      </div>
      <div>
        <span class="data-label">
          Saldo no fim do período
          <span
            class="info-mark"
            role="img"
            aria-label="Saldo da conta até a data final do período selecionado."
            title="Saldo da conta até a data final do período selecionado."
            >i</span
          >
        </span>
        <strong class="summary-value">
          {formatCents(statement.closing_balance_cents)}
        </strong>
      </div>
      <div>
        <span class="data-label">Saldo atual da conta</span>
        <strong class="summary-value">
          {formatCents(statement.account.balance_cents)}
        </strong>
      </div>
      <div class="summary-note">
        <span class="mono">{periodRangeLabel}</span>
      </div>
    </section>
  {:else}
    <section
      class="summary-strip summary-strip--current"
      aria-label="Saldo da conta"
    >
      <div>
        <span class="data-label">
          Saldo atual
          <span
            class="info-mark"
            role="img"
            aria-label="Soma de todos os lançamentos publicados da conta."
            title="Soma de todos os lançamentos publicados da conta."
            >i</span
          >
        </span>
        <strong class="hero-value">
          {formatCents(statement.account.balance_cents)}
        </strong>
      </div>
      <div>
        <span class="data-label">Lançamentos no histórico</span>
        <strong class="summary-value">{statement.lines.length}</strong>
      </div>
      <div class="summary-note">
        <span class="mono">histórico completo</span>
      </div>
    </section>
  {/if}

  <section class="panel" aria-labelledby="statement-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">Livro</span>
        <h3 class="panel-title" id="statement-title">Lançamentos</h3>
      </div>
      <span class="panel-meta">
        {#if lineFiltersActive}
          {filteredLines.length} de {statement.lines.length}
          {statement.lines.length === 1 ? "linha" : "linhas"}
        {:else}
          {statement.lines.length}
          {statement.lines.length === 1 ? "linha" : "linhas"}
        {/if}
      </span>
    </header>

    {#if statement.lines.length === 0}
      <div class="empty-panel">
        <p>Nenhum lançamento nesta conta.</p>
      </div>
    {:else if filteredLines.length === 0}
      <div class="empty-panel">
        <p>Nenhum lançamento com estes filtros.</p>
        <button type="button" class="button" onclick={clearFilters}>
          Limpar filtros
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
              <th scope="col">Categoria</th>
              <th scope="col">Contraparte</th>
              <th scope="col">Período</th>
              <th scope="col" class="align-right">Valor</th>
              <th scope="col" class="align-right">Saldo</th>
            </tr>
          </thead>
          <tbody>
            {#each displayedLines as line (line.entry_id)}
              <tr>
                <td>{formatDate(line.occurred_on)}</td>
                <td>
                  {line.description || kindLabel(line.kind)}
                </td>
                <td>
                  <span class="tag tag--{line.kind}"
                    >{kindLabel(line.kind)}</span
                  >
                </td>
                <td>{line.category_name ?? "—"}</td>
                <td>{line.counterparty_account_name ?? "—"}</td>
                <td>{periodLabel(line.period)}</td>
                <td
                  class="mono align-right"
                  class:amount-negative={line.amount_cents < 0 &&
                    line.kind !== "transfer"}
                  class:amount-positive={line.amount_cents > 0 &&
                    line.kind !== "transfer"}
                  class:amount-neutral={line.kind === "transfer"}
                >
                  {formatCents(line.amount_cents)}
                </td>
                <td
                  class="mono align-right"
                  class:amount-negative={line.running_balance_cents < 0}
                >
                  {formatCents(line.running_balance_cents)}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
{/if}
