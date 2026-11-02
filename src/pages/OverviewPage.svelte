<script lang="ts">
  import { createQuery } from "@tanstack/svelte-query";

  import { listAccounts, listMovements } from "../lib/api";
  import { normalizeError } from "../lib/errors";
  import {
    accountTypeLabel,
    formatCents,
    formatDate,
    kindLabel,
    periodLabel,
  } from "../lib/format";

  const accountsQuery = createQuery(() => ({
    queryKey: ["accounts", false],
    queryFn: () => listAccounts(false),
  }));

  const movementsQuery = createQuery(() => ({
    queryKey: ["movements", "overview"],
    queryFn: () => listMovements(),
  }));

  const accounts = $derived(accountsQuery.data ?? []);
  const movements = $derived(movementsQuery.data ?? []);
  const totalCents = $derived(
    accounts.reduce((total, account) => total + account.balance_cents, 0),
  );
  const recentMovements = $derived(movements.slice(0, 8));
  const queryError = $derived(
    accountsQuery.error
      ? normalizeError(accountsQuery.error)
      : movementsQuery.error
        ? normalizeError(movementsQuery.error)
        : null,
  );
  const loading = $derived(
    accountsQuery.isPending || movementsQuery.isPending,
  );
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Visão geral</span>
    <h2>O retrato do presente.</h2>
    <p>
      Saldos derivados dos lançamentos publicados. Nenhum número desta tela é
      calculado como autoridade no frontend.
    </p>
  </div>
  <div class="page-actions">
    <a class="button" href="#/contas">Contas</a>
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
      <div class="state-actions">
        <button
          type="button"
          class="button button--primary"
          onclick={() => {
            void accountsQuery.refetch();
            void movementsQuery.refetch();
          }}
        >
          Tentar novamente
        </button>
      </div>
    </div>
  </section>
{:else}
  <section class="summary-strip summary-strip--hero" aria-label="Resumo financeiro">
    <div>
      <span class="data-label">Patrimônio em contas</span>
      <strong class="hero-value">{formatCents(totalCents)}</strong>
    </div>
    <div>
      <span class="data-label">Contas ativas</span>
      <strong class="summary-value">{accounts.length}</strong>
    </div>
    <div>
      <span class="data-label">Movimentações</span>
      <strong class="summary-value">{movements.length}</strong>
    </div>
    <div class="summary-note">
      <span class="mono">soma dos saldos derivados</span>
    </div>
  </section>

  <div class="overview-grid">
    <section class="panel" aria-labelledby="overview-accounts-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">01</span>
          <h3 class="panel-title" id="overview-accounts-title">Contas</h3>
        </div>
        <a class="text-link" href="#/contas">ver todas</a>
      </header>

      {#if accounts.length === 0}
        <div class="empty-panel">
          <p>Nenhuma conta ativa. Comece pelo cadastro de uma conta.</p>
          <a class="button" href="#/contas">Criar conta</a>
        </div>
      {:else}
        <ul class="account-list">
          {#each accounts as account (account.id)}
            <li>
              <a
                href={account.account_type === "credit_card"
                  ? `#/cartoes/${account.id}`
                  : `#/contas/${account.id}`}
              >
                <span>
                  <strong>{account.name}</strong>
                  <small>{accountTypeLabel(account.account_type)}</small>
                </span>
                <span
                  class="mono"
                  class:amount-negative={account.balance_cents < 0}
                >
                  {formatCents(account.balance_cents)}
                </span>
              </a>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="panel" aria-labelledby="overview-movements-title">
      <header class="panel-header">
        <div class="panel-heading">
          <span class="panel-kicker">02</span>
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
