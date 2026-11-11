# Tally

Aplicativo desktop local-first para registrar, acompanhar e medir a vida financeira pessoal.

**Fase atual:** Fase 3 — Reports & Data concluída. O app agora entrega dashboard com dados reais, relatórios de gastos, receitas, fluxo de caixa, patrimônio e compromissos, além de backup/restauração local, exportação CSV e verificação de integridade — tudo calculado no Rust a partir dos lançamentos no SQLite.

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
- UI por hash router: visão geral, movimentações, contas, extrato, cartões, parcelamentos, categorias e configurações.
- Testes financeiros de integração, rollback, períodos, saldos e histórico.
- `Ctrl+N`, confirmações destrutivas, foco em formulários e erros reais via IPC.

## Fase 2 — entregue

- Cartão de crédito como conta de passivo, com limite informativo e dívida inicial.
- Compra parcelada de 1 a 60 vezes, com divisão exata do total e resto nas primeiras parcelas.
- Vencimentos preservando o dia original e ajustando para o último dia do mês quando necessário.
- Reconhecimento de despesa por parcela, sem duplicar a dívida em compromissos futuros.
- Pagamento de cartão como saída de caixa, FIFO: dívida inicial primeiro, depois parcelas mais antigas.
- Pagamento parcial, exclusão de pagamento reabrindo parcelas e exclusão segura de parcelamento.
- Resumo de dívida, disponível, vencido, mês atual, futuro e alerta de limite excedido.
- Telas de cartões e detalhe do cartão com parcelamentos, parcelas e pagamentos.
- Telas de parcelamentos com lista consolidada e detalhe do cronograma.
- Reconhecimentos por parcela excluídos do livro de movimentações comuns; compras e pagamentos de cartão abrem a tela do cartão.
- Compra no cartão também disponível pelo fluxo geral de nova movimentação, com escolha de cartão e compra à vista ou parcelada.
- Livro de movimentações mantém a compra parcelada em uma única linha, exibindo o total da compra, o resumo das parcelas e link para o parcelamento.
- Período visível por parcela, herdado da compra.
- Pagamento acima da dívida permanece não alocado, aparece como crédito no cartão e nunca vira despesa.
- Exclusão de parcelamento com pagamentos bloqueada com explicação explícita na interface.
- Testes financeiros de integração cobrindo os cenários críticos da fase, incluindo pagamento acima da dívida.

## Fase 3 — entregue

- Dashboard com patrimônio líquido, resultado do mês, dívida de cartão, compromissos futuros, maiores despesas e últimos lançamentos.
- Relatório de gastos e receitas por categoria, com filtros combináveis por conta, forma de pagamento, período, período do dia e categoria.
- Fluxo de caixa consolidado por mês e por origem, excluindo transferências, saldos iniciais e reconhecimentos de parcela, com filtros por conta, forma de pagamento, categoria, período e datas.
- Patrimônio líquido em uma data, com ativos, passivos e exposição separada da conta interna "Despesa a reconhecer".
- Compromissos futuros por mês e parcelas vencidas, com horizonte padrão de 12 meses.
- Backup local via `VACUUM INTO`, retenção configurável, listagem e restauração com snapshot de segurança, validação de schema e referências, migração pendente e rechecagem de integridade.
- Exportação CSV UTF-8 com BOM, separador `;`, CRLF e valores em BRL, respeitando os mesmos filtros da lista de movimentações.
- Verificação de integridade combinando `PRAGMA integrity_check` e `PRAGMA foreign_key_check`.
- Backup automático antes de aplicar migrations pendentes ao abrir o banco; backups de migração e de restauração não entram na retenção normal.
- Testes de integração para fórmulas, filtros, parcelas, fluxo de caixa, patrimônio, compromissos, backup/restore, CSV e migração de backup legado.
