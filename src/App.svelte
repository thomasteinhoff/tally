<script lang="ts">
  import { createQuery } from "@tanstack/svelte-query";

  import { getBootstrap } from "./lib/api";
  import { normalizeError } from "./lib/errors";
  import {
    isSectionActive,
    NAV_GROUPS,
    parseRoute,
    type Route,
  } from "./lib/router";
  import {
    setThemePreference,
    themeState,
  } from "./lib/theme.svelte";
  import { THEME_LABELS, THEME_PREFERENCES } from "./lib/theme";
  import { dismissToast, toastState } from "./lib/toast.svelte";
  import { requestNewMovement } from "./lib/ui.svelte";
  import BrandMark from "./lib/components/BrandMark.svelte";
  import AccountsPage from "./pages/AccountsPage.svelte";
  import AnalysisPage from "./pages/AnalysisPage.svelte";
  import CardDetailPage from "./pages/CardDetailPage.svelte";
  import CardsPage from "./pages/CardsPage.svelte";
  import CategoriesPage from "./pages/CategoriesPage.svelte";
  import InstallmentDetailPage from "./pages/InstallmentDetailPage.svelte";
  import InstallmentsPage from "./pages/InstallmentsPage.svelte";
  import MovementsPage from "./pages/MovementsPage.svelte";
  import OverviewPage from "./pages/OverviewPage.svelte";
  import SavingsDetailPage from "./pages/SavingsDetailPage.svelte";
  import SettingsPage from "./pages/SettingsPage.svelte";
  import StatementPage from "./pages/StatementPage.svelte";

  let route = $state<Route>(parseRoute(window.location.hash));

  $effect(() => {
    const onHashChange = () => {
      route = parseRoute(window.location.hash);
    };
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  });

  const bootstrapQuery = createQuery(() => ({
    queryKey: ["bootstrap"],
    queryFn: getBootstrap,
  }));

  const data = $derived(bootstrapQuery.data);
  const bootError = $derived(
    bootstrapQuery.error ? normalizeError(bootstrapQuery.error) : null,
  );
  const statusLabel = $derived(
    bootstrapQuery.isPending
      ? "Lendo banco local"
      : bootError
        ? "Falha de bootstrap"
        : data?.integrity.ok
          ? "Banco íntegro"
          : "Atenção",
  );

  const pageTitle = $derived(
    route.name === "overview"
      ? "Visão geral"
      : route.name === "analysis"
        ? "Quadro de análises"
        : route.name === "movements"
          ? "Movimentações"
          : route.name === "accounts"
            ? "Contas"
            : route.name === "statement"
              ? "Extrato"
              : route.name === "savings"
                ? "Cofrinho"
                : route.name === "cards"
                  ? "Cartões"
                  : route.name === "card"
                  ? "Cartão"
                  : route.name === "installments"
                    ? "Parcelamentos"
                    : route.name === "installment"
                      ? "Parcelamento"
                      : route.name === "categories"
                        ? "Categorias"
                        : route.name === "settings"
                          ? "Configurações"
                          : "Rota indisponível",
  );

  function openNewMovement(): void {
    if (route.name !== "movements") {
      window.location.hash = "#/movimentacoes";
    }
    requestNewMovement();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "n") {
      event.preventDefault();
      openNewMovement();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<svelte:head>
  <title>{pageTitle} · Tally</title>
  <meta
    name="description"
    content="Tally — instrumento de precisão financeira, local-first e offline."
  />
</svelte:head>

<div class="app-shell">
  <aside class="sidebar">
    <a class="brand" href="#/" aria-label="Tally — visão geral">
      <!-- Símbolo oficial centralizado em BrandMark / brand.ts. -->
      <BrandMark class="brand-mark" />
      <div>
        <span class="brand-name">Tally</span>
        <span class="brand-sub">precisão financeira</span>
      </div>
    </a>

    <nav class="nav" aria-label="Navegação principal">
      {#each NAV_GROUPS as group (group.label)}
        <div class="nav-group">
          <span class="nav-section">{group.label}</span>
          {#each group.items as item (item.section)}
            <a
              class="nav-item"
              class:is-active={isSectionActive(route, item.section)}
              aria-current={isSectionActive(route, item.section)
                ? "page"
                : undefined}
              href={item.href}
            >
              {item.label}
            </a>
          {/each}
        </div>
      {/each}
    </nav>

    <div class="sidebar-footer">
      <div class="status-line">
        <span
          class="status-dot"
          class:ok={Boolean(data?.integrity.ok)}
          class:error={Boolean(bootError)}
        ></span>
        <span>{statusLabel}</span>
      </div>
      <span class="sidebar-note">local-first · offline</span>
      <span class="sidebar-note">Ctrl+N · nova movimentação</span>
    </div>
  </aside>

  <main class="main">
    <header class="topbar">
      <div class="topbar-left">
        <h1>{pageTitle}</h1>
      </div>
      <div class="topbar-right">
        <span class="version">v{data?.app_version ?? "—"}</span>
        <div class="theme-switch" role="group" aria-label="Tema da interface">
          {#each THEME_PREFERENCES as preference (preference)}
            <button
              type="button"
              class="theme-switch-item"
              class:is-active={themeState.preference === preference}
              aria-pressed={themeState.preference === preference}
              aria-label={THEME_LABELS[preference]}
              title={THEME_LABELS[preference]}
              onclick={() => setThemePreference(preference)}
            >
              {#if preference === "light"}
                <svg
                  viewBox="0 0 16 16"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.4"
                  stroke-linecap="round"
                  aria-hidden="true"
                >
                  <circle cx="8" cy="8" r="3.1" />
                  <path
                    d="M8 1v1.6M8 13.4V15M1 8h1.6M13.4 8H15M3 3l1.1 1.1M11.9 11.9 13 13M13 3l-1.1 1.1M4.1 11.9 3 13"
                  />
                </svg>
              {:else if preference === "dark"}
                <svg
                  viewBox="0 0 16 16"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.4"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  aria-hidden="true"
                >
                  <path d="M13.4 9.7A5.7 5.7 0 0 1 6.3 2.6a5.9 5.9 0 1 0 7.1 7.1Z" />
                </svg>
              {:else}
                <svg
                  viewBox="0 0 16 16"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.4"
                  stroke-linecap="round"
                  aria-hidden="true"
                >
                  <rect x="1.6" y="2.6" width="12.8" height="8.4" rx="1.2" />
                  <path d="M5.6 13.6h4.8M8 11v2.6" />
                </svg>
              {/if}
            </button>
          {/each}
        </div>
        {#if route.name !== "movements"}
          <button
            type="button"
            class="button button--primary button--small"
            onclick={openNewMovement}
          >
            Nova movimentação
          </button>
        {/if}
      </div>
    </header>

    <div class="content">
      {#key route.name}
        <div class="page-transition">
          {#if route.name === "overview"}
            <OverviewPage />
          {:else if route.name === "analysis"}
            <AnalysisPage />
          {:else if route.name === "movements"}
            <MovementsPage />
          {:else if route.name === "accounts"}
            <AccountsPage />
          {:else if route.name === "statement"}
            <StatementPage accountId={route.accountId} />
          {:else if route.name === "savings"}
            <SavingsDetailPage accountId={route.accountId} />
          {:else if route.name === "cards"}
            <CardsPage />
          {:else if route.name === "card"}
            <CardDetailPage cardId={route.cardId} />
          {:else if route.name === "installments"}
            <InstallmentsPage />
          {:else if route.name === "installment"}
            <InstallmentDetailPage planId={route.planId} />
          {:else if route.name === "categories"}
            <CategoriesPage />
          {:else if route.name === "settings"}
            <SettingsPage />
          {:else}
            <section class="panel">
              <div class="state-panel">
                <span class="state-kicker">404</span>
                <h2>Rota não implementada.</h2>
                <p>
                  O caminho <span class="mono">{route.path}</span> não existe
                  nesta aplicação.
                </p>
                <div class="state-actions">
                  <a class="button button--primary" href="#/">
                    Voltar à visão geral
                  </a>
                </div>
              </div>
            </section>
          {/if}
        </div>
      {/key}
    </div>
  </main>
</div>

<div class="toast-region" aria-live="polite" aria-atomic="true">
  {#if toastState.current}
    <div
      class="toast"
      class:toast--warn={toastState.current.tone === "warn"}
      role={toastState.current.tone === "warn" ? "alert" : "status"}
    >
      <span>{toastState.current.message}</span>
      <button
        type="button"
        class="icon-button icon-button--small"
        aria-label="Fechar aviso"
        onclick={dismissToast}
      >
        ×
      </button>
    </div>
  {/if}
</div>
