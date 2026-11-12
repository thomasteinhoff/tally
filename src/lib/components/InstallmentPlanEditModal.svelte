<script lang="ts">
  import { untrack } from "svelte";

  import type {
    CategoryDto,
    InstallmentPlanDto,
    UpdateInstallmentPlanInput,
  } from "../api/bindings";
  import { formatAmountInput, parseAmountToCents } from "../format";
  import Modal from "./Modal.svelte";

  let {
    plan,
    categories,
    saving,
    error,
    onsave,
    onclose,
  }: {
    plan: InstallmentPlanDto;
    categories: CategoryDto[];
    saving: boolean;
    error: string | null;
    onsave: (input: UpdateInstallmentPlanInput) => void;
    onclose: () => void;
  } = $props();

  const initial = untrack(() => ({
    totalText: formatAmountInput(plan.total_cents),
    installmentCount: plan.installment_count,
    firstDueOn: plan.first_due_on,
    categoryId: plan.category_id,
    period: plan.period,
    description: plan.description ?? "",
  }));

  const locked = $derived(plan.paid_cents > 0);

  let totalText = $state(initial.totalText);
  let installmentCount = $state(initial.installmentCount);
  let firstDueOn = $state(initial.firstDueOn);
  let categoryId = $state(initial.categoryId);
  let period = $state(initial.period);
  let description = $state(initial.description);
  let localError = $state<string | null>(null);

  const categoryOptions = $derived(
    categories.filter(
      (category) =>
        category.kind === "expense" &&
        (!category.archived_at || category.id === plan.category_id),
    ),
  );

  function submit(): void {
    localError = null;

    if (locked) {
      onsave({
        category_id: plan.category_id,
        total_cents: plan.total_cents,
        installment_count: plan.installment_count,
        first_due_on: plan.first_due_on,
        period: plan.period,
        description: description.trim() || null,
      });
      return;
    }

    const totalCents = parseAmountToCents(totalText);
    if (totalCents === null || totalCents <= 0) {
      localError = "Informe um valor maior que zero.";
      return;
    }
    if (
      !Number.isInteger(installmentCount) ||
      installmentCount < 1 ||
      installmentCount > 60
    ) {
      localError = "A quantidade de parcelas deve ficar entre 1 e 60.";
      return;
    }
    if (installmentCount > totalCents) {
      localError = "Cada parcela precisa ter pelo menos um centavo.";
      return;
    }
    if (!firstDueOn) {
      localError = "Informe a data da primeira parcela.";
      return;
    }
    if (!categoryId) {
      localError = "Escolha a categoria da despesa.";
      return;
    }

    onsave({
      category_id: categoryId,
      total_cents: totalCents,
      installment_count: installmentCount,
      first_due_on: firstDueOn,
      period,
      description: description.trim() || null,
    });
  }
</script>

<Modal title="Editar parcelamento" {onclose}>
  <form
    class="form"
    onsubmit={(event) => {
      event.preventDefault();
      submit();
    }}
  >
    <div class="form-grid">
      <label class="field field--wide">
        <span>Valor total <small>use vírgula para centavos</small></span>
        <input
          class="money-input"
          inputmode="decimal"
          bind:value={totalText}
          disabled={locked}
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
          disabled={locked}
          required
        />
      </label>

      <label class="field">
        <span>Primeira parcela</span>
        <input
          type="date"
          bind:value={firstDueOn}
          disabled={locked}
          required
        />
      </label>

      <label class="field">
        <span>Categoria</span>
        <select bind:value={categoryId} disabled={locked} required>
          <option value="" disabled>Escolha a categoria</option>
          {#each categoryOptions as category (category.id)}
            <option value={category.id}>{category.name}</option>
          {/each}
        </select>
      </label>

      <label class="field">
        <span>Período</span>
        <select bind:value={period} disabled={locked}>
          <option value="madrugada">Madrugada</option>
          <option value="manha">Manhã</option>
          <option value="tarde">Tarde</option>
          <option value="noite">Noite</option>
          <option value="custom">Personalizado</option>
        </select>
      </label>

      <label class="field field--wide">
        <span>Descrição <small>opcional</small></span>
        <input bind:value={description} maxlength="500" placeholder="Ex.: notebook" />
      </label>
    </div>

    <p class="form-hint">
      {locked
        ? "Este parcelamento possui pagamentos: apenas a descrição pode ser alterada. Remova os pagamentos no cartão para recalcular as parcelas."
        : "Alterar o plano recalcula todas as parcelas e seus reconhecimentos de despesa. O total continua igual à soma das parcelas."}
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
        {saving ? "Salvando…" : "Salvar alterações"}
      </button>
    </footer>
  </form>
</Modal>
