<script module lang="ts">
  let nextId = 0;
</script>

<script lang="ts">
  import type { CategoryDto } from "../api/bindings";
  import CategoryIcon from "./CategoryIcon.svelte";

  let {
    categories,
    value = $bindable(""),
    placeholder = "Escolha a categoria",
    required = false,
    disabled = false,
  }: {
    categories: CategoryDto[];
    value?: string;
    placeholder?: string;
    required?: boolean;
    disabled?: boolean;
  } = $props();

  const listId = `category-select-${nextId++}`;

  let open = $state(false);
  let activeIndex = $state(0);
  let root = $state<HTMLDivElement>();
  let listbox = $state<HTMLDivElement>();

  const selected = $derived(
    categories.find((category) => category.id === value) ?? null,
  );

  function optionId(index: number): string {
    return `${listId}-option-${index}`;
  }

  function show(): void {
    if (disabled || open) return;
    const index = categories.findIndex((category) => category.id === value);
    activeIndex = index < 0 ? 0 : index;
    open = true;
  }

  function hide(): void {
    open = false;
  }

  function choose(index: number): void {
    const category = categories[index];
    if (!category) return;
    value = category.id;
    hide();
  }

  function onKeydown(event: KeyboardEvent): void {
    if (disabled) return;

    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        if (!open) {
          show();
        } else if (categories.length > 0) {
          activeIndex = (activeIndex + 1) % categories.length;
        }
        break;
      case "ArrowUp":
        event.preventDefault();
        if (!open) {
          show();
        } else if (categories.length > 0) {
          activeIndex =
            (activeIndex - 1 + categories.length) % categories.length;
        }
        break;
      case "Home":
        if (!open) return;
        event.preventDefault();
        activeIndex = 0;
        break;
      case "End":
        if (!open) return;
        event.preventDefault();
        activeIndex = Math.max(0, categories.length - 1);
        break;
      case "Enter":
      case " ":
        event.preventDefault();
        if (open) choose(activeIndex);
        else show();
        break;
      case "Escape":
        if (!open) return;
        event.preventDefault();
        // Não deixa o Escape fechar o modal enquanto a lista está aberta.
        event.stopPropagation();
        hide();
        break;
      case "Tab":
        hide();
        break;
    }
  }

  $effect(() => {
    if (!open || !listbox) return;
    // Opcional porque jsdom não implementa scrollIntoView.
    listbox.children[activeIndex]?.scrollIntoView?.({ block: "nearest" });
  });
</script>

<svelte:window
  onpointerdown={(event) => {
    if (open && root && !root.contains(event.target as Node)) hide();
  }}
/>

<div class="category-select" bind:this={root}>
  <button
    type="button"
    class="category-select-trigger"
    role="combobox"
    aria-label="Categoria"
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={listId}
    aria-activedescendant={open && categories.length > 0
      ? optionId(activeIndex)
      : undefined}
    aria-required={required || undefined}
    {disabled}
    onclick={() => (open ? hide() : show())}
    onkeydown={onKeydown}
  >
    {#if selected}
      <span class="category-select-value">
        <CategoryIcon icon={selected.icon} />
        {selected.name}
      </span>
    {:else}
      <span class="category-select-placeholder">{placeholder}</span>
    {/if}
    <svg
      class="category-select-caret"
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <path d="M6 9l6 6 6-6" />
    </svg>
  </button>

  {#if open}
    <div
      class="category-select-menu"
      role="listbox"
      id={listId}
      aria-label="Categorias"
      bind:this={listbox}
    >
      {#if categories.length === 0}
        <div class="category-select-empty">Nenhuma categoria disponível.</div>
      {:else}
        {#each categories as category, index (category.id)}
          <button
            type="button"
            class="category-select-option"
            class:is-active={index === activeIndex}
            class:is-selected={category.id === value}
            role="option"
            id={optionId(index)}
            aria-selected={category.id === value}
            tabindex="-1"
            onclick={() => choose(index)}
            onmousemove={() => (activeIndex = index)}
          >
            <CategoryIcon icon={category.icon} />
            <span>{category.name}</span>
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>
