# Tally — Roadmap v0.2.0

> Documento de referência para continuidade do desenvolvimento.
>
> **Fonte de verdade:** o código do repositório define o que já existe; este
> documento define decisões, escopo e ordem de implementação. Itens planejados
> **não** devem ser tratados como implementados.
>
> Este arquivo substitui o antigo `TALLY_ROADMAP.md`, removido do repositório no
> commit `957bf37` durante a limpeza da v0.1.0. Uma cópia histórica permanece em
> `C:\Users\Thomas\Documents\Projects\codeprojects\tally-local-archive\TALLY_ROADMAP.md`.

---

## 1. Estado atual

- Versão publicada: **v0.1.0**.
- Commit da release: `82105cf44907b2b940d17fceb0131ff812449514` (`feat: finalize Tally 0.1.0`).
- Release: `https://github.com/thomasteinhoff/tally/releases/tag/v0.1.0`
  (instalador `Tally_0.1.0_x64-setup.exe`, SHA-256
  `313b3bb53243db19f07438594cd0b812ad7c96bb690064ef6f1832f4436ccf32`).
- Commit de limpeza/documentação: `957bf3725fe1c2a107f5f722562d2ef23478d605`
  (`chore: clean repository and improve documentation`).
- `origin/main` = `957bf37`; tag `v0.1.0` aponta para `82105cf`.
- v0.2.0 em andamento: **Etapa 1 concluída** em
  `ca80d70e03e43758420d5c67568e8ab375ff4500`
  (`feat: ordena movimentações cronologicamente e ajusta formulário`).
- **Etapa 2 concluída** em `81977c84efd03bc4dcb4592f7893fba12afad167`
  (`feat: adiciona ícones de categoria e seletor com suporte visual`).
- **Etapa 3 concluída** em `186074e046cca95f763260e70cd45ed5f937cf41`
  (`feat: separa visao geral e quadro de analises`).
- **Etapa 4 concluída** em `435c980a56ebdbd0cf514d313b6491e210a46256`
  (`feat: conclui o quadro de analises da v0.2.0`); Etapa 5 pendente.
- Repositório público com descrição e topics atualizados; sem homepage e sem
  arquivo `LICENSE` (o `Cargo.toml` declara MIT).
- `package.json` ainda declara versão `0.1.0`; a atualização de versão faz parte
  do fechamento da v0.2.0.

### Arquitetura e princípios mantidos

- Rust é autoridade sobre regras financeiras, saldos e persistência; o frontend
  nunca calcula saldos nem escreve diretamente no banco.
- Tauri 2 + Svelte 5/TypeScript + SQLite (`rusqlite` bundled); TanStack Query
  para estado de servidor, runes do Svelte para estado local.
- Uma mutação = uma transação; relatórios leem apenas lançamentos publicados.
- Local-first/offline: dados em `%LOCALAPPDATA%\Tally\`, sem login, nuvem,
  sincronização ou telemetria.
- BRL e pt-BR apenas; Windows como plataforma inicial.
- Identidade visual preservada: tema grafite escuro, IBM Plex, símbolo
  `tally.png`, `--accent`/terracota reservado a marca e ações, `--positive` e
  `--negative` para a semântica de receita/despesa.

---

## 2. Decisões consolidadas da v0.2.0

### 2.1 Navegação (ordem intencional)

**Operação**

1. Visão Geral
2. Quadro de Análises
3. Movimentações
4. Contas e cartões
5. Parcelamentos

**Sistema**

6. Categorias
7. Configurações

- `#/quadro` é a rota do Quadro de Análises. O caminho legado `#/relatorios`
  passa a redirecionar para `#/quadro` (hoje aponta para Visão Geral).
- Rotas existentes (`#/contas`, `#/cartoes`, `#/parcelamentos`, `#/categorias`,
  `#/configuracoes`, extratos e detalhes) permanecem.

### 2.2 Visão Geral

- Preservar a primeira seção atual: patrimônio, ativos/passivos, resultado do
  mês, contas, cartões, resumo mensal, compromissos e últimos lançamentos.
- Limitar "Últimos lançamentos" a **5** itens.
- Remover gráficos, análises aprofundadas e o painel de filtros complexo.
- **Não** haverá seletor global de mês nesta tela.
- Saldos, patrimônio, limites e compromissos continuam **atuais**, independentes
  de qualquer seleção de mês feita no Quadro de Análises.
