<script lang="ts">
  import type { PeriodSpendingReportDto } from "../api/bindings";
  import { formatPercent } from "../category-chart";
  import { formatCents, formatDays, periodRangeLabel } from "../format";

  let { report }: { report: PeriodSpendingReportDto } = $props();

  let mode = $state<"accumulated" | "average">("accumulated");

  function share(totalCents: number): number {
    return report.total_cents > 0
      ? (totalCents / report.total_cents) * 100
      : 0;
  }

  function width(percent: number): string {
    return `${Math.min(100, Math.max(0, percent))}%`;
  }
</script>

<div class="period-card">
  <div
    class="chart-toggle"
    role="group"
    aria-label="Métrica dos gastos por período do dia"
  >
    <button
      type="button"
      class:is-active={mode === "accumulated"}
      aria-pressed={mode === "accumulated"}
      onclick={() => (mode = "accumulated")}
    >
      Acumulado
    </button>
    <button
      type="button"
      class:is-active={mode === "average"}
      aria-pressed={mode === "average"}
      onclick={() => (mode = "average")}
    >
      Média diária
    </button>
  </div>

  <p class="period-explainer">
    {#if mode === "accumulated"}
      Participação de cada faixa sobre o total de despesas do mês.
    {:else}
      Média por dia ativo na faixa e frequência sobre os
      {report.days_with_expense}
      {report.days_with_expense === 1 ? "dia" : "dias"} com despesa no mês. A
      frequência pode passar de 100% porque um dia pode ter mais de uma faixa.
    {/if}
  </p>

  <ol class="period-list">
    {#each report.buckets as bucket (bucket.period)}
      <li>
        <div class="bar-head">
          <span class="bar-label">{periodRangeLabel(bucket.period)}</span>
          <span class="bar-values">
            {#if mode === "accumulated"}
              <span class="mono">{formatCents(bucket.total_cents)}</span>
              <span class="bar-percent mono">
                {formatPercent(share(bucket.total_cents))}
              </span>
            {:else}
              <span class="mono">{formatCents(bucket.average_cents)}/dia</span>
              <span class="bar-percent mono">
                {formatPercent(bucket.frequency_percent)}
              </span>
            {/if}
          </span>
        </div>
        <div class="bar-track" aria-hidden="true">
          <span
            class="bar-fill bar-fill--expense"
            style:width={width(
              mode === "accumulated"
                ? share(bucket.total_cents)
                : bucket.frequency_percent,
            )}
          ></span>
        </div>
        <small class="period-note">
          {bucket.entry_count}
          {bucket.entry_count === 1 ? "lançamento" : "lançamentos"} ·
          {formatDays(bucket.active_days)}
        </small>
      </li>
    {/each}
  </ol>

  {#if report.unclassified.entry_count > 0}
    <div class="period-unclassified">
      <span class="data-label">Sem período identificado</span>
      <p>
        {formatCents(report.unclassified.total_cents)} ·
        {report.unclassified.entry_count}
        {report.unclassified.entry_count === 1 ? "lançamento" : "lançamentos"}
        {#if mode === "average"}
          · {formatDays(report.unclassified.active_days)} · média de
          {formatCents(report.unclassified.average_cents)}/dia
        {/if}
      </p>
      <small>
        Período manual ou não informado; não é redistribuído entre as faixas.
      </small>
    </div>
  {/if}
</div>
