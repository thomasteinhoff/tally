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
  let backdropPressed = $state(false);

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
  onmousedown={(event) => {
    backdropPressed = event.target === event.currentTarget;
  }}
  onclick={(event) => {
    // Só fecha quando o clique começou e terminou no fundo. Selecionar texto
    // dentro do formulário e arrastar para fora não deve fechar o modal.
    if (backdropPressed && event.target === event.currentTarget) {
      onclose();
    }
    backdropPressed = false;
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