- O card resumido de Compromissos permanece aqui (decisão D2).
- O painel "Compromissos em detalhe" é removido da Visão Geral e **não** é
  migrado para o Quadro de Análises: o detalhamento completo passa a ser
  exclusividade da tela Parcelamentos (decisão D2).

### 2.3 Quadro de Análises

- Nova tela dedicada a gráficos, indicadores e comparações.
- Seletor global de mês no topo: mês atual por padrão, navegação entre meses.
- Todos os widgets respeitam o mês selecionado.
- Cada widget possui apenas controles próprios que agreguem valor:
  barras/rosquinha nos gráficos de categoria; acumulado/média diária no
  gráfico de período do dia.
- **Sem** personalização de layout pelo usuário: sem drag-and-drop, sem
  ocultar widgets, sem redimensionamento livre e sem sistema genérico de
  widgets. Componentes Svelte comuns, independentes e reutilizáveis, com props
  simples; a disposição visual é definida no código.
- Ordem de entrega e disposição inicial (definição manual no código):
  1. Fluxo de caixa (reaproveita consulta existente).
  2. Gastos por categoria e Receitas por categoria (barras/rosquinha).
  3. Maiores despesas do mês (top 5 da consulta de gastos).
  4. Gastos por período do dia (acumulado/média diária).
  5. Evolução dos gastos mensais (mês selecionado + cinco anteriores).
  6. Patrimônio histórico (último widget).
- Layout inicial sugerido: linha 1 com os dois cards de categoria lado a lado;
  linha 2 com fluxo de caixa; linha 3 com período do dia + evolução mensal;
  linha 4 com maiores despesas + patrimônio histórico.

### 2.4 Correções rápidas

- Ordenação de movimentações e últimos lançamentos por data e horário, do mais
  recente ao mais antigo, com lançamentos retroativos na posição cronológica
  correta. Registros sem horário ficam **depois** dos registros com horário na
  mesma data; o desempate final é `created_at`, depois `id`. Nenhum horário é
  inventado para quem registrou apenas período manual.
- Links comuns passam a usar cor neutra; terracota fica reservado a marca,
  ações e destaques. A diferenciação semântica receita/despesa é preservada.
- Faixas exibidas: Madrugada 00:00–04:59, Manhã 05:00–11:59, Tarde 12:00–18:59,
  Noite 19:00–23:59.
- No formulário de movimentação, o horário exato continua pré-preenchido com a
  hora atual, mas o período manual fica visível como alternativa. Horário e
  período manual são **mutuamente exclusivos**: preencher um limpa o outro.
  As regras atuais de derivação do período e as exceções manuais são mantidas.
- Lançamentos sem período identificado aparecem em quadro separado, nunca
  distribuídos artificialmente entre as quatro faixas.

### 2.5 Ícones nas categorias

- Coleção curada de aproximadamente 24 ícones SVG, com ícone genérico de
  fallback.
- Cada categoria pode ter um ícone; categorias personalizadas usam o mesmo
  picker; chave desconhecida ou vazia cai no genérico.
- O banco armazena apenas uma chave curta — nenhuma imagem ou coleção grande.
- Ícones em `<select>` nativos **não** são necessários. Eles aparecem no picker
  do cadastro, na tabela de categorias, na célula de categoria de
  Movimentações e nas legendas dos gráficos.
- Sem dependência npm nova; paleta da rosquinha derivada dos temas atuais.

### 2.6 Gastos por período do dia — métricas (decisão D1)

- **Acumulado do mês:** total gasto em cada faixa e participação percentual
  sobre o total de despesas do mês. A porcentagem de participação no gasto é
  **exclusiva desta visualização**.
- **Média diária:** `total_do_período / dias_distintos_com_despesa_naquele_período`.
  Exemplo: R$ 200 em quatro dias diferentes = R$ 50 por dia ativo.
- A visualização de média diária apresenta: valor médio, quantidade de dias
  ativos e **frequência** = `dias ativos no período / dias com qualquer despesa
  no mês`. A soma das frequências pode ultrapassar 100% porque um dia pode ter
  gastos em mais de um período; isso deve estar explícito na interface.
