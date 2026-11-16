<script lang="ts">
  import {
    createMutation,
    createQuery,
    useQueryClient,
  } from "@tanstack/svelte-query";
  import { openUrl } from "@tauri-apps/plugin-opener";

  import {
    createBackup,
    exportCsv,
    getBootstrap,
    getSettings,
    listAccounts,
    listBackups,
    listCategories,
    openAppFolder,
    restoreBackup,
    runIntegrityCheck,
    updateSettings,
  } from "../lib/api";
  import type { AppErrorDto } from "../lib/api/bindings";
  import { normalizeError } from "../lib/errors";
  import { confirmAction } from "../lib/confirm";
  import { formatDate, kindLabel, paymentLabel } from "../lib/format";
  import {
    setThemePreference,
    themeState,
  } from "../lib/theme.svelte";
  import {
    THEME_PREFERENCES,
    THEME_SHORT_LABELS,
  } from "../lib/theme";

  const queryClient = useQueryClient();

  const bootstrapQuery = createQuery(() => ({
    queryKey: ["bootstrap"],
    queryFn: getBootstrap,
  }));

  const settingsQuery = createQuery(() => ({
    queryKey: ["settings"],
    queryFn: getSettings,
  }));

  const backupsQuery = createQuery(() => ({
    queryKey: ["backups"],
    queryFn: listBackups,
  }));

  const accountsQuery = createQuery(() => ({
    queryKey: ["accounts", true],
    queryFn: () => listAccounts(true),
  }));

  const categoriesQuery = createQuery(() => ({
    queryKey: ["categories", true],
    queryFn: () => listCategories(true),
  }));

  let retention = $state(10);
  let exportFrom = $state("");
  let exportTo = $state("");
  let exportAccountId = $state("");
  let exportCategoryId = $state("");
  let exportKind = $state("");
  let exportPeriod = $state("");
  let exportPaymentMethod = $state("");

  let folderError = $state<AppErrorDto | null>(null);
  let opening = $state<"data" | "logs" | null>(null);

  const data = $derived(bootstrapQuery.data);
  const settings = $derived(settingsQuery.data ?? data?.settings);
  const backups = $derived(backupsQuery.data ?? []);
  const accounts = $derived(accountsQuery.data ?? []);
  const categories = $derived(categoriesQuery.data ?? []);

  const kindOptions = [
    "expense",
    "income",
    "transfer",
    "adjustment",
    "card_purchase",
    "card_payment",
  ];

  const periodOptions = [
    ["madrugada", "Madrugada"],
    ["manha", "Manhã"],
    ["tarde", "Tarde"],
    ["noite", "Noite"],
    ["custom", "Personalizado"],
  ];

  const paymentOptions = [
    "pix",
    "debit",
    "credit",
    "subscription_credit",
    "cash",
  ];

  const repositoryUrl = "https://github.com/thomasteinhoff/tally";

  $effect(() => {
    if (settings) {
      retention = settings.backup_retention;
    }
  });

  const retentionError = $derived(
    !Number.isInteger(retention) || retention < 1
      ? "Use um inteiro maior ou igual a 1."
      : null,
  );

  const bootError = $derived(
    bootstrapQuery.error ? normalizeError(bootstrapQuery.error) : null,
  );
  const backupsError = $derived(
    backupsQuery.error ? normalizeError(backupsQuery.error) : null,
  );

  const integrityMutation = createMutation(() => ({
    mutationFn: runIntegrityCheck,
  }));

  const createBackupMutation = createMutation(() => ({
    mutationFn: createBackup,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["backups"] });
      void queryClient.invalidateQueries({ queryKey: ["settings"] });
      void queryClient.invalidateQueries({ queryKey: ["bootstrap"] });
    },
  }));

  const restoreMutation = createMutation(() => ({
    mutationFn: restoreBackup,
    onSuccess: () => {
      void queryClient.invalidateQueries();
    },
  }));

  const settingsMutation = createMutation(() => ({
    mutationFn: updateSettings,
    onSuccess: (updated) => {
      queryClient.setQueryData(["settings"], updated);
      void queryClient.invalidateQueries({ queryKey: ["bootstrap"] });
    },
  }));

  const exportMutation = createMutation(() => ({
    mutationFn: exportCsv,
  }));

  const integrityError = $derived(
    integrityMutation.error ? normalizeError(integrityMutation.error) : null,
  );
  const createBackupError = $derived(
    createBackupMutation.error
      ? normalizeError(createBackupMutation.error)
      : null,
  );
  const restoreError = $derived(
    restoreMutation.error ? normalizeError(restoreMutation.error) : null,
  );
  const settingsError = $derived(
    settingsMutation.error ? normalizeError(settingsMutation.error) : null,
  );
  const exportError = $derived(
    exportMutation.error ? normalizeError(exportMutation.error) : null,
  );
  const integrity = $derived(
    integrityMutation.data ?? data?.integrity ?? null,
  );

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

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function saveRetention(): void {
    if (retentionError) return;
    settingsMutation.mutate({ backup_retention: retention });
  }

  async function restore(fileName: string): Promise<void> {
    const confirmed = await confirmAction(
      `Restaurar "${fileName}"? Os dados atuais serão substituídos.`,
      "Restaurar backup",
      "Restaurar",
    );
    if (!confirmed) return;
    restoreMutation.mutate(fileName);
  }

  function handleExport(): void {
    exportMutation.mutate({
      account_id: exportAccountId || null,
      category_id: exportCategoryId || null,
      kind: exportKind || null,
      period: exportPeriod || null,
      payment_method: exportPaymentMethod || null,
      from: exportFrom || null,
      to: exportTo || null,
    });
  }
