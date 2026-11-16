<script lang="ts">
  import { formatCents } from "../format";
  import {
    formatPercent,
    type CategoryBarDatum,
  } from "../category-chart";

  let {
    bars,
    totalCents,
    entryCount,
    tone = "expense",
  }: {
    bars: CategoryBarDatum[];
    totalCents: number;
    entryCount: number;
    tone?: "expense" | "income";
  } = $props();
</script>

<div class="category-chart">
  <div class="chart-summary">
    <span class="mono">{formatCents(totalCents)}</span>
    <span>
      {entryCount}
      {entryCount === 1 ? "lançamento" : "lançamentos"}
    </span>
  </div>

  <ol class="category-bars">
    {#each bars as bar (bar.key)}
      <li>
        <div class="bar-head">
          <span class="bar-label" title={bar.label}>{bar.label}</span>
          <span class="bar-values">
            <span class="mono">{formatCents(bar.totalCents)}</span>
            <span class="bar-percent mono">{formatPercent(bar.percent)}</span>
          </span>
        </div>
        <div class="bar-track" aria-hidden="true">
          <span class="bar-fill bar-fill--{tone}" style:width="{bar.percent}%"
          ></span>
        </div>
      </li>
    {/each}
  </ol>
</div>
