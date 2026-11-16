<div align="center">
  <img src="tally.png" alt="Símbolo do Tally" width="88" height="88" />
  <h1>Tally</h1>
  <p><strong>Controle financeiro pessoal, local-first.</strong></p>
  <p>Aplicativo desktop para Windows. Sem conta, sem nuvem, sem assinatura.</p>
</div>

---

## Sobre o projeto

O Tally registra e acompanha a vida financeira pessoal a partir de partidas
dobradas: cada movimentação afeta contas reais e o patrimônio de forma
consistente. Tudo é calculado localmente, em Rust, sobre um banco SQLite que
fica no seu computador.

Não há login, sincronização ou servidor. O aplicativo funciona offline e os
dados financeiros nunca saem da máquina.

<!-- Capturas de tela: adicionar aqui quando estiverem disponíveis. -->

## Funcionalidades

- Contas de ativo e cartões de crédito, com limite, dívida e disponível.
- Receitas, despesas, transferências e ajustes.
- Compras parceladas de 1 a 60 vezes, com vencimentos ajustados por mês.
- Compromissos futuros e parcelas vencidas.
- Visão geral com patrimônio líquido, resultado do mês e últimos lançamentos.
- Relatórios e análises por categoria, com filtros combináveis.
- Fluxo de caixa por mês e por origem.
- Backup local, restauração com snapshot de segurança e exportação CSV.
- Temas claro, escuro e sistema.

## Download

O Tally é distribuído como instalador NSIS para Windows 10/11.

**[Baixar a versão mais recente](https://github.com/thomasteinhoff/tally/releases/latest)**

Versões anteriores e notas de cada lançamento ficam na página de
[Releases](https://github.com/thomasteinhoff/tally/releases).

## Privacidade e armazenamento

- Os dados ficam em um único banco SQLite local, em `%LOCALAPPDATA%\Tally\`.
- Não existe sincronização em nuvem, telemetria ou necessidade de login.
- O Tally cria backups locais antes de migrações e permite backup manual.
- O usuário é responsável por manter e guardar seus próprios backups.

## Tecnologias

- **Rust** — regras financeiras, casos de uso e acesso a dados.
- **Tauri 2** — shell desktop, comandos IPC e empacotamento.
- **Svelte 5** — interface, com runes para estado local.
- **TypeScript** — tipagem da UI e bindings gerados.
- **SQLite** — armazenamento local com migrations.

## Desenvolvimento

Requisitos:

- Node.js 24+
- Rust 1.95+
- WebView2 (Windows 10/11)
- SQLite não precisa ser instalado: o `rusqlite` usa a build `bundled`.

Para rodar em desenvolvimento:

```powershell
npm install
npm run tauri dev
```

Verificações:

```powershell
npm run check
npm test
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Build do instalador:

```powershell
npm run tauri build
```

O instalador NSIS é gerado em `target/release/bundle/nsis/`.

## Estrutura

```text
crates/
  tally-domain/   regras puras (dinheiro, período, ledger, contas, categorias)
  tally-app/      casos de uso, DTOs, portas e erros
  tally-infra/    SQLite, migrations e repositórios
src-tauri/        shell Tauri, comandos IPC, paths e logging
src/              interface Svelte, rotas e formulários
```

## Licença

MIT, conforme declarado em `Cargo.toml`.
