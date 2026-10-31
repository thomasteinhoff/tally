<script lang="ts">
  import { untrack } from "svelte";

  import type { AccountDto } from "../api/bindings";
  import type { AccountFormRequest } from "../form-types";
  import { accountTypeLabel, parseAmountToCents } from "../format";
  import Modal from "./Modal.svelte";

  let {
    account = null,
    saving,
    error,
    onsave,
    onclose,
  }: {
    account?: AccountDto | null;
    saving: boolean;
    error: string | null;
    onsave: (request: AccountFormRequest) => void;
    onclose: () => void;
  } = $props();

  const initial = untrack(() => ({
    editing: account !== null,
    name: account?.name ?? "",
    accountType: account?.account_type ?? "bank",
  }));
  const editing = initial.editing;
  const accountTypes = [
    "bank",
    "cash",
    "savings",
    "investment",
    "digital_wallet",
    "other_asset",
  ];

  let name = $state(initial.name);
  let accountType = $state(initial.accountType);
  let openingText = $state("0,00");
  let localError = $state<string | null>(null);

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
      localError = "Informe um saldo inicial válido.";
      return;
    }

    onsave({
      mode: editing ? "edit" : "create",
      id: account?.id ?? null,
      name: name.trim(),
      accountType,
      openingBalanceCents: openingBalanceCents ?? 0,
    });
  }
</script>

<Modal title={editing ? "Editar conta" : "Nova conta"} {onclose}>
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
        <select bind:value={accountType} disabled={editing}>
          {#each accountTypes as type (type)}
            <option value={type}>{accountTypeLabel(type)}</option>
          {/each}
        </select>
      </label>

      {#if !editing}
        <label class="field">
          <span>Saldo inicial</span>
          <input
            class="money-input"
            inputmode="decimal"
            bind:value={openingText}
            placeholder="0,00"
          />
        </label>
      {/if}
    </div>

    {#if !editing}
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
        {saving ? "Salvando…" : editing ? "Salvar nome" : "Criar conta"}
      </button>
    </footer>
  </form>
</Modal>
