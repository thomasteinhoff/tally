<script lang="ts">
  import { createQuery } from "@tanstack/svelte-query";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import { getBootstrap, openAppFolder } from "../lib/api";
  import type { AppErrorDto } from "../lib/api/bindings";
  import { normalizeError } from "../lib/errors";

  const bootstrapQuery = createQuery(() => ({
    queryKey: ["bootstrap"],
    queryFn: getBootstrap,
  }));

  const data = $derived(bootstrapQuery.data);
  const bootError = $derived(
    bootstrapQuery.error ? normalizeError(bootstrapQuery.error) : null,
  );

  let folderError = $state<AppErrorDto | null>(null);
  let opening = $state<"data" | "logs" | null>(null);

  async function openFolder(folder: "data" | "logs"): Promise<void> {
    folderError = null;
    opening = folder;
    try {
      await openAppFolder(folder);
    } catch (error) {
      folderError = normalizeError(error);
    } finally {
      opening = null;
    }
  }

  const repositoryUrl = "https://github.com/thomasteinhoff/tally";
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Sistema</span>
    <h2>Sobre</h2>
    <p>
      Instrumento de precisão financeira, local-first e offline. Seus dados
      vivem em um único arquivo SQLite no seu computador.
    </p>
  </div>
  <div class="page-actions">
    <button
      type="button"
      class="button"
      onclick={() => void openUrl(repositoryUrl)}
    >
      Repositório
    </button>
    <a class="button button--primary" href="#/">Visão geral</a>
  </div>
</section>

{#if bootstrapQuery.isPending}
  <section class="panel" aria-busy="true">
    <div class="state-panel">
      <span class="state-kicker">Lendo</span>
      <h2>Consultando o aplicativo.</h2>
      <div class="loading-rule" aria-hidden="true"></div>
    </div>
  </section>
{:else if bootError}
  <section class="panel">
    <div class="state-panel state-panel--error" role="alert">
      <span class="state-kicker">{bootError.code}</span>
      <h2>Não foi possível ler o diagnóstico.</h2>
      <p>{bootError.message}</p>
      <div class="state-actions">
        <button
          type="button"
          class="button button--primary"
          onclick={() => bootstrapQuery.refetch()}
        >
          Tentar novamente
        </button>
      </div>
    </div>
  </section>
{:else if data}
  <section class="panel" aria-labelledby="about-app-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">01</span>
        <h3 class="panel-title" id="about-app-title">Aplicação</h3>
      </div>
      <span class="badge badge--ok">v{data.app_version}</span>
    </header>

    <div class="data-grid data-grid--auto">
      <div class="data-cell">
        <div class="data-label">Versão</div>
        <div class="data-value">{data.app_version}</div>
        <div class="data-note">Windows · Tauri</div>
      </div>
      <div class="data-cell">
        <div class="data-label">Schema</div>
        <div class="data-value">{data.schema_version}</div>
        <div class="data-note">user_version</div>
      </div>
      <div class="data-cell">
        <div class="data-label">Contas de sistema</div>
        <div class="data-value">{data.system_accounts}</div>
        <div class="data-note">patrimônio · receita · despesa · diferido</div>
      </div>
    </div>

    <footer class="panel-footer">
      <div class="panel-footnote">
        Rust + SQLite no backend, Svelte no frontend. Nenhum número é calculado
        no navegador.
      </div>
      <a class="text-link" href="#/configuracoes">Ver diagnóstico completo</a>
    </footer>
  </section>

  <section class="panel" aria-labelledby="about-data-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">02</span>
        <h3 class="panel-title" id="about-data-title">Dados e logs</h3>
      </div>
      <span class="panel-meta">local-first</span>
    </header>

    <div class="data-grid">
      <div class="data-cell data-cell--wide">
        <div class="data-label">Arquivo do banco</div>
        <div class="data-value data-value--text">{data.db_path}</div>
      </div>
      <div class="data-cell data-cell--wide">
        <div class="data-label">Diretório de dados</div>
        <div class="data-value data-value--text">{data.data_dir}</div>
        <div class="data-note">
          Backups, exportações e logs ficam neste diretório.
        </div>
      </div>
    </div>

    <footer class="panel-footer">
      <div class="panel-footnote">
        Atualizar ou desinstalar o Tally não remove estes arquivos. Para apagar
        os dados, remova a pasta manualmente.
      </div>
      <div class="actions">
        <button
          type="button"
          class="button"
          onclick={() => void openFolder("logs")}
          disabled={opening !== null}
        >
          {opening === "logs" ? "Abrindo…" : "Abrir logs"}
        </button>
        <button
          type="button"
          class="button button--primary"
          onclick={() => void openFolder("data")}
          disabled={opening !== null}
        >
          {opening === "data" ? "Abrindo…" : "Abrir pasta de dados"}
        </button>
      </div>
    </footer>

    {#if folderError}
      <div class="inline-result inline-result--warn" role="alert">
        <span class="mono">{folderError.code}</span>
        <span>{folderError.message}</span>
      </div>
    {/if}
  </section>

  <section class="panel" aria-labelledby="about-keys-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">03</span>
        <h3 class="panel-title" id="about-keys-title">Atalhos</h3>
      </div>
      <span class="panel-meta">teclado</span>
    </header>

    <div class="data-grid data-grid--auto">
      <div class="data-cell">
        <div class="data-label">Ctrl+N</div>
        <div class="data-value data-value--text">Nova movimentação</div>
        <div class="data-note">disponível em qualquer tela</div>
      </div>
      <div class="data-cell">
        <div class="data-label">Esc</div>
        <div class="data-value data-value--text">Fechar modal</div>
        <div class="data-note">também fecha pelo botão ×</div>
      </div>
    </div>

    <footer class="panel-footer">
      <div class="panel-footnote">
        Exclusões e restaurações pedem confirmação antes de alterar o banco.
      </div>
    </footer>
  </section>
{/if}
