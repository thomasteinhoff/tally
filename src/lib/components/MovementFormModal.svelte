<script lang="ts">
  import { untrack } from "svelte";

  import type { AccountDto, CategoryDto, MovementDto } from "../api/bindings";
  import {
    currentTime,
    formatAmountInput,
    movementAmountForInput,
    normalizeTimeInput,
    parseAmountToCents,
    todayIso,
  } from "../format";
  import type {
    MovementDraft,
    SaveMovementRequest,
    SimpleMovementKind,
  } from "../movement-form";
  import Modal from "./Modal.svelte";

  let {
    accounts,
    categories,
    movement = null,
    startedTime = "",
    saving,
    error,
    onsave,
    onclose,
  }: {
    accounts: AccountDto[];
    categories: CategoryDto[];
    movement?: MovementDto | null;
    startedTime?: string;
    saving: boolean;
    error: string | null;
    onsave: (request: SaveMovementRequest) => void;
    onclose: () => void;
  } = $props();

  const initial = untrack(() => {
    const current = movement;
    return {
      editing: current !== null,
      kind:
        (current?.kind as SimpleMovementKind | undefined) ?? "expense",
      accountId: current?.primary_account_id ?? "",
      destinationId: current?.counterparty_account_id ?? "",
      categoryId: current?.category_id ?? "",
      amountText: current
        ? formatAmountInput(
            movementAmountForInput(current.kind, current.amount_cents),
          )
        : "",
      occurredOn: current?.occurred_on ?? todayIso(),
      occurredTime: current
        ? (current.occurred_time ?? "")
        : startedTime || currentTime(),
      manualPeriod: current?.manual_period ?? "",
      description: current?.description ?? "",
      paymentMethod: current?.payment_method ?? "pix",
    };
  });
  const editing = initial.editing;

  let kind = $state<SimpleMovementKind>(initial.kind);
  let accountId = $state(initial.accountId);
  let destinationId = $state(initial.destinationId);
  let categoryId = $state(initial.categoryId);
  let amountText = $state(initial.amountText);
  let occurredOn = $state(initial.occurredOn);
  let occurredTime = $state(initial.occurredTime);
  let manualPeriod = $state(initial.manualPeriod);
  let description = $state(initial.description);
  let paymentMethod = $state(initial.paymentMethod);
  let localError = $state<string | null>(null);

  const isAdjustment = $derived(kind === "adjustment");
  const isTransfer = $derived(kind === "transfer");
  const needsCategory = $derived(kind === "expense" || kind === "income");

  const accountOptions = $derived(
    accounts.filter(
      (account) =>
        !account.archived_at ||
        account.id === accountId ||
        account.id === destinationId,
    ),
  );

  const categoryOptions = $derived(
    categories.filter(
      (category) =>
        category.kind === kind &&
        (!category.archived_at || category.id === categoryId),
    ),
  );

  function changeKind(event: Event): void {
    kind = (event.currentTarget as HTMLSelectElement)
      .value as SimpleMovementKind;
    destinationId = "";
    categoryId = "";
    paymentMethod = "pix";
    if (kind === "adjustment") {
      occurredTime = "";
    } else if (!occurredTime && !manualPeriod) {
      occurredTime = startedTime || currentTime();
    }
  }

  function blurTime(): void {
    const normalized = normalizeTimeInput(occurredTime);
    if (normalized !== null) {
      occurredTime = normalized;
    }
  }

  function submit(): void {
    localError = null;

    const normalizedTime = isAdjustment ? "" : normalizeTimeInput(occurredTime);
    if (normalizedTime === null) {
      localError = "Informe um horário válido no formato 24h (HH:MM).";
      return;
    }

    const amountCents = parseAmountToCents(amountText);
    if (amountCents === null) {
      localError = "Informe um valor válido, como 1.234,56.";
      return;
    }
    if (isAdjustment && amountCents === 0) {
      localError = "O ajuste precisa de um valor diferente de zero.";
      return;
    }
    if (!isAdjustment && amountCents <= 0) {
      localError = "Informe um valor maior que zero.";
      return;
    }
    if (!accountId) {
      localError = "Escolha a conta da movimentação.";
      return;
    }
    if (isTransfer && !destinationId) {
      localError = "Escolha a conta de destino.";
      return;
    }
    if (isTransfer && destinationId === accountId) {
      localError = "As contas de origem e destino devem ser diferentes.";
      return;
    }
    if (needsCategory && !categoryId) {
      localError = "Escolha a categoria.";
      return;
    }
    if (!isAdjustment && !normalizedTime && !manualPeriod) {
      localError = "Informe um horário ou escolha um período manual.";
      return;
    }

    const draft: MovementDraft = {
      kind,
      primaryAccountId: accountId,
      counterpartyAccountId: isTransfer ? destinationId : "",
      categoryId: needsCategory ? categoryId : "",
      amountCents,
      occurredOn,
      occurredTime: normalizedTime,
      manualPeriod: isAdjustment || normalizedTime ? "" : manualPeriod,
      description,
      paymentMethod: needsCategory ? paymentMethod : "",
    };

    onsave({
      mode: editing ? "edit" : "create",
      id: movement?.id ?? null,
      draft,
    });
  }
