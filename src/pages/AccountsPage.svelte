<script lang="ts">
  import {
    createMutation,
    createQuery,
    useQueryClient,
  } from "@tanstack/svelte-query";

  import type { AccountDto } from "../lib/api/bindings";
  import {
    createAccount,
    deleteAccount,
    listAccounts,
    setAccountArchived,
    updateAccount,
  } from "../lib/api";
  import AccountFormModal from "../lib/components/AccountFormModal.svelte";
  import { confirmAction } from "../lib/confirm";
  import { normalizeError } from "../lib/errors";
  import {
    accountTypeLabel,
    formatCents,
  } from "../lib/format";
  import type { AccountFormRequest } from "../lib/form-types";
  import { notify } from "../lib/toast.svelte";

  const queryClient = useQueryClient();

  let includeArchived = $state(false);
  let composerOpen = $state(false);
  let editing = $state<AccountDto | null>(null);

  const accountsQuery = createQuery(() => ({
    queryKey: ["accounts", includeArchived],
    queryFn: () => listAccounts(includeArchived),
  }));

  const saveMutation = createMutation(() => ({
    mutationFn: (request: AccountFormRequest) => saveRequest(request),
    onSuccess: () => {
      void queryClient.invalidateQueries();
      composerOpen = false;
      editing = null;
      notify("Conta salva.");
    },
  }));

  const archiveMutation = createMutation(() => ({
    mutationFn: (input: { id: string; archived: boolean }) =>
      setAccountArchived(input.id, input.archived),
    onSuccess: (_account, input) => {
      void queryClient.invalidateQueries();
      notify(input.archived ? "Conta arquivada." : "Conta restaurada.");
    },
  }));

  const deleteMutation = createMutation(() => ({
    mutationFn: deleteAccount,
    onSuccess: () => {
      void queryClient.invalidateQueries();
      notify("Conta excluída.");
    },
  }));

  const accounts = $derived(accountsQuery.data ?? []);
  const activeAccounts = $derived(accounts.filter((account) => !account.archived_at));
  const activeRegular = $derived(
    activeAccounts.filter((account) => !account.is_savings),
  );
  const savingsAccounts = $derived(accounts.filter((account) => account.is_savings));
  const accountsCents = $derived(
    activeRegular.reduce((total, account) => total + account.balance_cents, 0),
  );
  // Arquivados continuam aqui: o saldo do cofrinho não desaparece.
  const savingsCents = $derived(
    savingsAccounts.reduce((total, account) => total + account.balance_cents, 0),
  );
  const queryError = $derived(
    accountsQuery.error ? normalizeError(accountsQuery.error) : null,
  );
  const saveError = $derived(
    saveMutation.error ? normalizeError(saveMutation.error).message : null,
  );

  function accountHref(account: AccountDto): string {
    if (account.is_savings) {
      return `#/cofrinhos/${account.id}`;
    }
    return account.account_type === "credit_card"
      ? `#/cartoes/${account.id}`
      : `#/contas/${account.id}`;
  }

  function openNew(): void {
    editing = null;
    composerOpen = true;
    saveMutation.reset();
  }

  function openEdit(account: AccountDto): void {
    editing = account;
    composerOpen = true;
    saveMutation.reset();
  }

  function closeComposer(): void {
    if (saveMutation.isPending) {
      return;
    }
    composerOpen = false;
    editing = null;
    saveMutation.reset();
  }

  async function saveRequest(request: AccountFormRequest): Promise<AccountDto> {
    if (request.mode === "edit") {
      if (!request.id) {
        throw new Error("Conta sem identificador.");
      }
      return updateAccount(
        request.id,
        request.name,
        request.creditLimitCents,
      );
    }

    return createAccount({
      name: request.name,
      account_type: request.accountType,
      opening_balance_cents: request.openingBalanceCents,
      credit_limit_cents: request.creditLimitCents,
      is_savings: request.isSavings,
      yield_mode: request.yieldMode ?? undefined,
      cdi_percent: request.cdiPercent ?? undefined,
    });
  }

  async function confirmArchive(account: AccountDto): Promise<void> {
    const archived = !account.archived_at;
    const confirmed = await confirmAction(
      archived
        ? `Arquivar “${account.name}”? O histórico e o saldo permanecem; a conta deixa de aparecer em novos lançamentos.`
        : `Restaurar “${account.name}” para uso em novas movimentações?`,
      archived ? "Arquivar conta" : "Restaurar conta",
      archived ? "Arquivar" : "Restaurar",
    );
    if (confirmed) {
      archiveMutation.mutate({ id: account.id, archived });
    }
  }

  async function confirmDelete(account: AccountDto): Promise<void> {
    const detail = account.is_savings
      ? "Cofrinhos só podem ser excluídos com saldo inicial. Se houver aplicações, resgates, ajustes ou rendimentos, arquive em vez de excluir."
      : "Só é permitido quando não há histórico. O saldo inicial e a conta serão removidos definitivamente.";
    const confirmed = await confirmAction(
      `Excluir “${account.name}”? ${detail}`,
      "Excluir conta",
      "Excluir",
    );
    if (confirmed) {
      deleteMutation.mutate(account.id);
    }
  }
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Patrimônio</span>
    <h2>Contas</h2>
    <p>
      Contas de ativo e cartões com saldo derivado do ledger. Arquivar preserva
      o histórico; excluir só é possível sem movimentações.
    </p>
  </div>
  <button type="button" class="button button--primary" onclick={openNew}>
    Nova conta
  </button>
