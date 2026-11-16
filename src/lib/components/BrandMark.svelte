<script lang="ts">
  import {
    BRAND_MARK_ACCENT,
    BRAND_MARK_STONE,
    BRAND_MARK_VIEWBOX,
  } from "../brand";

  let { class: className = "" }: { class?: string } = $props();
</script>

<!-- Símbolo oficial. Único ponto de desenho: trocar o ícone em brand.ts. -->
<svg
  class={className}
  viewBox={BRAND_MARK_VIEWBOX}
  role="img"
  aria-hidden="true"
  focusable="false"
  shape-rendering="geometricPrecision"
>
  {#each BRAND_MARK_STONE as bar}
    <rect
      x={bar.x}
      y={bar.y}
      width={bar.width}
      height={bar.height}
      fill="currentColor"
    />
  {/each}
  {#each BRAND_MARK_ACCENT as bar}
    <rect
      x={bar.x}
      y={bar.y}
      width={bar.width}
      height={bar.height}
      fill="var(--brand-terracotta)"
    />
  {/each}
</svg>