- A razão `média_do_período / soma_das_médias` **não** é usada como
  "participação percentual": ela mede peso em um dia composto, não parcela do
  gasto, e pode ser confusionada com o acumulado.
- Nenhum horário é inferido para lançamentos que só possuem período manual.

### 2.7 Compromissos (decisão D2)

- Card resumido de Compromissos permanece na Visão Geral.
- O detalhamento mês a mês não migra para o Quadro de Análises.
- A tela Parcelamentos é a fonte completa de compromissos e parcelas.

### 2.8 Patrimônio histórico

- Incluído como **último widget** do Quadro de Análises.
- Mostra a evolução do patrimônio líquido até o fechamento do mês selecionado:
  série mensal com o mês selecionado e os cinco anteriores, cada ponto calculado
  na data final do respectivo mês.
- O ponto final corresponde ao fechamento do mês selecionado. Este widget é
  histórico/analítico e não altera o patrimônio atual exibido na Visão Geral.

### 2.9 Cofrinhos e rendimentos (escopo fixado, detalhes adiados)

Regras já aprovadas e que devem ser preservadas:

- Aplicações e resgates são **transferências**, não despesas nem receitas.
- Rendimentos são registrados separadamente do valor aplicado, por um método
  específico de cofrinho — não como uma receita comum.
- **Já existe uma categoria chamada "Rendimentos"**: não criar outra categoria.
- O saldo dos cofrinhos aparece separadamente e **nunca** é somado ao
  patrimônio líquido exibido.
- Sem integração com Mercado Pago, corretoras ou sincronização automática. A
  consulta manual de taxas públicas ao SGS 12 passa a fazer parte da Etapa 5.
- Sem gestão genérica de investimentos: fora de escopo CDBs, ações, fundos,
  cotas, corretoras, passes de ônibus e saldos pré-pagos.
- Não haverá análise temporal de rendimentos nesta versão.

Decisões **explicitamente diferidas** para a etapa final (não tomar antes):

- local de gerenciamento (dentro de Contas e cartões, como tipo específico de
  conta, ou tela independente);
- modelo contábil exato dos rendimentos e validação contra os invariantes de
  transferência do Tally;
- visibilidade dos rendimentos na lista geral de movimentações;
- inclusão e forma de um card compacto de cofrinhos na Visão Geral — que, se
  existir, não poderá ser afetado pelo mês selecionado no Quadro de Análises.

### 2.10 Questões encerradas (não reabri sem novo pedido)

- Horário atual pré-preenchido; período manual visível como alternativa.
- Horário e período manual mutuamente exclusivos.
- Registros sem horário ordenados depois dos registros com horário na mesma data.
- Ícones em selects nativos: não necessários.
- Coleção inicial de ~24 ícones com fallback genérico.
- Paleta da rosquinha derivada dos temas atuais.
- Média diária por dias ativos, com frequência em vez de participação
  percentual (D1).
- Compromissos detalhados fora do Quadro de Análises (D2).
- Análise temporal de rendimentos: adiada.

---

## 3. Descobertas técnicas de base

Registros da investigação do código em `957bf37`, para orientar a implementação:

