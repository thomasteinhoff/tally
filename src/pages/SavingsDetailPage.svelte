<script lang="ts">
  import { createQuery, useQueryClient } from "@tanstack/svelte-query";

  import type {
    CdiStatusDto,
    MovementFilter,
    SavingsYieldDto,
  } from "../lib/api/bindings";
  import {
    createSavingsYield,
    createTransfer,
    getCdiStatus,
    getSavingsOverview,
    listAccounts,
    listMovements,
    refreshCdiRates,
    replaceSavingsYield,
    reverseSavingsYield,
  } from "../lib/api";
  import { confirmAction } from "../lib/confirm";
  import { normalizeError } from "../lib/errors";
  import {
    currentTime,
    formatCents,
    formatDate,
    kindLabel,
    parseAmountToCents,
    todayIso,
  } from "../lib/format";
  import { notify } from "../lib/toast.svelte";

  let { accountId }: { accountId: string } = $props();

  const queryClient = useQueryClient();

  let yieldDate = $state(todayIso());
  let transferMode = $state<"apply" | "redeem">("apply");
  let counterpartyId = $state("");
  let transferAmount = $state("");
  let transferDate = $state(todayIso());
  let transferDescription = $state("");
  let localError = $state<string | null>(null);
  let busy = $state(false);
  let cdiFrom = $state("");
  let cdiStatus = $state<CdiStatusDto | null>(null);
  let cdiLoading = $state(false);
  let cdiError = $state<string | null>(null);

  // A leitura do cache é local; nenhuma requisição acontece na abertura.
  const cdiQuery = createQuery(() => ({
    queryKey: ["cdi", "status"],
    queryFn: getCdiStatus,
  }));

  const overviewQuery = createQuery(() => ({
    queryKey: ["savings", accountId],
    queryFn: () => getSavingsOverview(accountId),
  }));

  const accountsQuery = createQuery(() => ({
    queryKey: ["accounts", true],
    queryFn: () => listAccounts(true),
  }));

  const accountFilter = $derived<MovementFilter>({
    account_id: accountId,
    category_id: null,
    kind: null,
    period: null,
    payment_method: null,
    from: null,
    to: null,
  });
  const historyQuery = createQuery(() => ({
    queryKey: ["movements", accountFilter],
    queryFn: () => listMovements(accountFilter),
  }));

  const overview = $derived(overviewQuery.data);
  const account = $derived(overview?.account);
  const yields = $derived(overview?.yields ?? []);
  const history = $derived(historyQuery.data ?? []);
  const cdi = $derived(cdiStatus ?? cdiQuery.data ?? null);
  const cdiStale = $derived(
    cdi !== null &&
      (cdi.rate_count === 0 || cdi.last_error !== null || cdi.last_success_at === null),
  );
  const archived = $derived(Boolean(account?.archived_at));
  const regularAccounts = $derived(
    (accountsQuery.data ?? []).filter(
      (item) =>
        !item.is_savings &&
        item.account_type !== "credit_card" &&
        !item.archived_at,
    ),
  );

  async function perform(
    success: string,
    action: () => Promise<unknown>,
  ): Promise<void> {
    busy = true;
    localError = null;
    try {
      await action();
      notify(success);
      await queryClient.invalidateQueries();
    } catch (error) {
      localError = normalizeError(error).message;
      notify(localError, "warn");
    } finally {
      busy = false;
    }
  }

  async function refreshCdi(): Promise<void> {
    cdiLoading = true;
    cdiError = null;
    try {
      cdiStatus = await refreshCdiRates(cdiFrom.trim() || undefined);
      notify("Taxas CDI atualizadas.");
      await queryClient.invalidateQueries({ queryKey: ["cdi", "status"] });
    } catch (error) {
      cdiError = normalizeError(error).message;
      notify(cdiError, "warn");
    } finally {
      cdiLoading = false;
    }
  }

  function submitYield(): void {
    if (!yieldDate) {
      localError = "Informe a data do rendimento.";
      return;
    }
    void perform("Rendimento registrado.", () =>
      createSavingsYield({ account_id: accountId, as_of: yieldDate }),
    );
  }

  async function reverseYield(item: SavingsYieldDto): Promise<void> {
    const confirmed = await confirmAction(
      `Estornar o rendimento de ${formatDate(item.as_of)}? O original fica preservado e um lançamento compensatório entra na data de hoje.`,
      "Estornar rendimento",
      "Estornar",
    );
    if (confirmed) {
      void perform("Rendimento estornado.", () =>
        reverseSavingsYield({ yield_id: item.id, as_of: todayIso() }),
      );
    }
  }

  async function substituteYield(item: SavingsYieldDto): Promise<void> {
    const confirmed = await confirmAction(
      `Substituir o rendimento de ${formatDate(item.as_of)}? O original é estornado hoje e um novo rendimento é calculado na data original.`,
      "Substituir rendimento",
      "Substituir",
    );
    if (confirmed) {
      void perform("Rendimento substituído.", () =>
        replaceSavingsYield({ yield_id: item.id }),
      );
    }
  }

  function submitTransfer(): void {
    const amount = parseAmountToCents(transferAmount);
    if (amount === null || amount <= 0) {
      localError = "Informe um valor válido para a transferência.";
      return;
    }
    if (!counterpartyId) {
      localError = "Selecione a conta convencional.";
      return;
    }
    if (!transferDate) {
      localError = "Informe a data da transferência.";
      return;
    }

    const applying = transferMode === "apply";
    // ponytail: usa o horário atual, igual ao formulário de movimentações; expor campo se precisarem horário histórico.
    void perform(applying ? "Aplicação registrada." : "Resgate registrado.", () =>
      createTransfer({
        from_account_id: applying ? counterpartyId : accountId,
        to_account_id: applying ? accountId : counterpartyId,
        amount_cents: amount,
        occurred_on: transferDate,
        occurred_time: currentTime(),
        manual_period: null,
        description:
          transferDescription.trim() ||
          (applying ? "Aplicação no cofrinho" : "Resgate do cofrinho"),
      }),
    );
  }
