<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    onclose,
    children,
  }: {
    title: string;
    onclose: () => void;
    children: Snippet;
  } = $props();

  let dialog = $state<HTMLElement>();
  let previousFocus: HTMLElement | null = null;

  $effect(() => {
    previousFocus =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    dialog?.querySelector<HTMLElement>("input, select, textarea")?.focus();

    return () => previousFocus?.focus();
  });
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      onclose();
    }
  }}
/>

<div
  class="modal-backdrop"
  role="presentation"
  onclick={(event) => {
    if (event.target === event.currentTarget) {
      onclose();
    }
  }}
>
  <div
    bind:this={dialog}
    class="modal"
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
  >
    <header class="modal-header">
      <h2>{title}</h2>
      <button
        type="button"
        class="icon-button"
        aria-label="Fechar"
        onclick={onclose}
      >
        ×
      </button>
    </header>

    <div class="modal-body">
      {@render children()}
    </div>
  </div>
</div>