</section>

<nav class="view-switch" aria-label="Contas e cartões">
  <a
    class="view-switch-item is-active"
    href="#/contas"
    aria-current="page">Contas</a
  >
  <a class="view-switch-item" href="#/cartoes">Cartões</a>
</nav>

<section class="summary-strip" aria-label="Resumo das contas ativas">
  <div>
    <span class="data-label">Patrimônio em contas</span>
    <strong class="summary-value">{formatCents(accountsCents)}</strong>
  </div>
  <div>
    <span class="data-label">Cofrinhos</span>
    <strong class="summary-value">{formatCents(savingsCents)}</strong>
  </div>
  <div>
    <span class="data-label">Patrimônio total com cofrinhos</span>
    <strong class="summary-value">
      {formatCents(accountsCents + savingsCents)}
    </strong>
  </div>
  <div>
    <span class="data-label">Contas e cartões ativos</span>
    <strong class="summary-value">{activeRegular.length}</strong>
  </div>
</section>

<section class="panel" aria-labelledby="accounts-title">
  <header class="panel-header">
    <div class="panel-heading">
      <span class="panel-kicker">Cadastro</span>
      <h3 class="panel-title" id="accounts-title">Contas</h3>
    </div>
    <label class="toggle">
      <input type="checkbox" bind:checked={includeArchived} />
      <span>
        Mostrar arquivadas
        <span
          class="info-mark"
          role="img"
          aria-label="Contas arquivadas não aparecem em novos lançamentos. O histórico e o saldo continuam disponíveis no extrato."
          title="Contas arquivadas não aparecem em novos lançamentos. O histórico e o saldo continuam disponíveis no extrato."
          >i</span
        >
      </span>
    </label>
  </header>

  {#if accountsQuery.isPending}
    <div class="state-panel" aria-busy="true">
      <span class="state-kicker">Lendo</span>
      <h2>Consultando contas.</h2>
      <div class="loading-rule" aria-hidden="true"></div>
    </div>
  {:else if queryError}
    <div class="state-panel state-panel--error" role="alert">
      <span class="state-kicker">{queryError.code}</span>
      <h2>Não foi possível carregar as contas.</h2>
      <p>{queryError.message}</p>
      <div class="state-actions">
        <button
          type="button"
          class="button button--primary"
          onclick={() => accountsQuery.refetch()}
        >
          Tentar novamente
        </button>
      </div>
    </div>
  {:else if accounts.length === 0}
    <div class="empty-panel">
      <p>Nenhuma conta cadastrada.</p>
      <button type="button" class="button" onclick={openNew}>
        Criar a primeira
      </button>
    </div>
  {:else}
    <div class="table-wrap">
      <table class="data-table">
        <thead>
          <tr>
            <th scope="col">Conta</th>
            <th scope="col">Tipo</th>
            <th scope="col">Situação</th>
            <th scope="col" class="align-right">Saldo</th>
            <th scope="col" class="align-right">Ações</th>
          </tr>
        </thead>
        <tbody>
          {#each accounts as account (account.id)}
            <tr class:row-muted={Boolean(account.archived_at)}>
              <td>
                <a class="text-link" href={accountHref(account)}>
                  {account.name}
                </a>
              </td>
              <td>
                {account.is_savings
                  ? "Cofrinho"
                  : accountTypeLabel(account.account_type)}
              </td>
              <td>
                {#if account.archived_at}
                  <span class="tag tag--archived">Arquivada</span>
                {:else}
                  <span class="tag tag--active">Ativa</span>
                {/if}
              </td>
              <td
                class="mono align-right"
                class:amount-negative={account.balance_cents < 0}
              >
                {formatCents(account.balance_cents)}
              </td>
              <td class="align-right">
                <div class="row-actions">
                  <a class="button button--small" href={accountHref(account)}>
                    {account.is_savings
                      ? "Cofrinho"
                      : account.account_type === "credit_card"
                        ? "Cartão"
                        : "Extrato"}
                  </a>
                  <button
                    type="button"
                    class="button button--small"
                    onclick={() => openEdit(account)}
                  >
                    Editar
                  </button>
                  <button
                    type="button"
                    class="button button--small"
                    onclick={() => void confirmArchive(account)}
                    disabled={archiveMutation.isPending}
                  >
                    {account.archived_at ? "Restaurar" : "Arquivar"}
                  </button>
                  <button
                    type="button"
                    class="button button--small button--danger"
                    onclick={() => void confirmDelete(account)}
                    disabled={deleteMutation.isPending}
                  >
                    Excluir
                  </button>
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

  {#if archiveMutation.error || deleteMutation.error}
    <div class="inline-result inline-result--warn" role="alert">
      <span class="mono">ERRO</span>
      <span>
        {normalizeError(archiveMutation.error ?? deleteMutation.error).message}
      </span>
    </div>
  {/if}
</section>

{#if composerOpen}
  <AccountFormModal
    account={editing}
    saving={saveMutation.isPending}
    error={saveError}
    onsave={(request) => saveMutation.mutate(request)}
    onclose={closeComposer}
  />
{/if}