| Tema | Local | Achado |
|---|---|---|
| Ordenação | `crates/tally-infra/src/movements_repo.rs:196` | `ORDER BY e.occurred_on DESC, e.created_at DESC, e.id DESC`. A Visão Geral chama `listMovements()` sem filtro: uma mudança SQL corrige livro e recentes. |
| Extrato | `crates/tally-infra/src/movements_repo.rs:273` | Ordem ascendente por `occurred_on, created_at, id`. |
| Links | `src/app.css:1137` | `.text-link` usa `--accent-ink` (terracota); `:1505` já possui variante neutra. |
| Períodos | `crates/tally-domain/src/period.rs`, `crates/tally-infra/migrations/0001_init.sql:67` | Quatro faixas e coluna gerada `period` já corretas; `src/lib/format.ts:157` traz `"Manha"` sem acento. |
| Formulário | `src/lib/components/MovementFormModal.svelte:618-641` | Horário pré-preenchido; seletor manual só aparece quando o horário está vazio. |
| Gráficos | `src/lib/category-chart.ts`, `src/lib/components/CategoryBars.svelte` | `buildCategoryBars()` já faz top 8 + "Outras" e percentuais; o componente é só apresentação. |
| Fluxo de caixa | `crates/tally-infra/src/reports_repo.rs:118` | Whitelist `('expense','income','card_payment','adjustment')`: transferências já ficam fora. |
| Categorias | `crates/tally-infra/src/reports_repo.rs:43` | `category_report()` filtra por `e.kind` e contas sistêmicas de receita/despesa. |
| Patrimônio | `crates/tally-infra/src/reports_repo.rs:227-234` | `net_worth()` soma `class IN ('asset','liability')`; `reports.rs::dashboard()` reaproveita esse resultado. |
| Transferência | `crates/tally-app/src/movements.rs:169-205` | `create_transfer` valida IDs distintos, valor positivo, data/período e descrição; não restringe classe/tipo/`is_system` das contas. Invariantes adicionais (CHECKs, triggers, caminho de edição) devem ser avaliados na etapa 5. |
| Navegação | `src/App.svelte:130-186`, `src/lib/router.ts` | Navegação e rotas atuais; `#/relatorios` hoje mapeia para `overview`. |
| Análises | `src/pages/OverviewPage.svelte:52-210`, `:597-1112` | Filtros e análise completa vivem hoje na Visão Geral; `:503-537` é o resumo de Compromissos (primeira seção); `:1042-1110` é o detalhamento a remover. |
| Categorias | `crates/tally-app/src/categories.rs`, `categories_repo.rs` | `categories` tem apenas `id, name, kind`; não há categorias semeadas no bootstrap. |
| CI | `.github/workflows/ci.yml` | Após alterar DTOs, `npm run bindings` deve ser executado e não pode gerar diff em `src/lib/api/bindings`. |

---

## 4. Etapas de implementação

### Etapa 1 — Correções cronológicas e visuais ✅ CONCLUÍDA

**Status.** Concluída em `ca80d70e03e43758420d5c67568e8ab375ff4500`
(`feat: ordena movimentações cronologicamente e ajusta formulário`). Validações
completas: `npm run check`, `npm test`, `npm run build`, `cargo fmt --all --
--check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` e `npm run bindings` sem diff.

**Objetivo.** Ordenação cronológica real, links neutros, faixas horárias legíveis
e opção de período evidente no formulário.

**Arquivos e componentes.**

- `crates/tally-infra/src/movements_repo.rs` (`:196`; opcionalmente `:273`)
- `src/app.css` (`:1137`, `:1505`)
- `src/lib/format.ts` + `src/lib/format.test.ts`
- `src/lib/components/MovementFormModal.svelte` (`:616-641`, `:692-697`)
- `src/pages/OverviewPage.svelte` (`:119`)

**Regras de negócio.**

- Ordem: `occurred_on DESC → COALESCE(occurred_time,'') DESC → created_at DESC → id DESC`.
- Registros sem horário depois dos com horário na mesma data; período manual
  nunca convertido em hora.
- Links comuns com cor neutra; terracota preservado para marca/ações;
  `--positive`/`--negative` intactos.
- Rótulos das quatro faixas com intervalos; `"Manha"` → `"Manhã"`.
- Horário e período manual exclusivos; horário atual continua pré-preenchido.
- `recentMovements = movements.slice(0, 5)`.

**Dependências e riscos.** Empate na mesma data (regra já definida); contraste
dos links nos temas claro e escuro; nenhuma alteração na coluna gerada `period`.

**Testes.**

- Rust: lançamentos retroativos, mesma data com horários distintos, período
  manual e desempate por `created_at`.
- `npm run check` e `npm test` para rótulos/helpers de período.
- Conferência visual nos dois temas.

**Critérios de conclusão.**

- Livro e últimos lançamentos usam a mesma consulta ordenada.
- Cinco recentes na Visão Geral.
- Formulário exibe as quatro faixas e deixa claro que o horário é opcional.
- Links comuns neutros, sem alteração semântica de receita/despesa.

---

### Etapa 2 — Ícones nas categorias ✅ CONCLUÍDA

**Status.** Concluída em `81977c84efd03bc4dcb4592f7893fba12afad167`
(`feat: adiciona ícones de categoria e seletor com suporte visual`). Incluiu os
ajustes identificados no teste: seletor de categoria das movimentações
substituído por combobox acessível com ícones, traços mais encorpados/tamanho
ligeiramente maior e catálogo ampliado de 24 para 50 ícones. Validações
completas: `npm run check`, `npm test`, `npm run build`, `cargo fmt --all --
--check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` e `npm run bindings` sem diff.

