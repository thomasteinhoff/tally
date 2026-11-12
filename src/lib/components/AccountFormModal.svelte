<script lang="ts">
  import { untrack } from "svelte";

  import type { AccountDto } from "../api/bindings";
  import type { AccountFormRequest } from "../form-types";
  import {
    accountTypeLabel,
    formatAmountInput,
    parseAmountToCents,
  } from "../format";
  import Modal from "./Modal.svelte";

  let {
    account = null,
    presetType = null,
    saving,
    error,
    onsave,
    onclose,
  }: {
    account?: AccountDto | null;
    presetType?: string | null;
    saving: boolean;
    error: string | null;
    onsave: (request: AccountFormRequest) => void;
    onclose: () => void;
  } = $props();

  const initial = untrack(() => ({
    editing: account !== null,
    name: account?.name ?? "",
    accountType: account?.account_type ?? presetType ?? "bank",
    lockType: account !== null || presetType !== null,
    creditLimitText:
      account?.credit_limit_cents !== null &&
      account?.credit_limit_cents !== undefined
        ? formatAmountInput(account.credit_limit_cents)
        : "",
  }));
  const editing = initial.editing;
  const lockType = initial.lockType;
  const accountTypes = [
    "bank",
    "cash",
    "savings",
    "investment",
    "digital_wallet",
    "other_asset",
    "credit_card",
  ];

  let name = $state(initial.name);
  let accountType = $state(initial.accountType);
  let openingText = $state("0,00");
  let creditLimitText = $state(initial.creditLimitText);
  let localError = $state<string | null>(null);

  const isCard = $derived(accountType === "credit_card");

  function submit(): void {
    localError = null;

    if (!name.trim()) {
      localError = "Informe o nome da conta.";
      return;
    }

    const openingBalanceCents = editing
      ? 0
      : parseAmountToCents(openingText);
    if (!editing && openingBalanceCents === null) {
      localError = isCard
        ? "Informe uma dívida inicial válida."
        : "Informe um saldo inicial válido.";
      return;
    }
    if (isCard && (openingBalanceCents ?? 0) < 0) {
      localError = "A dívida inicial do cartão não pode ser negativa.";
      return;
    }

    let creditLimitCents: number | null = null;
    if (isCard && creditLimitText.trim()) {
      creditLimitCents = parseAmountToCents(creditLimitText);
      if (creditLimitCents === null || creditLimitCents <= 0) {
        localError = "Informe um limite de crédito válido ou deixe em branco.";
        return;
      }
    }

    onsave({
      mode: editing ? "edit" : "create",
      id: account?.id ?? null,
      name: name.trim(),
      accountType,
      openingBalanceCents: openingBalanceCents ?? 0,
      creditLimitCents,
    });
  }
</script>

<Modal
  title={editing
    ? "Editar conta"
    : isCard
      ? "Novo cartão"
      : "Nova conta"}
  {onclose}
>
  <form
    class="form"
    onsubmit={(event) => {
      event.preventDefault();
      submit();
    }}
  >
    <div class="form-grid">
      <label class="field field--wide">
        <span>Nome</span>
        <input
          bind:value={name}
          maxlength="120"
          placeholder="Ex.: Banco do Brasil"
          required
        />
      </label>

      <label class="field">
        <span>Tipo</span>
        <select bind:value={accountType} disabled={lockType}>
          {#each accountTypes as type (type)}
            <option value={type}>{accountTypeLabel(type)}</option>
          {/each}
        </select>
      </label>

      {#if !editing}
        <label class="field">
          <span>{isCard ? "Dívida inicial (passivo)" : "Saldo inicial"}</span>
          <input
            class="money-input"
            inputmode="decimal"
            bind:value={openingText}
            placeholder="0,00"
          />
        </label>
      {/if}

      {#if isCard}
        <label class="field">
          <span>Limite de crédito <small>opcional</small></span>
          <input
            class="money-input"
            inputmode="decimal"
            bind:value={creditLimitText}
            placeholder="0,00"
          />
        </label>
      {/if}
    </div>

    {#if isCard}
      <p class="form-hint">
        O cartão é um passivo: não tem saldo de ativo nem aceita saldo negativo.
        {#if !editing}
          A dívida inicial é opcional e entra contra o patrimônio; compras depois
          aumentam a dívida e o pagamento não vira despesa.
        {:else}
          Para corrigir a dívida, use um ajuste no cartão.
        {/if}
        O limite é informativo e não bloqueia compras.
      </p>
    {:else if !editing}
      <p class="form-hint">
        O saldo inicial pode ser negativo. Ele entra contra o patrimônio e não é
        contado como receita. Zero não cria lançamento.
      </p>
    {:else}
      <p class="form-hint">
        O tipo da conta é imutável. O saldo é sempre derivado dos lançamentos;
        para corrigi-lo, use um ajuste.
      </p>
    {/if}

    {#if error || localError}
      <div class="inline-result inline-result--warn" role="alert">
        <span class="mono">{error ? "ERRO" : "VALIDAR"}</span>
        <span>{error ?? localError}</span>
      </div>
    {/if}

    <footer class="form-actions">
      <button type="button" class="button" onclick={onclose} disabled={saving}>
        Cancelar
      </button>
      <button type="submit" class="button button--primary" disabled={saving}>
        {saving ? "Salvando…" : editing ? "Salvar alterações" : "Criar conta"}
      </button>
    </footer>
  </form>
</Modal>
