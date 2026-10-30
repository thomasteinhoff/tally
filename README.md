# Tally

Aplicativo desktop local-first para registrar, acompanhar e medir a vida financeira pessoal.

**Fase atual:** Fase 0 — Fundação. O app abre, resolve o diretório de dados, cria/migra o SQLite, semeia as contas de sistema e expõe um bootstrap real via IPC.

## Stack

- Tauri 2.12.x
- Rust (workspace: `tally-domain`, `tally-app`, `tally-infra`, `src-tauri`)
- Svelte 5 + Vite + TypeScript
- SQLite (`rusqlite 0.40` bundled, `tokio-rusqlite 0.8`, `rusqlite_migration 2.6`)
- TanStack Query para estado de servidor; runes do Svelte para estado local

> TypeScript está em 6.0.3 porque o `svelte-check` atual ainda não declara suporte a TypeScript 7. É uma restrição de tooling, não uma mudança de arquitetura.

## Requisitos de desenvolvimento

- Node.js 24+
- Rust 1.95+ (testado com 1.98.1)
- WebView2 (Windows 10/11)
- SQLite não precisa estar instalado: o `rusqlite` usa a build `bundled`.

## Desenvolvimento

```powershell
npm install
npm run tauri dev
```

## Verificações

```powershell
npm run check
npm test
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Build

```powershell
npm run tauri build
```

O instalador NSIS é gerado em `target/release/bundle/nsis/`.

## Dados

Em produção, o Tally usa:

```text
%LOCALAPPDATA%\Tally\
  tally.db
  backups\
  exports\
  logs\
```

O caminho é resolvido pela API de paths do Tauri; não há caminho hardcoded.

## Estrutura

```text
crates/
  tally-domain/   regras puras (dinheiro, período)
  tally-app/      casos de uso, DTOs, portas, erros
  tally-infra/    SQLite, migrations, repositórios
src-tauri/        shell Tauri, comandos IPC, paths, logging
src/              UI Svelte
```

## Fase 0 — entregue

- Workspace Rust e shell Tauri.
- UI Svelte/Vite com shell mínimo e bootstrap real.
- Diretório de dados e logging em arquivo.
- SQLite com WAL, `synchronous=FULL`, FKs, migrations e contas de sistema.
- Trigger de equilíbrio de partidas dobradas (draft → post).
- `get_bootstrap` e `run_integrity_check`.
- CI Windows.