</script>

<Modal title={editing ? "Editar movimentação" : "Nova movimentação"} {onclose}>
  <form
    class="form"
    onsubmit={(event) => {
      event.preventDefault();
      submit();
    }}
  >
    <div class="form-grid">
      <label class="field">
        <span>Tipo</span>
        <select value={kind} onchange={changeKind} disabled={editing}>
          <option value="expense">Despesa</option>
          <option value="income">Receita</option>
          <option value="transfer">Transferência</option>
          <option value="adjustment">Ajuste</option>
        </select>
      </label>

      <label class="field">
        <span>Data</span>
        <input type="date" bind:value={occurredOn} required />
      </label>

      <label class="field field--wide">
        <span>
          {isAdjustment ? "Valor com sinal" : "Valor"}
          <small>use vírgula para centavos</small>
        </span>
        <input
          class="money-input"
          inputmode="decimal"
          bind:value={amountText}
          placeholder={isAdjustment ? "-100,00" : "0,00"}
          required
        />
      </label>

      <label class="field">
        <span>{isTransfer ? "Conta de origem" : "Conta"}</span>
        <select bind:value={accountId} required>
          <option value="" disabled>Escolha a conta</option>
          {#each accountOptions as account (account.id)}
            <option value={account.id}>{account.name}</option>
          {/each}
        </select>
      </label>

      {#if isTransfer}
        <label class="field">
          <span>Conta de destino</span>
          <select bind:value={destinationId} required>
            <option value="" disabled>Escolha a conta</option>
            {#each accountOptions as account (account.id)}
              {#if account.id !== accountId}
                <option value={account.id}>{account.name}</option>
              {/if}
            {/each}
          </select>
        </label>
      {/if}

      {#if needsCategory}
        <label class="field">
          <span>Categoria</span>
          <select bind:value={categoryId} required>
            <option value="" disabled>Escolha a categoria</option>
            {#each categoryOptions as category (category.id)}
              <option value={category.id}>{category.name}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span>Método</span>
          <select bind:value={paymentMethod}>
            <option value="pix">Pix</option>
            <option value="debit">Débito</option>
            <option value="cash">Dinheiro</option>
          </select>
        </label>
      {/if}

      {#if !isAdjustment}
        <label class="field">
          <span>Horário <small>opcional · 24h</small></span>
          <input
            type="text"
            inputmode="numeric"
            maxlength="5"
            placeholder="HH:MM"
            bind:value={occurredTime}
            onblur={blurTime}
          />
        </label>

        {#if !occurredTime}
          <label class="field">
            <span>Período manual</span>
            <select bind:value={manualPeriod} required>
              <option value="" disabled>Escolha o período</option>
              <option value="madrugada">Madrugada</option>
              <option value="manha">Manhã</option>
              <option value="tarde">Tarde</option>
              <option value="noite">Noite</option>
              <option value="custom">Personalizado</option>
            </select>
          </label>
        {/if}
      {/if}

      <label class="field field--wide">
        <span>Descrição <small>opcional</small></span>
        <input bind:value={description} maxlength="500" placeholder="Ex.: mercado da semana" />
      </label>
    </div>

    {#if isAdjustment}
      <p class="form-hint">
        Ajuste corrige o saldo da conta contra o patrimônio. Use valor negativo
        para reduzir. Não altera receitas nem despesas.
      </p>
    {:else if !occurredTime}
      <p class="form-hint">
        Sem horário, o período manual é obrigatório. O período é derivado pelo
        banco quando há horário.
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
        {saving ? "Salvando…" : editing ? "Salvar alterações" : "Lançar"}
      </button>
    </footer>
  </form>
</Modal>