**Objetivo.** Associar ícone curado a cada categoria com fallback genérico, sem
armazenar imagens no banco.

**Arquivos e componentes.**

- `crates/tally-infra/migrations/0003_category_icons.sql` (novo)
- `crates/tally-infra/src/categories_repo.rs`
- `crates/tally-app/src/categories.rs` + `crates/tally-app/src/dto.rs`
- `src-tauri/src/commands.rs` + `src/lib/api/bindings.ts` (regenerado)
- `src/lib/category-icons.ts` (novo) + `src/lib/components/CategoryIcon.svelte` (novo)
- `src/lib/components/CategoryFormModal.svelte`, `src/pages/CategoriesPage.svelte`
- `src/pages/MovementsPage.svelte:419`, `src/app.css`

**Regras de negócio.**

- Chave curta validada; desconhecida/vazia → `generic`.
- Tipo da categoria permanece imutável; histórico vinculado é preservado.
- Categorias personalizadas usam o mesmo picker.
- Nenhuma imagem, SVG completo ou coleção grande gravada no SQLite.

**Dependências e riscos.** Migration aditiva compatível com backup antigo;
`cargo test -p tally-app export_bindings` e `npm run bindings` não podem deixar
diff; ~24 paths SVG escritos à mão, sem dependência nova.

**Testes.**

- Rust: criar/renomear com ícone válido e inválido; fallback em listagem;
  compatibilidade com linhas sem ícone.
- TS: toda chave da coleção possui path e rótulo; `generic` sempre existe.
- `npm run bindings` sem diff; `npm test`.

**Critérios de conclusão.**

- Picker de ícones no cadastro e na edição de categorias.
- Ícones visíveis em categorias, movimentações e legendas do Quadro.
- Dados antigos continuam válidos com o fallback.

---

### Etapa 3 — Navegação, Visão Geral enxuta e Quadro de Análises base ✅ CONCLUÍDA

**Status.** Concluída em `186074e046cca95f763260e70cd45ed5f937cf41`
(`feat: separa visao geral e quadro de analises`). Incluiu os sete itens de
navegação na ordem definida, `#/quadro` com `#/relatorios` legado, Visão Geral
enxuta (sem filtros, gráficos ou detalhamento de compromissos, com cinco
lançamentos recentes) e o Quadro com seletor de mês e os quatro widgets
iniciais. `CategoryBars.svelte` foi substituído por `CategoryChart.svelte`
(barras/rosquinha com ícones). Validações completas: `npm run check`,
`npm test` (46 testes), `npm run build`, `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` e `npm run bindings` sem diff.

**Objetivo.** Aplicar a nova ordem de navegação, transformar a Visão Geral em
resumo rápido e migrar as análises já existentes para o Quadro de Análises com
seletor de mês.

**Arquivos e componentes.**

- `src/lib/router.ts` + `src/lib/router.test.ts`
- `src/App.svelte` (imports, `pageTitle`, `active()`, nav `:130-186`, dispatch `:274-309`)
- `src/pages/OverviewPage.svelte` (remover `:52-70`, `:82-108`, `:597-1112` e o
  painel de detalhamento de compromissos `:1042-1110`)
- `src/pages/AnalysisPage.svelte` (novo)
- `src/lib/month.ts` (novo: `currentMonthIso`, `shiftMonth`, `monthBounds`)
- `src/lib/components/CategoryChart.svelte` (novo, substitui `CategoryBars.svelte`)
- `src/lib/api.ts`, `src/app.css`

**Regras de negócio.**

- Navegação exatamente na ordem de `2.1`; `#/quadro` é a rota do Quadro;
  `#/relatorios` redireciona para `#/quadro`.
- Visão Geral sem filtros, gráficos, seletor de mês ou detalhamento de
  compromissos; resumo de compromissos permanece.
- Quadro com mês atual por padrão; `from`/`to` derivados do mês e compartilhados
  por todos os widgets.
- Widgets migrados nesta etapa: Fluxo de caixa, Gastos por categoria, Receitas
  por categoria (com toggle barras/rosquinha) e Maiores despesas do mês.
