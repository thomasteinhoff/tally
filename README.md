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

Não há login, sincronização ou servidor. O aplicativo funciona offline. A única
requisição opcional consulta a taxa pública do CDI no Banco Central e envia
apenas o identificador da série e um intervalo de datas — nenhum dado
financeiro ou pessoal sai da máquina.

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
- Cofrinhos com aplicações, resgates, ajustes e rendimentos estimados pelo CDI.
- Temas claro, escuro e sistema.

## Download

O Tally é distribuído como instalador NSIS para Windows 10/11.

**[Baixar a versão mais recente](https://github.com/thomasteinhoff/tally/releases/latest)**

Versões anteriores e notas de cada lançamento ficam na página de
[Releases](https://github.com/thomasteinhoff/tally/releases).

## Cofrinhos e CDI

Os rendimentos são uma **estimativa local do Tally**, não uma reprodução
contratual de produto financeiro algum. A taxa diária vem da SGS 12 do Banco
Central (`% p.d.`, convertida para fração decimal dividindo por 100) e fica em
cache no SQLite. O cache só muda quando o usuário aciona “Atualizar taxas”:
abrir o aplicativo ou gerar um rendimento nunca faz rede.

Com `p = percentual contratado / 10.000` e `r = taxa diária publicada`, o Tally
calcula `yield_cents = round(base_cents × (f − 1))`, onde:

- `business`: `f = 1 + r × p` e exige taxa publicada exatamente na data;
- `calendar`: `f = (1 + r × p)^(252/365)`, usa a última taxa publicada em ou
  antes da data e é uma aproximação matemática do Tally para distribuir o
  rendimento em 365 dias corridos, inclusive sem publicação na data. Não é uma
  convenção contratual.

A base é o saldo no início do dia: saldo inicial, aplicações, resgates e ajustes
da própria data só entram na base no dia seguinte. Cada rendimento registra a
data da publicação, a série (`12`), a taxa diária usada e quando ela foi obtida;
mudar a configuração ou atualizar o cache não recalcula snapshots já gravados.

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
