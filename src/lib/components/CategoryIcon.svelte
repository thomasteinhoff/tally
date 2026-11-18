<script lang="ts">
  import { categoryIconPath } from "../category-icons";

  let {
    icon = null,
    size = 18,
    class: className = "",
  }: {
    icon?: string | null;
    size?: number;
    class?: string;
  } = $props();
</script>

<svg
  class={className}
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={categoryIconPath(icon)} />
</svg>
