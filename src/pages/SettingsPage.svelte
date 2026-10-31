<script lang="ts">
  import { createMutation, createQuery } from "@tanstack/svelte-query";

  import { getBootstrap, runIntegrityCheck } from "../lib/api";
  import { normalizeError } from "../lib/errors";

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
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Sistema</span>
    <h2>Configurações</h2>
    <p>
      Diagnóstico local do banco. Dados de backup, schema e caminhos vêm do
      Rust; nada é simulado.
    </p>
  </div>
</section>

{#if bootstrapQuery.isPending}
  <section class="panel" aria-busy="true">
    <div class="state-panel">
      <span class="state-kicker">Bootstrap</span>
      <h2>Lendo o banco local.</h2>
      <div class="loading-rule" aria-hidden="true"></div>
    </div>
  </section>
{:else if bootError}
  <section class="panel">
    <div class="state-panel state-panel--error" role="alert">
      <span class="state-kicker">{bootError.code}</span>
      <h2>O banco local não respondeu.</h2>
      <p>{bootError.message}</p>
      {#if bootError.details}
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
    </div>
  </section>
{:else if data}
  <section class="panel" aria-labelledby="settings-status-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">01</span>
        <h3 class="panel-title" id="settings-status-title">
          Estado do banco
        </h3>
      </div>
      <span
        class="badge"
        class:badge--ok={Boolean(integrity?.ok)}
        class:badge--warn={Boolean(integrity && !integrity.ok)}
      >
        {integrity ? (integrity.ok ? "SQLite ok" : "SQLite atenção") : "aguardando"}
      </span>
    </header>

    <div class="data-grid">
      <div class="data-cell">
        <div class="data-label">Aplicação</div>
        <div class="data-value">{data.app_version}</div>
        <div class="data-note">Tauri · Windows</div>
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
      <div class="data-cell">
        <div class="data-label">Retenção de backups</div>
        <div class="data-value">{data.settings.backup_retention}</div>
        <div class="data-note">cópias configuradas</div>
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
      </div>
      <div class="actions">
        <button
          type="button"
          class="button"
          onclick={() => integrityMutation.mutate()}
          disabled={integrityMutation.isPending}
        >
          {integrityMutation.isPending ? "Verificando…" : "Verificar integridade"}
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
        <span>{integrityError.message}</span>
      </div>
    {/if}
  </section>
{/if}
