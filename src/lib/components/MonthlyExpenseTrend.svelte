<script lang="ts">
  import type { ExpenseTrendDto } from "../api/bindings";
  import { formatCents, formatMonth } from "../format";

  let {
    trend,
    selectedMonth,
  }: { trend: ExpenseTrendDto; selectedMonth: string } = $props();

  const max = $derived(
    Math.max(1, ...trend.months.map((month) => month.total_cents)),
  );

  function width(totalCents: number): string {
    return `${(totalCents / max) * 100}%`;
  }
</script>

<div class="trend">
  <ol
    class="trend-list"
    aria-label="Despesas nos seis meses terminando no mês selecionado"
  >
    {#each trend.months as month (month.month)}
      <li class:is-current={month.month === selectedMonth}>
        <div class="bar-head">
          <span class="bar-label">{formatMonth(month.month)}</span>
          <span class="bar-values">
            <span class="mono">{formatCents(month.total_cents)}</span>
          </span>
        </div>
        <div class="bar-track" aria-hidden="true">
          <span
            class="bar-fill bar-fill--expense"
            style:width={width(month.total_cents)}
          ></span>
        </div>
      </li>
    {/each}
  </ol>
</div>
