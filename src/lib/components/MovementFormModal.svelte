<script lang="ts">
  import { untrack } from "svelte";

  import type {
    AccountDto,
    CardSummaryDto,
    CategoryDto,
    MovementDto,
  } from "../api/bindings";
  import {
    currentTime,
    formatAmountInput,
    formatCents,
    movementAmountForInput,
    normalizeTimeInput,
    parseAmountToCents,
    periodRangeLabel,
    todayIso,
  } from "../format";
  import type {
    MovementDraft,
    MovementFormKind,
    MovementFormRequest,
    SimpleMovementKind,
  } from "../movement-form";
  import Modal from "./Modal.svelte";

  const allKinds: MovementFormKind[] = [
    "expense",
    "income",
    "transfer",
    "card_purchase",
    "card_payment",
    "adjustment",
  ];

  const kindLabels: Record<MovementFormKind, string> = {
    expense: "Despesa",
    income: "Receita",
    transfer: "Transferência",
    card_purchase: "Compra no cartão",
    card_payment: "Pagamento de cartão",
    adjustment: "Ajuste",
  };

  let {
    accounts,
    categories,
    cards = [],
    movement = null,
    startedTime = "",
    defaultKind = "expense",
    allowedKinds = allKinds,
    defaultCardId = "",
    saving,
    error,
    onsave,
    onclose,
  }: {
    accounts: AccountDto[];
    categories: CategoryDto[];
    cards?: CardSummaryDto[];
    movement?: MovementDto | null;
    startedTime?: string;
    defaultKind?: MovementFormKind;
    allowedKinds?: MovementFormKind[];
    defaultCardId?: string;
    saving: boolean;
    error: string | null;
    onsave: (request: MovementFormRequest) => void;
    onclose: () => void;
  } = $props();

  const initial = untrack(() => {
    const current = movement;
    const editing = current !== null;
    const initialKind: MovementFormKind = current
      ? (current.kind as MovementFormKind)
      : allowedKinds.includes(defaultKind)
        ? defaultKind
        : (allowedKinds[0] ?? "expense");
    const initialPaymentMethod =
      current?.payment_method ??
      (initialKind === "card_purchase" ? "credit" : "pix");

    return {
      editing,
      kind: initialKind,
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
      paymentMethod: initialPaymentMethod,
      cardId: defaultCardId || cards[0]?.id || "",
      bankId: "",
      paymentMode: "cash" as "cash" | "installments",
      installmentCount: 1,
      firstDueOn: todayIso(),
    };
  });
  const editing = initial.editing;

  let kind = $state<MovementFormKind>(initial.kind);
  let accountId = $state(initial.accountId);
  let destinationId = $state(initial.destinationId);
  let categoryId = $state(initial.categoryId);
  let amountText = $state(initial.amountText);
  let occurredOn = $state(initial.occurredOn);
  let occurredTime = $state(initial.occurredTime);
  let manualPeriod = $state(initial.manualPeriod);
  let description = $state(initial.description);
  let paymentMethod = $state(initial.paymentMethod);
  let cardId = $state(initial.cardId);
  let bankId = $state(initial.bankId);
  let paymentMode = $state<"cash" | "installments">(initial.paymentMode);
  let installmentCount = $state(initial.installmentCount);
  let firstDueOn = $state(initial.firstDueOn);
  let adjustmentMode = $state<"add" | "subtract" | "set">("add");
  let localError = $state<string | null>(null);

  const isCardPurchase = $derived(kind === "card_purchase");
  const isCardPayment = $derived(kind === "card_payment");
  const isAdjustment = $derived(kind === "adjustment");
  const isTransfer = $derived(kind === "transfer");
  const needsCategory = $derived(kind === "expense" || kind === "income");
  const visibleKinds = $derived(
    editing ? [kind] : allKinds.filter((item) => allowedKinds.includes(item)),
  );

  // Cartão é passivo e só participa de ajuste simples; compras e pagamentos têm
  // campos próprios no mesmo formulário.
  const accountOptions = $derived(
    accounts.filter(
      (account) =>
        (isAdjustment || account.account_type !== "credit_card") &&
        (!account.archived_at ||
          account.id === accountId ||
          account.id === destinationId),
    ),
  );

  const bankOptions = $derived(
    accounts.filter(
      (account) =>
        account.account_type !== "credit_card" && !account.archived_at,
    ),
  );

  const cardOptions = $derived(
    cards.filter((card) => !card.archived_at || card.id === cardId),
  );

  const selectedAccount = $derived(
    accounts.find((account) => account.id === accountId) ?? null,
  );
  const selectedCard = $derived(
    cards.find((card) => card.id === cardId) ?? null,
  );
  const currentBalanceCents = $derived(
    selectedAccount?.balance_cents ?? null,
  );
  const adjustmentResultCents = $derived.by(() => {
    if (!isAdjustment || editing || currentBalanceCents === null) {
      return null;
    }
    const value = parseAmountToCents(amountText);
    if (value === null) {
      return null;
    }
    if (adjustmentMode === "set") {
      return value;
    }
    const magnitude = Math.abs(value);
    return adjustmentMode === "subtract"
      ? currentBalanceCents - magnitude
      : currentBalanceCents + magnitude;
  });

  const categoryOptions = $derived(
    categories.filter(
      (category) =>
        category.kind === (isCardPurchase ? "expense" : kind) &&
        (!category.archived_at || category.id === categoryId),
    ),
  );

  const title = $derived(
    editing
      ? "Editar movimentação"
      : isCardPurchase
        ? "Compra no cartão"
        : isCardPayment
          ? "Pagamento de cartão"
          : "Nova movimentação",
  );

  function changeKind(event: Event): void {
    kind = (event.currentTarget as HTMLSelectElement)
      .value as MovementFormKind;
    const selected = accounts.find((account) => account.id === accountId);
    if (
      kind !== "adjustment" &&
      selected?.account_type === "credit_card"
    ) {
      accountId = "";
    }
    destinationId = "";
    categoryId = "";
    bankId = "";
    paymentMethod = kind === "card_purchase" ? "credit" : "pix";
    paymentMode = "cash";
    installmentCount = 1;
    firstDueOn = todayIso();
    if (kind === "adjustment") {
      adjustmentMode = "add";
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

    const parsedCents = parseAmountToCents(amountText);
    if (parsedCents === null) {
      localError = "Informe um valor válido, como 1.234,56.";
      return;
    }
    if (!isAdjustment && parsedCents <= 0) {
      localError = "Informe um valor maior que zero.";
      return;
    }

    if (isCardPurchase) {
      const count = paymentMode === "cash" ? 1 : installmentCount;
      if (paymentMode === "installments" && count < 2) {
        localError = "A compra parcelada precisa de pelo menos 2 parcelas.";
        return;
      }
      if (!Number.isInteger(count) || count < 1) {
        localError = "A quantidade de parcelas deve começar em 1.";
        return;
      }
      if (count > 60) {
        localError = "O máximo é 60 parcelas.";
        return;
      }
      if (count > parsedCents) {
        localError = "Cada parcela precisa ter pelo menos um centavo.";
        return;
      }
      if (!cardId) {
        localError = "Escolha o cartão.";
        return;
      }
      if (!categoryId) {
        localError = "Escolha a categoria da despesa.";
        return;
      }
      if (!firstDueOn) {
        localError = "Informe a data da primeira parcela.";
        return;
      }
      if (!normalizedTime && !manualPeriod) {
        localError = "Informe um horário ou escolha um período manual.";
        return;
      }

      onsave({
        form: "card_purchase",
        input: {
          card_account_id: cardId,
          category_id: categoryId,
          total_cents: parsedCents,
          installment_count: count,
          first_due_on: firstDueOn,
          occurred_on: occurredOn,
          occurred_time: normalizedTime || null,
          manual_period: normalizedTime ? null : manualPeriod,
          description: description.trim() || null,
          payment_method: paymentMethod,
        },
      });
      return;
    }

    if (isCardPayment) {
      if (!cardId) {
        localError = "Escolha o cartão.";
        return;
      }
      if (!bankId) {
        localError = "Escolha a conta de origem.";
        return;
      }
      if (!normalizedTime && !manualPeriod) {
        localError = "Informe um horário ou escolha um período manual.";
        return;
      }

      onsave({
        form: "card_payment",
        input: {
          card_account_id: cardId,
          bank_account_id: bankId,
          amount_cents: parsedCents,
          occurred_on: occurredOn,
          occurred_time: normalizedTime || null,
          manual_period: normalizedTime ? null : manualPeriod,
          description: description.trim() || null,
        },
      });
      return;
    }

    if (!accountId) {
      localError = "Escolha a conta da movimentação.";
      return;
    }

    let amountCents = parsedCents;
    if (isAdjustment && !editing) {
      if (currentBalanceCents === null) {
        localError = "Escolha a conta do ajuste.";
        return;
      }
      if (adjustmentMode === "set") {
        amountCents = parsedCents - currentBalanceCents;
        if (amountCents === 0) {
          localError = "O novo saldo é igual ao saldo atual.";
          return;
        }
      } else {
        const magnitude = Math.abs(parsedCents);
        if (magnitude === 0) {
          localError = "O ajuste precisa de um valor diferente de zero.";
          return;
        }
        amountCents =
          adjustmentMode === "subtract" ? -magnitude : magnitude;
      }
    } else if (isAdjustment && amountCents === 0) {
      localError = "O ajuste precisa de um valor diferente de zero.";
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
      kind: kind as SimpleMovementKind,
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
      form: "movement",
      mode: editing ? "edit" : "create",
      id: movement?.id ?? null,
      draft,
    });
  }
