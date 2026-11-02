<script lang="ts">
  import { untrack } from "svelte";

  import type {
    CardSummaryDto,
    CategoryDto,
    CreateCardPurchaseInput,
  } from "../api/bindings";
  import {
    currentTime,
    normalizeTimeInput,
    parseAmountToCents,
    todayIso,
  } from "../format";
  import Modal from "./Modal.svelte";

  let {
    cards,
    categories,
    defaultCardId = "",
    saving,
    error,
    onsave,
    onclose,
  }: {
    cards: CardSummaryDto[];
    categories: CategoryDto[];
    defaultCardId?: string;
    saving: boolean;
    error: string | null;
    onsave: (input: CreateCardPurchaseInput) => void;
    onclose: () => void;
  } = $props();

  const initial = untrack(() => {
    const today = todayIso();
    return {
      cardId: defaultCardId || cards[0]?.id || "",
      occurredOn: today,
      firstDueOn: today,
      time: currentTime(),
      manualPeriod: "",
    };
  });

  let cardId = $state(initial.cardId);
  let amountText = $state("");
  let installmentCount = $state(1);
  let firstDueOn = $state(initial.firstDueOn);
  let occurredOn = $state(initial.occurredOn);
  let categoryId = $state("");
  let paymentMethod = $state("credit");
  let occurredTime = $state(initial.time);
  let manualPeriod = $state(initial.manualPeriod);
  let description = $state("");
  let localError = $state<string | null>(null);

  const categoryOptions = $derived(
    categories.filter(
      (category) => category.kind === "expense" && !category.archived_at,
    ),
  );

  function blurTime(): void {
    const normalized = normalizeTimeInput(occurredTime);
    if (normalized !== null) {
      occurredTime = normalized;
    }
  }

  function submit(): void {
    localError = null;

    const totalCents = parseAmountToCents(amountText);
    if (totalCents === null || totalCents <= 0) {
      localError = "Informe um valor maior que zero.";
      return;
    }
    if (!Number.isInteger(installmentCount) || installmentCount < 1) {
      localError = "A quantidade de parcelas deve começar em 1.";
      return;
    }
    if (installmentCount > 60) {
      localError = "O máximo é 60 parcelas.";
      return;
    }
    if (installmentCount > totalCents) {
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
      category_id: categoryId,
      total_cents: totalCents,
      installment_count: installmentCount,
      first_due_on: firstDueOn,
      occurred_on: occurredOn,
      occurred_time: normalizedTime || null,
      manual_period: normalizedTime ? null : manualPeriod,
      description: description.trim() || null,
      payment_method: paymentMethod,
    });
  }
</script>

<Modal title="Compra no cartão" {onclose}>
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
        <span>Data da compra</span>
        <input type="date" bind:value={occurredOn} required />
      </label>

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
        <span>Parcelas</span>
        <input
          type="number"
          min="1"
          max="60"
          bind:value={installmentCount}
          required
        />
      </label>

      <label class="field">
        <span>Primeira parcela</span>
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
          placeholder="Ex.: notebook"
        />
      </label>
    </div>

    <p class="form-hint">
      A compra aumenta a dívida total do cartão imediatamente. Cada parcela vira
      despesa na data de vencimento; o total não é despesa do mês da compra.
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
        {saving ? "Salvando…" : "Lançar compra"}
      </button>
    </footer>
  </form>
</Modal>