</script>

<section class="page-intro">
  <div>
    <a class="text-link text-link--back" href="#/contas">← Contas</a>
    <span class="eyebrow">Cofrinho</span>
    <h2>{account?.name ?? "Cofrinho"}</h2>
    <p>
      Rendimento diário sobre o saldo no início do dia. Aplicações e resgates
      são transferências com uma conta convencional; rendimentos e estornos
      aparecem em Movimentações com tipo próprio e continuam fora de receitas,
      despesas e fluxo de caixa.
    </p>
  </div>
  <button
    type="button"
    class="button"
    onclick={() => void queryClient.invalidateQueries()}
  >
    Atualizar
  </button>
</section>

{#if localError}
  <section class="panel" role="alert">
    <div class="inline-result inline-result--warn">
      <span>{localError}</span>
    </div>
  </section>
{/if}

{#if overviewQuery.isPending}
  <section class="panel" aria-busy="true">
    <div class="state-panel">
      <span class="state-kicker">Lendo</span>
      <h2>Abrindo o cofrinho.</h2>
      <div class="loading-rule" aria-hidden="true"></div>
    </div>
  </section>
{:else if overviewQuery.error || !account}
  <section class="panel">
    <div class="state-panel state-panel--error" role="alert">
      <span class="state-kicker">
        {overviewQuery.error ? normalizeError(overviewQuery.error).code : "404"}
      </span>
      <h2>Não foi possível abrir este cofrinho.</h2>
      <p>
        {overviewQuery.error
          ? normalizeError(overviewQuery.error).message
          : "Conta não encontrada."}
      </p>
      <div class="state-actions">
        <button
          type="button"
          class="button button--primary"
          onclick={() => overviewQuery.refetch()}
        >
          Tentar novamente
        </button>
      </div>
    </div>
  </section>
{:else}
  <section class="panel" aria-labelledby="cdi-status-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">SGS 12</span>
        <h3 class="panel-title" id="cdi-status-title">Taxas CDI</h3>
      </div>
      <span class="panel-meta">
        {cdi ? `${cdi.rate_count} taxas` : "carregando"}
      </span>
    </header>
    <div class="panel-body">
      <p>
        {#if cdi && cdi.last_rate_date}
          Última publicação em {formatDate(cdi.last_rate_date)}.
        {:else}
          Nenhuma taxa em cache. O rendimento usa apenas taxas locais.
        {/if}
      </p>
      {#if cdiStale}
        <div class="inline-result inline-result--warn" role="status">
          <span>
            {cdi?.last_error
              ? `Última atualização falhou: ${cdi.last_error}. `
              : "Cache desatualizado. "}
            As taxas em cache continuam disponíveis; use “Atualizar taxas” quando
            houver conexão.
          </span>
        </div>
      {/if}
      {#if cdiError}
        <div class="inline-result inline-result--warn" role="alert">
          <span>{cdiError}</span>
        </div>
      {/if}
      <div class="filters">
        <label class="field">
          <span>Data inicial (opcional)</span>
          <input type="date" bind:value={cdiFrom} max={todayIso()} />
        </label>
        <button
          type="button"
          class="button button--primary"
          disabled={cdiLoading}
          onclick={() => void refreshCdi()}
        >
          {cdiLoading ? "Atualizando…" : "Atualizar taxas"}
        </button>
      </div>
      <p class="panel-footnote">
        Somente o identificador da série (12) e o intervalo de datas são
        enviados ao Banco Central. Nenhum dado financeiro ou pessoal sai do
        aplicativo.
      </p>
    </div>
  </section>

  <section class="summary-strip" aria-label="Resumo do cofrinho">
    <div>
      <span class="data-label">Saldo do cofrinho</span>
      <strong class="hero-value">{formatCents(account.balance_cents)}</strong>
    </div>
    <div>
      <span class="data-label">Modalidade</span>
      <strong class="summary-value">
        {account.yield_mode === "business" ? "Dias úteis" : "Todos os dias"}
      </strong>
    </div>
    <div>
      <span class="data-label">Percentual contratado</span>
      <strong class="summary-value">
        {account.cdi_percent_bps !== null
          ? `${account.cdi_percent_bps / 100}% do CDI`
          : "—"}
      </strong>
    </div>
    <div>
      <span class="data-label">Situação</span>
      <strong class="summary-value">
        <span class="tag {archived ? 'tag--archived' : 'tag--active'}">
          {archived ? "Arquivado" : "Ativo"}
        </span>
      </strong>
    </div>
  </section>

  {#if archived}
    <section class="panel">
      <div class="inline-result inline-result--warn">
        <span>
          Cofrinho arquivado: novas aplicações, resgates, rendimentos e
          estornos não são permitidos. O saldo continua nos indicadores.
        </span>
      </div>
    </section>
  {/if}

  <section class="panel" aria-labelledby="savings-yield-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">Rendimento</span>
        <h3 class="panel-title" id="savings-yield-title">Histórico de rendimentos</h3>
      </div>
      <span class="panel-meta">{yields.length} registros</span>
    </header>

    {#if !archived}
      <div class="filters">
        <label class="field">
          <span>Data do rendimento</span>
          <input type="date" bind:value={yieldDate} max={todayIso()} />
        </label>
        <div class="field field--inline">
          <button
            type="button"
            class="button button--primary"
            disabled={busy}
            onclick={submitYield}
          >
            Gerar rendimento
          </button>
        </div>
      </div>
      <div class="panel-footer">
        <div class="panel-footnote">
          <span class="mono">base = saldo no início do dia</span>
          <span>
            Saldo inicial, aplicações e resgates na data só entram na base no
            dia seguinte. “Somente dias úteis” exige a taxa publicada na data e
            não rende em fins de semana ou feriados. “Todos os dias” usa a
            última taxa disponível e uma aproximação de 365 dias definida pelo
            Tally — não é uma convenção contratual.
          </span>
        </div>
      </div>
    {/if}

    {#if yields.length === 0}
      <div class="empty-panel">
        <p>Nenhum rendimento registrado neste cofrinho.</p>
      </div>
    {:else}
      <div class="table-wrap">
        <table class="data-table">
          <thead>
            <tr>
              <th scope="col">Data</th>
              <th scope="col">Modalidade</th>
              <th scope="col" class="align-right">Base</th>
              <th scope="col" class="align-right">Rendimento</th>
              <th scope="col">Parâmetros</th>
              <th scope="col">Situação</th>
              <th scope="col" class="align-right">Ações</th>
            </tr>
          </thead>
          <tbody>
            {#each [...yields].reverse() as item (item.id)}
              <tr class:row-muted={item.status !== "confirmed"}>
                <td>{formatDate(item.as_of)}</td>
                <td>
                  {item.mode === "business" ? "Dias úteis" : "Todos os dias"}
                </td>
                <td class="mono align-right">{formatCents(item.base_cents)}</td>
                <td class="mono align-right amount-positive">
                  {formatCents(item.yield_cents)}
                </td>
                <td class="mono">
                  {item.contracted_percent_bps / 100}% CDI ·
                  {(item.daily_rate * 100).toFixed(6)}%/dia
                </td>
                <td>
                  <span
                    class="tag {item.status === 'confirmed'
                      ? 'tag--active'
                      : 'tag--archived'}"
                  >
                    {item.status === "confirmed" ? "Confirmado" : "Estornado"}
                  </span>
                </td>
                <td class="align-right">
                  {#if !archived && item.status === "confirmed"}
                    <div class="row-actions">
                      <button
                        type="button"
                        class="button button--small button--quiet"
                        disabled={busy}
                        onclick={() => void reverseYield(item)}
                      >
                        Estornar
                      </button>
                      <button
                        type="button"
                        class="button button--small button--quiet"
                        disabled={busy}
                        onclick={() => void substituteYield(item)}
                      >
                        Substituir
                      </button>
                    </div>
                  {:else}
                    —
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>

  <section class="panel" aria-labelledby="savings-transfer-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">Caixa</span>
        <h3 class="panel-title" id="savings-transfer-title">Aplicações e resgates</h3>
      </div>
    </header>

    {#if archived}
      <div class="empty-panel">
        <p>Cofrinho arquivado não aceita novas transferências.</p>
      </div>
    {:else if regularAccounts.length === 0}
      <div class="empty-panel">
        <p>
          Cadastre uma conta bancária ativa para aplicar ou resgatar neste
          cofrinho.
        </p>
      </div>
    {:else}
      <div class="filters">
        <label class="field">
          <span>Operação</span>
          <select bind:value={transferMode}>
            <option value="apply">Aplicar (conta → cofrinho)</option>
            <option value="redeem">Resgatar (cofrinho → conta)</option>
          </select>
        </label>
        <label class="field">
          <span>Conta convencional</span>
          <select bind:value={counterpartyId}>
            <option value="">Selecione</option>
            {#each regularAccounts as item (item.id)}
              <option value={item.id}>{item.name}</option>
            {/each}
          </select>
        </label>
        <label class="field">
          <span>Valor</span>
          <input
            class="money-input"
            inputmode="decimal"
            bind:value={transferAmount}
            placeholder="0,00"
          />
        </label>
        <label class="field">
          <span>Data</span>
          <input type="date" bind:value={transferDate} />
        </label>
        <label class="field field--wide">
          <span>Descrição (opcional)</span>
          <input bind:value={transferDescription} />
        </label>
        <div class="field field--inline">
          <button
            type="button"
            class="button button--primary"
            disabled={busy}
            onclick={submitTransfer}
          >
            Registrar transferência
          </button>
        </div>
      </div>
      <div class="panel-footer">
        <div class="panel-footnote">
          <span class="mono">transferência</span>
          <span>
            A operação aparece em Movimentações nas duas contas e não altera
            receitas, despesas ou fluxo de caixa.
          </span>
        </div>
      </div>
    {/if}
  </section>

  <section class="panel" aria-labelledby="savings-history-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">Livro</span>
        <h3 class="panel-title" id="savings-history-title">Histórico do cofrinho</h3>
      </div>
      <span class="panel-meta">{history.length} lançamentos</span>
    </header>

    {#if history.length === 0}
      <div class="empty-panel">
        <p>Nenhum lançamento neste cofrinho.</p>
      </div>
    {:else}
      <div class="table-wrap">
        <table class="data-table">
          <thead>
            <tr>
              <th scope="col">Data</th>
              <th scope="col">Tipo</th>
              <th scope="col">Descrição</th>
              <th scope="col" class="align-right">Valor</th>
            </tr>
          </thead>
          <tbody>
            {#each history as movement (movement.id)}
              <tr>
                <td>{formatDate(movement.occurred_on)}</td>
                <td><span class="tag">{kindLabel(movement.kind)}</span></td>
                <td>{movement.description ?? "—"}</td>
                <td class="mono align-right">
                  {formatCents(movement.amount_cents)}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
{/if}