- `category-chart.ts` continua sendo a única fonte de agregação/percentual;
  `CategoryBars.svelte` é substituído, não mantido em paralelo.

**Dependências e riscos.** Mudança do legado `#/relatorios`; queries de
`netWorth`/`commitments` trocam de dono; necessidade de garantir que nenhuma
query da Visão Geral receba o mês selecionado; substituição do componente de
gráfico sem duplicação.

**Testes.**

- `router.test.ts`: nova rota, legado, ordem dos itens e estado ativo.
- TS: helpers de mês, incluindo virada de ano.
- Verificação de que as queries do topo da Visão Geral seguem sem filtro.
- `npm run check`, `npm test`.

**Critérios de conclusão.**

- Sete itens de navegação na ordem definida.
- Visão Geral enxuta, com resumo de compromissos e cinco lançamentos recentes.
- Quadro funcional com os quatro widgets iniciais, todos respondendo ao seletor
  de mês.
- Nenhum painel de filtro complexo permanece na Visão Geral.

---

### Etapa 4 — Novas análises e acabamento do Quadro ✅ CONCLUÍDA

**Status.** Concluída em `435c980a56ebdbd0cf514d313b6491e210a46256`
(`feat: conclui o quadro de analises da v0.2.0`). Incluiu Gastos por período do
dia (acumulado/média diária), Evolução dos gastos mensais e Patrimônio histórico;
polimento visual do Quadro e da Visão Geral; e Motion Design discreto com
respeito a `prefers-reduced-motion`. Validações completas: `npm run check`,
`npm test` (46 testes), `npm run build`, `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` e `npm run bindings` sem diff.

**Objetivo.** Entregar Gastos por período do dia, Evolução dos gastos mensais e
Patrimônio histórico; fechar apresentação e percentuais conforme D1.

**Arquivos e componentes.**

- `crates/tally-app/src/reports.rs` + `crates/tally-app/src/dto.rs`
- `crates/tally-infra/src/reports_repo.rs`
- `src-tauri/src/commands.rs` + `src/lib/api/bindings.ts` (regenerado)
- `src/pages/AnalysisPage.svelte`
- `src/lib/components/PeriodSpendingCard.svelte` (novo)
- `src/lib/components/MonthlyExpenseTrend.svelte` (novo)
- `src/lib/components/NetWorthHistory.svelte` (novo)
- `src/lib/format.ts`, `src/app.css`, `src/lib/category-chart.test.ts`

**Regras de negócio.**

- Período do dia agrupado por `e.period` e `occurred_on`; média por dias ativos
  do período; frequência sobre dias com qualquer despesa no mês.
- Participação percentual no gasto somente na visão de acumulado (D1).
- Sem período identificado → quadro separado, sem redistribuição.
- Evolução dos gastos mensais: mês selecionado + cinco anteriores, competência
  de despesa.
- Patrimônio histórico: último widget; série dos seis meses terminando no mês
  selecionado, ponto final no último dia do mês selecionado; não altera o
  patrimônio atual exibido na Visão Geral.
- Controles de acumulado/média diária dentro do card de período; sem
  personalização de layout.

**Dependências e riscos.** Agregação de dias ativos em Rust (padrão já usado em
`cash_flow`); série mensal exige nova consulta em vez de seis chamadas
independentes; risco de a razão entre médias ser lida como participação no
gasto — mitigada pela decisão D1; `npm run bindings` sem diff.

**Testes.**

- Rust: mês sem dados, período `custom`, contagem de dias ativos, série de seis
  meses com virada de ano, série patrimonial encerrando no mês selecionado.
- TS: formatação de valor/dias/frequência e helpers de mês.
- `npm test`, `npm run check`.

**Critérios de conclusão.**

- Todos os widgets do Quadro respeitam o mês selecionado.
- As duas novas análises e o patrimônio histórico entregues na ordem de `2.3`.
- Média diária mostra valor, dias ativos e frequência; participação percentual
  aparece apenas no acumulado.
- Disposição manual consistente nos temas claro e escuro, sem personalização.

---

### Etapa 5 — Cofrinhos e rendimentos (última etapa)

**Objetivo.** Implementar o suporte a cofrinhos com aplicações, resgates e
rendimentos manuais, preservando a integridade dos relatórios atuais e a regra
de exclusão do patrimônio exibido.

