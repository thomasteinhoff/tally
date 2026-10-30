<script lang="ts">
  import { createMutation, createQuery } from "@tanstack/svelte-query";

  import { getBootstrap, runIntegrityCheck } from "./lib/api";
  import { normalizeError } from "./lib/errors";

  const bootstrapQuery = createQuery(() => ({
    queryKey: ["bootstrap"],
    queryFn: getBootstrap,
  }));

  const integrityMutation = createMutation(() => ({
    mutationFn: runIntegrityCheck,
  }));

  const data = $derived(bootstrapQuery.data);
  const bootError = $derived(
    bootstrapQuery.error ? normalizeError(bootstrapQuery.error) : null,
  );
  const integrityError = $derived(
    integrityMutation.error ? normalizeError(integrityMutation.error) : null,
  );
  const integrity = $derived(
    integrityMutation.data ?? data?.integrity ?? null,
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
</script>

<svelte:head>
  <meta
    name="description"
    content="Tally — instrumento de precisão financeira, local-first e offline."
  />
</svelte:head>

<div class="app-shell">
  <aside class="sidebar">
    <div class="brand">
      <span class="brand-mark" aria-hidden="true"></span>
      <div>
        <span class="brand-name">Tally</span>
        <span class="brand-sub">precisão financeira</span>
      </div>
    </div>

    <nav class="nav" aria-label="Navegação principal">
      <div class="nav-group">
        <span class="nav-section">Operação</span>
        <span class="nav-item is-active" aria-current="page">Fundação</span>
        <span class="nav-item is-disabled" aria-disabled="true"
          >Visão geral</span
        >
        <span class="nav-item is-disabled" aria-disabled="true"
          >Movimentações</span
        >
        <span class="nav-item is-disabled" aria-disabled="true">Contas</span>
        <span class="nav-item is-disabled" aria-disabled="true">Cartões</span>
      </div>

      <div class="nav-group">
        <span class="nav-section">Sistema</span>
        <span class="nav-item is-disabled" aria-disabled="true"
          >Categorias</span
        >
        <span class="nav-item is-disabled" aria-disabled="true"
          >Relatórios</span
        >
        <span class="nav-item is-disabled" aria-disabled="true"
          >Configurações</span
        >
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
      <span class="sidebar-note">sem nuvem · sem login</span>
    </div>
  </aside>

  <main class="main">
    <header class="topbar">
      <div class="topbar-left">
        <span class="eyebrow">Fase 0 / Fundação</span>
        <h1>Base operacional</h1>
      </div>
      <div class="topbar-right">
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
      {#if bootstrapQuery.isPending}
        <section
          class="state-panel"
          aria-busy="true"
          aria-labelledby="loading-title"
        >
          <span class="state-kicker">Bootstrap</span>
          <h2 id="loading-title">Lendo o banco local.</h2>
          <p>
            Resolvendo caminhos, abrindo o SQLite e conferindo o schema real. O
            resultado só aparece depois da confirmação do Rust.
          </p>
          <div class="loading-rule" aria-hidden="true"></div>
        </section>
      {:else if bootError}
        <section
          class="state-panel state-panel--error"
          role="alert"
          aria-labelledby="boot-error-title"
        >
          <span class="state-kicker">Falha de bootstrap</span>
          <h2 id="boot-error-title">O banco local não respondeu.</h2>
          <p>{bootError?.message}</p>
          <div class="error-meta">
            <span class="badge badge--warn">{bootError?.code}</span>
            <span class="mono">nenhum estado simulado foi exibido</span>
          </div>
          {#if bootError?.details}
            <details class="details">
              <summary>Detalhes técnicos</summary>
              <pre>{bootError.details}</pre>
            </details>
          {/if}
          <div class="state-actions">
            <button
              type="button"
              class="button button--primary"
              onclick={() => bootstrapQuery.refetch()}
              disabled={bootstrapQuery.isFetching}
            >
              {bootstrapQuery.isFetching ? "Tentando…" : "Tentar novamente"}
            </button>
          </div>
        </section>
      {:else if data}
        <section class="intro" aria-labelledby="intro-title">
          <div>
            <h2 id="intro-title">A base está<br />no lugar.</h2>
            <p>
              O Tally abriu o banco local, aplicou o schema e leu o estado real
              pelo IPC. Nada nesta tela é simulado: versão, schema, contas de
              sistema e integridade vêm do SQLite em
              <span class="mono">%LOCALAPPDATA%\Tally</span>.
            </p>
          </div>
          <div class="intro-meta" aria-label="Princípios técnicos">
            <span>Rust / Tauri</span>
            <span>SQLite / WAL</span>
            <span>Local / Offline</span>
          </div>
        </section>

        <section class="panel" aria-labelledby="bootstrap-title">
          <header class="panel-header">
            <div class="panel-heading">
              <span class="panel-kicker">01</span>
              <h3 class="panel-title" id="bootstrap-title">
                Estado do bootstrap
              </h3>
            </div>
            <span class="panel-meta">leitura direta via IPC</span>
          </header>

          <div class="data-grid">
            <div class="data-cell">
              <div class="data-label">Aplicação</div>
              <div class="data-value">{data.app_version}</div>
              <div class="data-note">Tauri 2 · Windows</div>
            </div>

            <div class="data-cell">
              <div class="data-label">Schema</div>
              <div class="data-value">{data.schema_version}</div>
              <div class="data-note">user_version</div>
            </div>

            <div class="data-cell">
              <div class="data-label">Contas de sistema</div>
              <div class="data-value">{data.system_accounts}</div>
              <div class="data-note">
                patrimônio · receita · despesa · diferido
              </div>
            </div>

            <div class="data-cell">
              <div class="data-label">Integridade</div>
              <div
                class="data-value"
                class:data-value--ok={data.integrity.ok}
                class:data-value--warn={!data.integrity.ok}
              >
                {data.integrity.ok ? "OK" : "Atenção"}
              </div>
              <div class="data-note">PRAGMA integrity_check</div>
            </div>

            <div class="data-cell data-cell--wide">
              <div class="data-label">Arquivo do banco</div>
              <div class="data-value data-value--text">{data.db_path}</div>
            </div>

            <div class="data-cell data-cell--wide">
              <div class="data-label">Diretório de dados</div>
              <div class="data-value data-value--text">{data.data_dir}</div>
            </div>
          </div>

          <footer class="panel-footer">
            <div class="panel-footnote">
              <span class="status-dot" class:ok={Boolean(integrity?.ok)}></span>
              <span>
                {integrity
                  ? integrity.ok
                    ? "integrity_check sem inconsistências"
                    : "integrity_check encontrou inconsistências"
                  : "verificação indisponível"}
              </span>
              {#if data.settings.first_run_completed}
                <span class="mono">· primeira execução concluída</span>
              {:else}
                <span class="mono">· primeira execução pendente</span>
              {/if}
            </div>

            <div class="actions">
              <button
                type="button"
                class="button"
                onclick={() => integrityMutation.mutate()}
                disabled={integrityMutation.isPending}
              >
                {integrityMutation.isPending
                  ? "Verificando…"
                  : "Verificar integridade"}
              </button>
              <button
                type="button"
                class="button button--primary"
                onclick={() => bootstrapQuery.refetch()}
                disabled={bootstrapQuery.isFetching}
              >
                {bootstrapQuery.isFetching ? "Atualizando…" : "Atualizar"}
              </button>
            </div>
          </footer>

          {#if integrity && !integrity.ok}
            <div class="inline-result inline-result--warn" role="alert">
              <span class="mono">integrity_check</span>
              <span>{integrity.issues.join(" · ")}</span>
            </div>
          {:else if integrityMutation.isSuccess}
            <div class="inline-result inline-result--ok" role="status">
              <span class="mono">integrity_check</span>
              <span>ok · nenhuma inconsistência encontrada</span>
            </div>
          {/if}

          {#if integrityError}
            <div class="inline-result inline-result--warn" role="alert">
              <span class="mono">{integrityError.code}</span>
              <span>
                {integrityError.message}{integrityError.details
                  ? ` — ${integrityError.details}`
                  : ""}
              </span>
            </div>
          {/if}
        </section>

        <section class="panel" aria-labelledby="phase-title">
          <header class="panel-header">
            <div class="panel-heading">
              <span class="panel-kicker">02</span>
              <h3 class="panel-title" id="phase-title">Fronteira da fase</h3>
            </div>
            <span class="panel-meta">sem funcionalidade antecipada</span>
          </header>

          <div class="phase-grid">
            <div class="phase-block">
              <h3>Entregue nesta fundação</h3>
              <ul class="phase-list">
                <li>
                  Workspace Rust com domínio, aplicação e infraestrutura
                  separados.
                </li>
                <li>
                  SQLite local com migração, WAL, foreign keys e triggers de
                  integridade.
                </li>
                <li>Bootstrap real via Tauri command, sem estado simulado.</li>
                <li>
                  Shell Svelte com tipografia tabular e tokens de precisão.
                </li>
              </ul>
            </div>

            <div class="phase-block">
              <h3>Próxima fase</h3>
              <p>
                O núcleo do ledger: contas, categorias, lançamentos em partidas
                dobradas e saldos derivados. Nenhum atalho de UI será usado para
                substituir a regra financeira no Rust.
              </p>
            </div>
          </div>

          <footer class="panel-footer">
            <div class="panel-footnote">
              <span class="mono">retenção de backups</span>
              <span>{data.settings.backup_retention} cópias</span>
            </div>
            <span class="panel-meta">Fase 0 de 4</span>
          </footer>
        </section>
      {/if}
    </div>
  </main>
</div>
