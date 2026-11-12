<script lang="ts">
  import { createQuery } from "@tanstack/svelte-query";

  import { getBootstrap } from "./lib/api";
  import { normalizeError } from "./lib/errors";
  import { parseRoute, type Route } from "./lib/router";
  import { dismissToast, toastState } from "./lib/toast.svelte";
  import { requestNewMovement } from "./lib/ui.svelte";
  import AboutPage from "./pages/AboutPage.svelte";
  import AccountsPage from "./pages/AccountsPage.svelte";
  import CardDetailPage from "./pages/CardDetailPage.svelte";
  import CardsPage from "./pages/CardsPage.svelte";
  import CategoriesPage from "./pages/CategoriesPage.svelte";
  import InstallmentDetailPage from "./pages/InstallmentDetailPage.svelte";
  import InstallmentsPage from "./pages/InstallmentsPage.svelte";
  import MovementsPage from "./pages/MovementsPage.svelte";
  import OverviewPage from "./pages/OverviewPage.svelte";
  import ReportsPage from "./pages/ReportsPage.svelte";
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
      : route.name === "movements"
        ? "Movimentações"
        : route.name === "accounts"
          ? "Contas"
          : route.name === "statement"
            ? "Extrato"
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
                      : route.name === "reports"
                        ? "Relatórios"
                        : route.name === "settings"
                          ? "Configurações"
                          : route.name === "about"
                            ? "Sobre"
                            : "Rota indisponível",
  );

  function active(section: string): boolean {
    if (section === "accounts") {
      return (
        route.name === "accounts" ||
        route.name === "statement" ||
        route.name === "cards" ||
        route.name === "card"
      );
    }
    if (section === "installments") {
      return route.name === "installments" || route.name === "installment";
    }
    return route.name === section;
  }

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
      <span class="brand-mark" aria-hidden="true"></span>
      <div>
        <span class="brand-name">Tally</span>
        <span class="brand-sub">precisão financeira</span>
      </div>
    </a>

    <nav class="nav" aria-label="Navegação principal">
      <div class="nav-group">
        <span class="nav-section">Operação</span>
        <a
          class="nav-item"
          class:is-active={active("overview")}
          aria-current={active("overview") ? "page" : undefined}
          href="#/"
        >
          Visão geral
        </a>
        <a
          class="nav-item"
          class:is-active={active("movements")}
          aria-current={active("movements") ? "page" : undefined}
          href="#/movimentacoes"
        >
          Movimentações
        </a>
        <a
          class="nav-item"
          class:is-active={active("accounts")}
          aria-current={active("accounts") ? "page" : undefined}
          href="#/contas"
        >
          Contas e cartões
        </a>
        <a
          class="nav-item"
          class:is-active={active("installments")}
          aria-current={active("installments") ? "page" : undefined}
          href="#/parcelamentos"
        >
          Parcelamentos
        </a>
      </div>

      <div class="nav-group">
        <span class="nav-section">Sistema</span>
        <a
          class="nav-item"
          class:is-active={active("categories")}
          aria-current={active("categories") ? "page" : undefined}
          href="#/categorias"
        >
          Categorias
        </a>
        <a
          class="nav-item"
          class:is-active={active("reports")}
          aria-current={active("reports") ? "page" : undefined}
          href="#/relatorios"
        >
          Relatórios
        </a>
        <a
          class="nav-item"
          class:is-active={active("settings")}
          aria-current={active("settings") ? "page" : undefined}
          href="#/configuracoes"
        >
          Configurações
        </a>
        <a
          class="nav-item"
          class:is-active={active("about")}
          aria-current={active("about") ? "page" : undefined}
          href="#/sobre"
        >
          Sobre
        </a>
      </div>
    </nav>

    <div class="sidebar-footer">
      <span class="sidebar-note">local-first · offline</span>
      <div class="status-line">
        <span
          class="status-dot"
          class:ok={Boolean(data?.integrity.ok)}
          class:error={Boolean(bootError)}
        ></span>
        <span>{statusLabel}</span>
      </div>
      <span class="sidebar-note">Ctrl+N · nova movimentação</span>
    </div>
  </aside>

  <main class="main">
    <header class="topbar">
      <div class="topbar-left">
        <span class="eyebrow">Tally / Phase 4</span>
        <h1>{pageTitle}</h1>
      </div>
      <div class="topbar-right">
        {#if route.name !== "movements"}
          <button
            type="button"
            class="button button--primary button--small"
            onclick={openNewMovement}
          >
            Nova movimentação
          </button>
        {/if}
        <span class="version">v{data?.app_version ?? "—"}</span>
        <span
          class="badge"
          class:badge--ok={Boolean(data?.integrity.ok)}
          class:badge--warn={Boolean(data && !data.integrity.ok)}
        >
          {data
            ? data.integrity.ok
              ? "SQLite ok"
              : "SQLite atenção"
            : "aguardando"}
        </span>
      </div>
    </header>

    <div class="content">
      {#if route.name === "overview"}
        <OverviewPage />
      {:else if route.name === "movements"}
        <MovementsPage />
      {:else if route.name === "accounts"}
        <AccountsPage />
      {:else if route.name === "statement"}
        <StatementPage accountId={route.accountId} />
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
      {:else if route.name === "reports"}
        <ReportsPage />
      {:else if route.name === "settings"}
        <SettingsPage />
      {:else if route.name === "about"}
        <AboutPage />
      {:else}
        <section class="panel">
          <div class="state-panel">
            <span class="state-kicker">404</span>
            <h2>Rota não implementada.</h2>
            <p>
              O caminho <span class="mono">{route.path}</span> não existe nesta
              aplicação.
            </p>
            <div class="state-actions">
              <a class="button button--primary" href="#/">Voltar à visão geral</a>
            </div>
          </div>
        </section>
      {/if}
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
