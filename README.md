# Tally

Aplicativo desktop local-first para registrar, acompanhar e medir a vida financeira pessoal.

**Fase atual:** Fase 1 — Core Accounting concluída. O app permite cadastrar contas e categorias, lançar despesas, receitas, transferências e ajustes, consultar o livro, filtrar movimentações e abrir extratos por conta com filtros, saldo atual e saldos por período — tudo derivado dos lançamentos no SQLite.

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
  tally-domain/   regras puras (dinheiro, período, ledger, contas, categorias)
  tally-app/      casos de uso, DTOs, portas, erros
  tally-infra/    SQLite, migrations, repositórios
src-tauri/        shell Tauri, comandos IPC, paths, logging
src/              UI Svelte, rotas e formulários
```

## Fase 0 — entregue

- Workspace Rust e shell Tauri.
- UI Svelte/Vite com shell mínimo e bootstrap real.
- Diretório de dados e logging em arquivo.
- SQLite com WAL, `synchronous=FULL`, FKs, migrations e contas de sistema.
- Trigger de equilíbrio de partidas dobradas (draft → post).
- `get_bootstrap` e `run_integrity_check`.
- CI Windows.

## Fase 1 — entregue

- Contas de ativo com saldo inicial, rename, arquivamento e exclusão segura.
- Categorias de despesa/receita com tipo imutável, rename, arquivamento e exclusão segura.
- Despesa, receita, transferência e ajuste como lançamentos balanceados.
- Períodos derivados pelo SQLite a partir do horário, com período manual quando não há horário.
- Lista de movimentações com filtros por conta, categoria, tipo, período e datas.
- Extrato por conta com filtros equivalentes, saldo atual como foco no histórico completo e saldos de início/fim quando há período selecionado.
- Edição e exclusão atômicas; confirmação destrutiva antes de qualquer mutation, sem sucesso otimista.
- Arquivamento fora do uso normal sem apagar o histórico, com explicação na interface.
- UI por hash router: visão geral, movimentações, contas, extrato, categorias e configurações.
- Testes financeiros de integração, rollback, períodos, saldos e histórico.
- `Ctrl+N`, confirmações destrutivas, foco em formulários e erros reais via IPC.