</script>

<Modal {title} {onclose}>
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
          {#each visibleKinds as option (option)}
            <option value={option}>{kindLabels[option]}</option>
          {/each}
        </select>
      </label>

      <label class="field">
        <span>Data</span>
        <input type="date" bind:value={occurredOn} required />
      </label>

      {#if isCardPurchase}
        <label class="field">
          <span>Cartão</span>
          <select bind:value={cardId} required>
            <option value="" disabled>Escolha o cartão</option>
            {#each cardOptions as card (card.id)}
              <option value={card.id}>{card.name}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span>Forma</span>
          <select
            bind:value={paymentMode}
            onchange={() => {
              if (paymentMode === "installments" && installmentCount < 2) {
                installmentCount = 2;
              }
            }}
          >
            <option value="cash">À vista</option>
            <option value="installments">Parcelada</option>
          </select>
        </label>

        {#if paymentMode === "installments"}
          <label class="field">
            <span>Parcelas</span>
            <input
              type="number"
              min="2"
              max="60"
              bind:value={installmentCount}
              required
            />
          </label>
        {/if}

        <label class="field field--wide">
          <span>Valor total <small>use vírgula para centavos</small></span>
          <input
            class="money-input"
            inputmode="decimal"
            bind:value={amountText}
            placeholder="0,00"
            required
          />
        </label>

        <label class="field">
          <span>
            {paymentMode === "cash" ? "Vencimento" : "Primeira parcela"}
          </span>
          <input type="date" bind:value={firstDueOn} required />
        </label>

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
            <option value="credit">Crédito</option>
            <option value="subscription_credit">Assinatura (crédito)</option>
          </select>
        </label>
      {:else if isCardPayment}
        <label class="field">
          <span>Cartão</span>
          <select bind:value={cardId} required>
            <option value="" disabled>Escolha o cartão</option>
            {#each cardOptions as card (card.id)}
              <option value={card.id}>{card.name}</option>
            {/each}
          </select>
        </label>

        <label class="field">
          <span>Conta de origem</span>
          <select bind:value={bankId} required>
            <option value="" disabled>Escolha a conta</option>
            {#each bankOptions as account (account.id)}
              <option value={account.id}>{account.name}</option>
            {/each}
          </select>
        </label>

        <label class="field field--wide">
          <span>Valor <small>use vírgula para centavos</small></span>
          <input
            class="money-input"
            inputmode="decimal"
            bind:value={amountText}
            placeholder="0,00"
            required
          />
        </label>
      {:else}
        {#if isAdjustment && !editing}
          <label class="field">
            <span>Como ajustar</span>
            <select bind:value={adjustmentMode}>
              <option value="add">Somar ao saldo</option>
              <option value="subtract">Subtrair do saldo</option>
              <option value="set">Definir novo saldo</option>
            </select>
          </label>
        {/if}

        <label class="field field--wide">
          <span>
            {isAdjustment
              ? editing
                ? "Valor com sinal"
                : adjustmentMode === "set"
                  ? "Novo saldo"
                  : "Valor do ajuste"
              : "Valor"}
            <small>use vírgula para centavos</small>
          </span>
          <input
            class="money-input"
            inputmode="decimal"
            bind:value={amountText}
            placeholder={isAdjustment && (editing || adjustmentMode === "set")
              ? "-100,00"
              : "0,00"}
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
      {/if}

      {#if !isAdjustment}
        <label class="field">
          <span>Horário <small>opcional · 24h</small></span>
          <input
            type="text"
            inputmode="numeric"
            maxlength="5"
            placeholder="HH:MM"
            value={occurredTime}
            oninput={(event) => {
              occurredTime = event.currentTarget.value;
              if (event.currentTarget.value.trim()) manualPeriod = "";
            }}
            onblur={blurTime}
          />
        </label>

        <label class="field">
          <span>ou Período manual</span>
          <select
            value={manualPeriod}
            required={!occurredTime}
            onchange={(event) => {
              manualPeriod = event.currentTarget.value;
              if (manualPeriod) occurredTime = "";
            }}
          >
            <option value="" disabled>Escolha o período</option>
            <option value="madrugada">{periodRangeLabel("madrugada")}</option>
            <option value="manha">{periodRangeLabel("manha")}</option>
            <option value="tarde">{periodRangeLabel("tarde")}</option>
            <option value="noite">{periodRangeLabel("noite")}</option>
            <option value="custom">{periodRangeLabel("custom")}</option>
          </select>
        </label>
      {/if}

      <label class="field field--wide">
        <span>Descrição <small>opcional</small></span>
        <input
          bind:value={description}
          maxlength="500"
          placeholder={isCardPurchase ? "Ex.: notebook" : "Ex.: mercado da semana"}
        />
      </label>
    </div>

    {#if !isAdjustment}
      <p class="form-hint">
        Horário e período manual são alternativas: preencher um limpa o outro.
        Faixas: Madrugada (00:00–04:59), Manhã (05:00–11:59), Tarde
        (12:00–18:59), Noite (19:00–23:59).
      </p>
    {/if}

    {#if isAdjustment}
      {#if selectedAccount && currentBalanceCents !== null}
        <div class="inline-result" role="status">
          <span class="mono">SALDO ATUAL</span>
          <span>
            {selectedAccount.name}:
            <strong class="mono">{formatCents(currentBalanceCents)}</strong>
            {#if !editing && adjustmentResultCents !== null}
              → <strong class="mono">{formatCents(adjustmentResultCents)}</strong>
            {/if}
            {#if selectedAccount.account_type === "credit_card"}
              <small> (no cartão, a dívida aparece como saldo negativo)</small>
            {/if}
          </span>
        </div>
      {/if}
      <p class="form-hint">
        Ajuste corrige o saldo da conta contra o patrimônio, sem virar receita
        ou despesa. {editing
          ? "Use valor negativo para reduzir."
          : adjustmentMode === "set"
            ? "O valor informado passa a ser o saldo final."
            : "O valor é somado ou subtraído do saldo atual."}
      </p>
    {:else if isCardPurchase}
      <p class="form-hint">
        A compra aumenta a dívida total do cartão imediatamente. Cada parcela vira
        despesa na data de vencimento; o total não é despesa do mês da compra.
      </p>
    {:else if isCardPayment}
      <p class="form-hint">
        O pagamento reduz a dívida e o caixa, mas não vira despesa. A alocação é
        FIFO: primeiro a dívida inicial do cartão, depois as parcelas mais
        antigas.
        {#if selectedCard && selectedCard.debt_cents > 0}
          Dívida atual: {formatCents(selectedCard.debt_cents)}.
        {/if}
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