</script>

<section class="page-intro">
  <div>
    <span class="eyebrow">Sistema</span>
    <h2>Configurações</h2>
    <p>
      Aparência, aplicativo, dados e sobre em um só lugar. Diagnóstico, backups
      e exportação rodam no Rust sobre o banco local; nada é simulado no
      frontend.
    </p>
  </div>
</section>

<section class="panel" aria-labelledby="settings-theme-title">
  <header class="panel-header">
    <div class="panel-heading">
      <span class="panel-kicker">01</span>
      <h3 class="panel-title" id="settings-theme-title">Aparência</h3>
    </div>
    <span class="panel-meta">preferência local</span>
  </header>

  <div class="panel-body">
    <div class="theme-choices" role="group" aria-label="Tema da interface">
      {#each THEME_PREFERENCES as preference (preference)}
        <button
          type="button"
          class="button"
          class:button--primary={themeState.preference === preference}
          aria-pressed={themeState.preference === preference}
          onclick={() => setThemePreference(preference)}
        >
          {THEME_SHORT_LABELS[preference]}
        </button>
      {/each}
    </div>
  </div>

  <footer class="panel-footer">
    <div class="panel-footnote">
      <span class="mono">tema</span>
      <span>
        Aplicado na hora e salvo neste computador. “Sistema” acompanha o Windows.
      </span>
    </div>
  </footer>
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
  <section class="panel" aria-labelledby="settings-app-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">02</span>
        <h3 class="panel-title" id="settings-app-title">Aplicativo</h3>
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
        <span class="mono">atalhos</span>
        <span>
          Ctrl+N abre nova movimentação em qualquer tela · Esc fecha o modal
          (ou use ×).
        </span>
      </div>
    </footer>
  </section>

  <section class="panel" aria-labelledby="settings-data-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">03</span>
        <h3 class="panel-title" id="settings-data-title">Dados</h3>
      </div>
      <span
        class="badge"
        class:badge--ok={Boolean(integrity?.ok)}
        class:badge--warn={Boolean(integrity && !integrity.ok)}
      >
        {integrity
          ? integrity.ok
            ? "SQLite ok"
            : "SQLite atenção"
          : "aguardando"}
      </span>
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
          class="button button--small button--quiet"
          onclick={() => void openFolder("logs")}
          disabled={opening !== null}
        >
          {opening === "logs" ? "Abrindo…" : "Abrir logs"}
        </button>
        <button
          type="button"
          class="button button--small button--quiet"
          onclick={() => void openFolder("data")}
          disabled={opening !== null}
        >
          {opening === "data" ? "Abrindo…" : "Abrir pasta"}
        </button>
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

    {#if folderError}
      <div class="inline-result inline-result--warn" role="alert">
        <span class="mono">{folderError.code}</span>
        <span>{folderError.message}</span>
      </div>
    {/if}

    <header class="panel-header panel-header--divided">
      <div class="panel-heading">
        <span class="panel-kicker">Backups</span>
        <h3 class="panel-title">Cópias locais</h3>
      </div>
      <span class="panel-meta">
        {backups.length} {backups.length === 1 ? "arquivo" : "arquivos"}
      </span>
    </header>

    <div class="filters filters--compact">
      <label class="field">
        <span>
          Retenção
          <small>quantas cópias manter</small>
        </span>
        <input
          type="number"
          min="1"
          step="1"
          bind:value={retention}
          disabled={settingsMutation.isPending}
        />
      </label>
      <div class="actions">
        <button
          type="button"
          class="button"
          onclick={saveRetention}
          disabled={Boolean(retentionError) || settingsMutation.isPending}
        >
          {settingsMutation.isPending ? "Salvando…" : "Salvar retenção"}
        </button>
        <button
          type="button"
          class="button button--primary"
          onclick={() => createBackupMutation.mutate()}
          disabled={createBackupMutation.isPending}
        >
          {createBackupMutation.isPending ? "Criando…" : "Criar backup"}
        </button>
      </div>
    </div>

    {#if settings?.last_backup_at}
      <div class="panel-footer">
        <div class="panel-footnote">
          <span class="mono">último backup</span>
          <span>{settings.last_backup_at}</span>
        </div>
      </div>
    {/if}

    {#if backupsQuery.isPending}
      <div class="state-panel" aria-busy="true">
        <span class="state-kicker">Lendo</span>
        <h2>Listando backups.</h2>
        <div class="loading-rule" aria-hidden="true"></div>
      </div>
    {:else if backups.length === 0}
      <div class="empty-panel">
        <p>Nenhum backup ainda. Crie a primeira cópia local.</p>
      </div>
    {:else}
      <div class="table-wrap">
        <table class="data-table">
          <thead>
            <tr>
              <th scope="col">Arquivo</th>
              <th scope="col">Criado em</th>
              <th scope="col" class="align-right">Tamanho</th>
              <th scope="col"></th>
            </tr>
          </thead>
          <tbody>
            {#each backups as backup (backup.file_name)}
              <tr>
                <td>
                  <div class="cell-stack">
                    <span>{backup.file_name}</span>
                    <small>{backup.path}</small>
                  </div>
                </td>
                <td class="mono">{formatDate(backup.created_at.slice(0, 10))}</td>
                <td class="mono align-right">
                  {formatBytes(backup.size_bytes)}
                </td>
                <td class="align-right">
                  <button
                    type="button"
                    class="button button--small button--quiet"
                    onclick={() => void restore(backup.file_name)}
                    disabled={restoreMutation.isPending}
                  >
                    Restaurar
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}

    {#if restoreMutation.isSuccess}
      <div class="inline-result inline-result--ok" role="status">
        <span class="mono">restore</span>
        <span>
          {restoreMutation.data.backup.file_name} restaurado · integridade
          {restoreMutation.data.integrity.ok ? "ok" : "com atenção"}
        </span>
      </div>
    {/if}

    {#if createBackupError}
      <div class="inline-result inline-result--warn" role="alert">
        <span class="mono">{createBackupError.code}</span>
        <span>{createBackupError.message}</span>
      </div>
    {/if}

    {#if settingsError}
      <div class="inline-result inline-result--warn" role="alert">
        <span class="mono">{settingsError.code}</span>
        <span>{settingsError.message}</span>
      </div>
    {/if}

    {#if restoreError}
      <div class="inline-result inline-result--warn" role="alert">
        <span class="mono">{restoreError.code}</span>
        <span>{restoreError.message}</span>
      </div>
    {/if}

    {#if backupsError}
      <div class="inline-result inline-result--warn" role="alert">
        <span class="mono">{backupsError.code}</span>
        <span>{backupsError.message}</span>
      </div>
    {/if}

    <header class="panel-header panel-header--divided">
      <div class="panel-heading">
        <span class="panel-kicker">Exportação</span>
        <h3 class="panel-title">Exportar CSV</h3>
      </div>
      <span class="panel-meta">UTF-8 · ponto e vírgula</span>
    </header>

    <div class="filters filters--compact">
      <label class="field">
        <span>De</span>
        <input type="date" bind:value={exportFrom} />
      </label>
      <label class="field">
        <span>Até</span>
        <input type="date" bind:value={exportTo} />
      </label>
      <label class="field">
        <span>Conta</span>
        <select bind:value={exportAccountId}>
          <option value="">Todas</option>
          {#each accounts as account (account.id)}
            <option value={account.id}>
              {account.name}{account.archived_at ? " (arquivada)" : ""}
            </option>
          {/each}
        </select>
      </label>
      <label class="field">
        <span>Categoria</span>
        <select bind:value={exportCategoryId}>
          <option value="">Todas</option>
          {#each categories as category (category.id)}
            <option value={category.id}>
              {kindLabel(category.kind)} · {category.name}
            </option>
          {/each}
        </select>
      </label>
      <label class="field">
        <span>Tipo</span>
        <select bind:value={exportKind}>
          <option value="">Todos</option>
          {#each kindOptions as kind (kind)}
            <option value={kind}>{kindLabel(kind)}</option>
          {/each}
        </select>
      </label>
      <label class="field">
        <span>Período do dia</span>
        <select bind:value={exportPeriod}>
          <option value="">Todos</option>
          {#each periodOptions as [value, label] (value)}
            <option value={value}>{label}</option>
          {/each}
        </select>
      </label>
      <label class="field">
        <span>Forma de pagamento</span>
        <select bind:value={exportPaymentMethod}>
          <option value="">Todas</option>
          {#each paymentOptions as method (method)}
            <option value={method}>{paymentLabel(method)}</option>
          {/each}
        </select>
      </label>
      <div class="actions">
        <button
          type="button"
          class="button button--primary"
          onclick={handleExport}
          disabled={exportMutation.isPending}
        >
          {exportMutation.isPending ? "Exportando…" : "Exportar movimentações"}
        </button>
      </div>
    </div>

    <footer class="panel-footer">
      <div class="panel-footnote">
        <span class="mono">vazio</span>
        <span>
          Sem filtros, todas as movimentações são exportadas; os campos seguem
          o mesmo filtro das movimentações.
        </span>
      </div>
    </footer>

    {#if exportMutation.isSuccess}
      <div class="inline-result inline-result--ok" role="status">
        <span class="mono">{exportMutation.data.row_count} linhas</span>
        <span>{exportMutation.data.path}</span>
      </div>
    {/if}

    {#if exportError}
      <div class="inline-result inline-result--warn" role="alert">
        <span class="mono">{exportError.code}</span>
        <span>{exportError.message}</span>
      </div>
    {/if}
  </section>

  <section class="panel" aria-labelledby="settings-about-title">
    <header class="panel-header">
      <div class="panel-heading">
        <span class="panel-kicker">04</span>
        <h3 class="panel-title" id="settings-about-title">Sobre</h3>
      </div>
      <span class="panel-meta">local-first · offline</span>
    </header>

    <div class="panel-body">
      <p class="about-lede">
        Tally é um instrumento de precisão financeira, local-first e offline.
        Seus dados vivem em um único arquivo SQLite neste computador.
      </p>
      <div class="about-notes">
        <div>
          <div class="data-label">Créditos</div>
          <p>
            Rust e SQLite no backend, Tauri na janela, Svelte no frontend.
            Nenhum número financeiro é calculado no navegador.
          </p>
        </div>
        <div>
          <div class="data-label">Licenças</div>
          <p>
            IBM Plex © IBM, SIL Open Font License 1.1. Rust, Tauri e Svelte em
            MIT/Apache-2.0; SQLite em domínio público.
          </p>
        </div>
        <div>
          <div class="data-label">Seus dados</div>
          <p>
            Atualizar ou desinstalar o Tally não remove os arquivos de dados.
            Para apagar, remova a pasta manualmente.
          </p>
        </div>
      </div>
    </div>

    <footer class="panel-footer">
      <div class="panel-footnote">
        Exclusões e restaurações pedem confirmação antes de alterar o banco.
      </div>
      <div class="actions">
        <button
          type="button"
          class="button"
          onclick={() => void openUrl(repositoryUrl)}
        >
          Repositório
        </button>
      </div>
    </footer>
  </section>
{/if}
