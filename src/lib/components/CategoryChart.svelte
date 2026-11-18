<script lang="ts">
  import { formatCents } from "../format";
  import {
    formatPercent,
    type CategoryBarDatum,
  } from "../category-chart";
  import CategoryIcon from "./CategoryIcon.svelte";

  let {
    bars,
    totalCents,
    entryCount,
    tone = "expense",
    mode = "bars",
    icons = {},
  }: {
    bars: CategoryBarDatum[];
    totalCents: number;
    entryCount: number;
    tone?: "expense" | "income";
    mode?: "bars" | "donut";
    /** Mapa category_id → chave de ícone. */
    icons?: Record<string, string>;
  } = $props();

  const slices = $derived.by(() => {
    let offset = 0;
    return bars.map((bar, index) => {
      const slice = { ...bar, index, offset };
      offset += bar.percent;
      return slice;
    });
  });

  function iconFor(bar: CategoryBarDatum): string {
    return icons[bar.key] ?? "generic";
  }
</script>

<div class="category-chart">
  <div class="chart-summary">
    <span class="mono">{formatCents(totalCents)}</span>
    <span>
      {entryCount}
      {entryCount === 1 ? "lançamento" : "lançamentos"}
    </span>
  </div>

  {#if mode === "donut"}
    <div class="donut-layout">
      <svg
        class="donut"
        viewBox="0 0 42 42"
        role="img"
        aria-label="Distribuição das categorias por valor"
      >
        <circle class="donut-track" cx="21" cy="21" r="15.9155" />
        <g transform="rotate(-90 21 21)">
          {#each slices as slice (slice.key)}
            <circle
              class="donut-slice {slice.grouped
                ? 'donut-slice--other'
                : `donut-slice--${slice.index % 8}`}"
              cx="21"
              cy="21"
              r="15.9155"
              pathLength="100"
              stroke-dasharray="{slice.percent} {Math.max(0, 100 - slice.percent)}"
              stroke-dashoffset={-slice.offset}
            />
          {/each}
        </g>
      </svg>

      <ul class="donut-legend">
        {#each slices as slice (slice.key)}
          <li>
            <span
              class="legend-swatch {slice.grouped
                ? 'legend-swatch--other'
                : `legend-swatch--${slice.index % 8}`}"
              aria-hidden="true"
            ></span>
            <CategoryIcon icon={iconFor(slice)} size={16} />
            <span class="legend-label" title={slice.label}>{slice.label}</span>
            <span class="legend-value mono">{formatCents(slice.totalCents)}</span>
            <span class="legend-percent mono">{formatPercent(slice.percent)}</span>
          </li>
        {/each}
      </ul>
    </div>
  {:else}
    <ol class="category-bars">
      {#each bars as bar (bar.key)}
        <li>
          <div class="bar-head">
            <span class="bar-label" title={bar.label}>
              <CategoryIcon icon={iconFor(bar)} size={16} />
              <span class="bar-label-text">{bar.label}</span>
            </span>
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
  {/if}
</div>
