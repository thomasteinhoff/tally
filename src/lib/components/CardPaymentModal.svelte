<script lang="ts">
  import { untrack } from "svelte";

  import type {
    AccountDto,
    CardSummaryDto,
    CreateCardPaymentInput,
  } from "../api/bindings";
  import {
    currentTime,
    formatCents,
    normalizeTimeInput,
    parseAmountToCents,
    todayIso,
  } from "../format";
  import Modal from "./Modal.svelte";

  let {
    cards,
    accounts,
    defaultCardId = "",
    saving,
    error,
    onsave,
    onclose,
  }: {
    cards: CardSummaryDto[];
    accounts: AccountDto[];
    defaultCardId?: string;
    saving: boolean;
    error: string | null;
    onsave: (input: CreateCardPaymentInput) => void;
    onclose: () => void;
  } = $props();

  const initial = untrack(() => ({
    cardId: defaultCardId || cards[0]?.id || "",
    bankId: "",
    occurredOn: todayIso(),
    time: currentTime(),
  }));

  let cardId = $state(initial.cardId);
  let bankId = $state(initial.bankId);
  let amountText = $state("");
  let occurredOn = $state(initial.occurredOn);
  let occurredTime = $state(initial.time);
  let manualPeriod = $state("");
  let description = $state("");
  let localError = $state<string | null>(null);

  const bankOptions = $derived(
    accounts.filter(
      (account) =>
        account.account_type !== "credit_card" && !account.archived_at,
    ),
  );

  const selectedCard = $derived(cards.find((card) => card.id === cardId));

  function blurTime(): void {
    const normalized = normalizeTimeInput(occurredTime);
    if (normalized !== null) {
      occurredTime = normalized;
    }
  }

  function submit(): void {
    localError = null;

    const amountCents = parseAmountToCents(amountText);
    if (amountCents === null || amountCents <= 0) {
      localError = "Informe um valor maior que zero.";
      return;
    }
    if (!cardId) {
      localError = "Escolha o cartão.";
      return;
    }
    if (!bankId) {
      localError = "Escolha a conta de origem.";
      return;
    }

    const normalizedTime = normalizeTimeInput(occurredTime);
    if (normalizedTime === null) {
      localError = "Informe um horário válido no formato 24h (HH:MM).";
      return;
    }
    if (!normalizedTime && !manualPeriod) {
      localError = "Informe um horário ou escolha um período manual.";
      return;
    }

    onsave({
      card_account_id: cardId,
      bank_account_id: bankId,
      amount_cents: amountCents,
      occurred_on: occurredOn,
      occurred_time: normalizedTime || null,
      manual_period: normalizedTime ? null : manualPeriod,
      description: description.trim() || null,
    });
  }
</script>

<Modal title="Pagamento de cartão" {onclose}>
  <form
    class="form"
    onsubmit={(event) => {
      event.preventDefault();
      submit();
    }}
  >
    <div class="form-grid">
      <label class="field">
        <span>Cartão</span>
        <select bind:value={cardId} required>
          <option value="" disabled>Escolha o cartão</option>
          {#each cards as card (card.id)}
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

      <label class="field">
        <span>Data</span>
        <input type="date" bind:value={occurredOn} required />
      </label>

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

      <label class="field field--wide">
        <span>Descrição <small>opcional</small></span>
        <input
          bind:value={description}
          maxlength="500"
          placeholder="Ex.: fatura de março"
        />
      </label>
    </div>

    <p class="form-hint">
      O pagamento reduz a dívida e o caixa, mas não vira despesa. A alocação é
      FIFO: primeiro a dívida inicial do cartão, depois as parcelas mais antigas.
      {#if selectedCard && selectedCard.debt_cents > 0}
        Dívida atual: {formatCents(selectedCard.debt_cents)}.
      {/if}
    </p>

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
        {saving ? "Salvando…" : "Registrar pagamento"}
      </button>
    </footer>
  </form>
</Modal>