**Status do planejamento.** Regras de produto já fechadas em `2.9`. As decisões
específicas de modelo contábil, local de gerenciamento e apresentação ficam
registradas quando esta etapa começar — **não** foram tomadas neste documento e
não devem ser antecipadas nas etapas 1–4.

**Arquivos e componentes (reservados, a confirmar na etapa).**

- `crates/tally-infra/migrations/0004_cofrinhos.sql` (numerar após `0003`)
- `crates/tally-infra/src/accounts_repo.rs`
- `crates/tally-domain/src/account.rs`
- `crates/tally-app/src/accounts.rs`, `movements.rs`, `dto.rs`
- novo serviço/repositório de cofrinhos, `src-tauri/src/commands.rs`, bindings
- `src/pages/AccountsPage.svelte` ou tela própria (decisão diferida)
- `src/pages/OverviewPage.svelte` (card compacto, decisão diferida)

**Regras de negócio já fixadas.**

- Aplicação e resgate são transferências; nunca despesa ou receita.
- Rendimento é operação específica de cofrinho, separada do principal e de
  receita comum; usar a categoria "Rendimentos" existente se necessário, sem
  criar outra.
- Saldo de cofrinho fora do patrimônio líquido exibido.
- Sem API/sync e sem gestão genérica de investimentos.

**Dependências e riscos a avaliar na etapa.** Validações de `create_transfer`
(não restringe contas sistêmicas), CHECKs de `class`/`is_system`, triggers de
lançamento publicado, edição/exclusão pelo caminho genérico de movimentação,
exclusão de rendimentos de fluxo de caixa e relatórios por categoria, restore de
backup legado e regeneração de bindings.

**Testes a definir na etapa.** Exemplo de referência: contas comuns R$ 2.000,
cofrinho R$ 500,35, passivos R$ 300 → patrimônio exibido R$ 1.700 e total
aplicado R$ 500,35. Aplicação não é despesa; resgate não é receita; rendimento
não entra em receita, despesa ou fluxo de caixa; soma dos postings permanece
equilibrada.

**Critérios de conclusão (a detalhar na etapa).** Operações de cofrinho
funcionando; valores apresentados separadamente; patrimônio exibido inalterado
pela aplicação e pelo rendimento.

**Encerramento (revisão CDI).** A taxa diária passou a vir da SGS 12, com cache
SQLite e refresh exclusivamente manual. Com `p = percentual / 10.000` e `r` em
fração decimal: `f_business = 1 + r × p`; `f_calendar = (1 + r × p)^(252/365)`;
`yield_cents = round(base_cents × (f − 1))`. `business` exige publicação na data;
`calendar` usa a última taxa `<= as_of` e é aproximação matemática do Tally para
365 dias corridos, não convenção contratual. Cada snapshot guarda `rate_date`,
`rate_series`, `cdi_daily_rate` e `rate_fetched_at`; colunas nullable preservam
snapshots antigos. A metodologia é estimativa do Tally, sem garantia de
reproduzir produto financeiro específico.

---

## 5. Ordem, dependências e validação global

Sequência real de dependências: **Etapa 1 → 2 → 3 → 4 → 5**.

- A Etapa 2 vem antes do Quadro para que as legendas já possam usar ícones.
- A Etapa 3 cria rota, navegação, shell e seletor de mês: pré-requisito da 4.
- A Etapa 5 fica isolada no fim e não bloqueia nenhuma das anteriores.

Validação exigida ao final de cada etapa:

```powershell
npm run check
npm test
npm run build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run bindings   # não pode gerar diff em src/lib/api/bindings
```

O fechamento da v0.2.0 também atualiza a versão em `package.json`, `Cargo.toml`
e demais manifestos, gera o changelog/release e publica o instalador via GitHub
Releases — fora das etapas de implementação acima.

---

## 6. Explicitamente adiado

- Análise temporal/evolução de rendimentos — versão futura, após cofrinhos.
- Personalização de dashboard pelo usuário (drag-and-drop, ocultação,
  redimensionamento, temas de widget).
- Gestão genérica de investimentos e integrações com APIs ou corretoras.
- Passe de ônibus e saldos pré-pagos.
- Criação de uma segunda categoria "Rendimentos" — a existente deve ser reutilizada.
