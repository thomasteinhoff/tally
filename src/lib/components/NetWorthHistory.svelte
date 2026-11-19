<script lang="ts">
  import type { NetWorthHistoryDto } from "../api/bindings";
  import { formatCents, formatMonth } from "../format";

  let { history }: { history: NetWorthHistoryDto } = $props();

  const points = $derived(history.points);
  const latest = $derived(points[points.length - 1]);
  const values = $derived(points.map((point) => point.net_worth_cents));
  const min = $derived(Math.min(...values));
  const max = $derived(Math.max(...values));
  const span = $derived(max - min);

  const coordinates = $derived.by(() =>
    points.map((point, index) => {
      const x = points.length > 1 ? (index / (points.length - 1)) * 96 + 2 : 50;
      const ratio =
        span > 0 ? (point.net_worth_cents - min) / span : 0.5;
      return { ...point, x, y: 30 - ratio * 26 };
    }),
  );
  const line = $derived(
    coordinates.map((point) => `${point.x},${point.y}`).join(" "),
  );
</script>

<div class="history">
  <div class="history-head">
    <span class="mono history-latest">
      {formatCents(latest?.net_worth_cents ?? 0)}
    </span>
    <span>fechamento de {latest ? formatMonth(latest.month) : "—"}</span>
  </div>

  <div class="history-plot">
    <svg
      class="history-chart"
      viewBox="0 0 100 36"
      preserveAspectRatio="none"
      role="img"
      aria-label="Evolução do patrimônio líquido nos seis meses da série"
    >
      <line class="history-axis" x1="0" y1="30" x2="100" y2="30"></line>
      <polyline class="history-line" points={line}></polyline>
    </svg>

    <div class="history-dots" aria-hidden="true">
      {#each coordinates as point (point.month)}
        <span
          class="history-dot"
          class:is-current={point.month === latest?.month}
          style:left={`${point.x}%`}
          style:top={`${(point.y / 36) * 100}%`}
          title={`${formatMonth(point.month)} · ${formatCents(point.net_worth_cents)}`}
        ></span>
      {/each}
    </div>
  </div>

  <ol class="history-labels">
    {#each points as point (point.month)}
      <li class:is-current={point.month === latest?.month}>
        <span>{formatMonth(point.month)}</span>
        <span class="mono">{formatCents(point.net_worth_cents)}</span>
      </li>
    {/each}
  </ol>
</div>
