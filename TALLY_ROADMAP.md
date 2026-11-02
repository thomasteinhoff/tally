# Tally — Roadmap & Continuity

> Documento portátil de contexto para continuidade do desenvolvimento do Tally.
>
> **Fonte de verdade:** o código atual do repositório define o que já existe; este documento define decisões, escopo e roadmap. Itens planejados não devem ser tratados como implementados.

## 1. Projeto

**Tally** é um aplicativo desktop de finanças pessoais, local-first e offline.

Objetivo: oferecer um controle financeiro sério, preciso e simples, com comportamento próximo ao de um banco para cartões/parcelamentos, sem depender de nuvem.

### Stack fechada

- Rust
- Tauri 2.x
- Svelte 5 + TypeScript + Vite
- SQLite
- TanStack Query
- Vitest
- ts-rs para DTOs
- Windows como plataforma inicial

Arquitetura:

`Tally.exe → Tauri → Svelte/TypeScript → Tauri Commands → Rust → SQLite`

Princípios:

- Rust é autoridade para regras financeiras e persistência.
- Frontend nunca é autoridade sobre saldos ou persistência.
- Uma mutação = uma transação.
- Sem optimistic success.
- Relatórios leem apenas dados persistidos/postados.
- Sem servidor HTTP local.
- Sem login, nuvem ou sincronização em V1.
- Sem Node/Rust/SQLite instalados pelo usuário final.
- Dados em `%LOCALAPPDATA%\Tally\`.
- BRL apenas em V1.
- pt-BR apenas em V1.

## 2. Estado atual

### Phase 0 — Foundation

**STATUS: COMPLETA.**

Commit da Phase 0:

`477dc4a95d38ced6dbe7f1fc2d16d5fcffb64465`

Mensagem:

`phase 0: foundation complete`

Branch:

`main`

Remote:

`https://github.com/thomasteinhoff/tally.git`

A Phase 0 foi publicada no GitHub.

Validações realizadas:

- `npm run check` sem erros/warnings
- `npm test` passou
- `cargo fmt --all -- --check` passou
- `cargo clippy --workspace --all-targets -- -D warnings` passou
- `cargo test --workspace` passou
- release build executado
- app abriu e leu SQLite real
- migrations/DB/bootstrap funcionando
- integrity check funcionando via IPC
- single-instance confirmado
- installer NSIS gerado

A UI de “Fundação” existente na Phase 0 é temporária. Não é a tela final do produto. Ferramentas de integridade podem permanecer em Configurações.

### Phase 1 — Core Accounting

**STATUS: COMPLETA.**

Mensagem do commit:

`phase 1: core accounting complete`

A Phase 1 foi publicada em `main`.

Entregue:

- contas de ativo com saldo inicial e saldo derivado;
- categorias de despesa/receita;
- despesa, receita, transferência e ajuste;
- períodos derivados pelo banco e período manual quando não há horário;
- lista de movimentações com filtros;
- extrato por conta com filtros, saldo atual e saldos de início/fim por período selecionado;
- edição/exclusão atômicas com confirmação antes da mutation;
- UI real com navegação por hash, formulários, loading, erros, confirmações destrutivas e explicações de arquivamento;
- testes financeiros de integração e testes de utilidades da UI.

Validações realizadas:

- `npm run check` sem erros/warnings
- `npm test` passou
- `cargo fmt --all -- --check` passou
- `cargo clippy --workspace --all-targets -- -D warnings` passou
- `cargo test --workspace` passou
- `npm run build` passou
- `npm run tauri build -- --no-bundle` passou
- smoke test do `tally.exe` iniciou e permaneceu aberto

### Próximo trabalho

**Phase 2 — Cards & Installments**

Não iniciar Phase 2 antes de versionar e publicar a Phase 1.

---

# 3. Arquitetura e decisões fechadas

## 3.1 Contabilidade

O núcleo usa **double-entry bookkeeping** internamente.

O usuário não precisa enxergar débito/crédito no fluxo normal.

Toda movimentação financeira importante é representada por lançamentos balanceados.

Exemplo conceitual:

- despesa: dinheiro sai de uma conta e vira despesa;
- receita: receita entra e aumenta patrimônio;
- transferência: sai de uma conta e entra em outra;
- compra no cartão: aumenta dívida do cartão;
- pagamento de cartão: reduz dívida e reduz caixa;
- ajuste: corrige patrimônio sem virar receita/despesa.

Saldos não são armazenados como verdade independente: são derivados dos postings.

## 3.2 Contas

Conceito unificado de conta:

- banco
- dinheiro/carteira
- poupança
- investimento
- carteira digital
- outro ativo
- cartão de crédito

Classes internas:

- asset
- liability
- equity
- income
- expense

Cartão é **liability**.

Limite de crédito é apenas informacional; não é ativo e não entra no patrimônio líquido.

Saldo negativo é permitido, com aviso.

Conta pode ser arquivada sem apagar histórico.

Renomear conta preserva o ID e o histórico.

Saldo inicial é obrigatório na criação de contas.

## 3.3 Categorias

Lista plana.

Cada categoria tem:

- nome
- tipo: `expense` ou `income`
- archived_at

Criar, renomear e arquivar.

Excluir apenas quando não houver histórico; caso contrário, arquivar.

Não criar hierarquia de categorias em V1.

## 3.4 Métodos de pagamento

V1:

- Pix
- Débito
- Crédito
- Assinatura (Crédito)
- Dinheiro

“Assinatura (Crédito)” é apenas método de pagamento em V1. Entidade de assinatura recorrente fica para V2.

## 3.5 Períodos

Períodos fechados:

- Madrugada: 00:00–04:59
- Manhã: 05:00–11:59
- Tarde: 12:00–18:59
- Noite: 19:00–23:59
- Customizada: exceção manual

Se houver horário, o período é derivado automaticamente.

Se não houver horário, o usuário precisa informar o período manual.

Se o horário mudar, o período é recalculado.

Parcelas herdam o período da compra, não o período da data de vencimento.

BRL e datas são locais/pt-BR; não usar UTC para alterar a competência financeira.

## 3.6 Transferências

Transferência é asset-to-asset.

Não é receita nem despesa.

É neutra no patrimônio líquido.

Aparece nos dois extratos.

Campos relevantes:

- saiu de
- entrou em
- valor
- data
- horário/período
- descrição opcional

Transferência para cartão não é transferência: é pagamento de cartão.

## 3.7 Cartão de crédito

Cartão é passivo.

V1 **não possui ciclo completo de fatura**:

- sem fechamento
- sem vencimento automático de fatura
- sem juros
- sem rotativo
- sem IOF
- sem anuidade automática

Rótulos preferidos:

- Dívida total
- Deste mês
- Futuro
- Vencido

Não usar “fatura” como conceito central em V1.

Compra no cartão aumenta a dívida.

Pagamento do cartão:

- reduz dívida do cartão;
- reduz caixa da conta bancária;
- não cria outra despesa.

Pagamento parcial é permitido.

Pagamento acima da dívida não deve criar despesa.

Alocação de pagamento usa FIFO:

1. saldo inicial do cartão;
2. parcelas mais antigas;
3. demais parcelas por ordem de vencimento.

“Disponível no limite” = limite - dívida, quando houver limite definido.

Ultrapassar limite é permitido com aviso.

## 3.8 Parcelamentos — regra crítica

O comportamento deve parecer com banco.

Exemplo:

`R$ 900 em 3x de R$ 300`

O sistema guarda:

- valor total da compra;
- quantidade de parcelas;
- valor de cada parcela;
- primeira data de vencimento;
- plano da compra;
- categoria;
- método;
- período;
- descrição.

A compra total identifica o compromisso, mas **não vira uma despesa mensal de R$900**.

A despesa reconhecida é uma parcela por vez.

### Modelo interno

Usar a conta interna:

**“Despesa a reconhecer”**

Na compra:

- débito em Despesa a reconhecer: R$900
- crédito no Cartão: R$900

Resultado:

- dívida do cartão +R$900
- patrimônio líquido neutro no momento da compra

Quando cada parcela é reconhecida:

- débito em Despesa: R$300
- crédito em Despesa a reconhecer: R$300

Assim:

- mês 1 reconhece R$300;
- mês 2 reconhece R$300;
- mês 3 reconhece R$300.

Isso resolve simultaneamente:

- dívida total imediata do cartão;
- despesa por competência/parcela;
- patrimônio líquido correto;
- ausência de dupla contagem.

**Consequência importante:** o patrimônio líquido não cai R$900 na compra; ele cai conforme as parcelas são reconhecidas.

### Compromissos futuros

“Compromissos futuros” são parcelas não pagas com vencimento posterior ao fim do período atual.

Eles já estão dentro da dívida total do cartão.

Nunca somar:

`dívida total + compromissos futuros`

como se fossem duas dívidas.

### Arredondamento

O total das parcelas deve bater exatamente com o total da compra.

Para divisões que geram centavos:

- distribuir o resto de centavo nas primeiras parcelas;
- soma das parcelas = total exato;
- cada parcela >= 1 centavo;
- quantidade entre 1 e 60.

### Datas

Preservar o dia da primeira parcela quando possível.

Se o mês não tiver esse dia, usar o último dia do mês.

Exemplo:

`31/01 → 28/02 → 31/03`

A primeira data de vencimento é informada pelo usuário/fluxo de compra. V1 não possui lógica de fechamento de fatura.

### Parcela atual

A parcela atual é a próxima parcela não paga por data de vencimento.

Despesa de parcela é reconhecida pela data da parcela, não pela data em que o cartão foi pago.

Pagamento de parcela nunca cria outra despesa.

---

# 4. Patrimônio e relatórios

## Despesa

`kind = expense` no período.

Inclui parcelas reconhecidas.

Não inclui o total bruto da compra parcelada.

## Receita

`kind = income` no período.

## Resultado

`receita - despesa`

É resultado por competência, não fluxo de caixa.

## Transferência

Não é receita/despesa.

Patrimônio líquido neutro.

## Pagamento de cartão

Não é despesa.

É saída de caixa e redução de passivo.

Patrimônio líquido neutro.

## Patrimônio líquido

`ativos - passivos`

Inclui o comportamento da conta interna “Despesa a reconhecer”.

## Fluxo de caixa

Baseado na variação efetiva das contas de caixa/ativos.

Transferências internas cancelam no consolidado.

Pagamento de cartão é saída de caixa.

## Saldo inicial

Não é receita nem despesa.

## Ajuste

Corrige patrimônio/equity.

Não deve aparecer como receita ou despesa normal.

---

# 5. Modelo de dados fechado

Tabelas principais:

### `accounts`

Contas e contas de sistema.

Campos relevantes:

- UUIDv7
- nome
- classe
- tipo
- `is_system`
- `credit_limit_cents`
- `archived_at`

Nome único sem diferenciar maiúsculas/minúsculas.

Cartão deve ser liability.

### `categories`

- nome
- kind: expense/income
- archived_at

Unique por `(name, kind)`.

### `journal_entries`

Representa eventos.

Inclui:

- kind
- occurred_on
- occurred_time
- manual_period
- generated period
- description
- primary_account_id
- counterparty_account_id
- category_id
- payment_method
- posted_at

### `postings`

- entry_id
- account_id
- amount_cents

Para cada lançamento postado:

- pelo menos 2 postings;
- soma dos postings = zero;
- valores não zero.

### `installment_plans`

Inclui:

- card_account_id
- purchase_entry_id
- category_id
- total_cents
- installment_count
- first_due_on
- period
- payment_method

### `installments`

Inclui:

- plan_id
- seq
- due_on
- amount_cents
- recognition_entry_id

### `card_payments`

Inclui:

- card_account_id
- bank_account_id
- entry_id
- amount_cents

### `installment_allocations`

- payment_id
- installment_id
- amount_cents

PK composta.

### `settings`

`key/value`.

### Contas internas

Existem contas de sistema para suportar o ledger, incluindo “Despesa a reconhecer”.

Essas contas não devem aparecer como contas normais para o usuário.

---

# 6. Regras de persistência

- SQLite.
- Foreign keys habilitadas.
- WAL.
- `synchronous = FULL`.
- `busy_timeout = 5000`.
- migrations embutidas.
- backup antes de migration.
- falha de migration não deve abrir o banco em modo de escrita.
- integridade baseada no SQLite.
- postings postados são imutáveis diretamente.
- edição de lançamento: unpost → alterar → repost, atomicamente.
- relatórios leem somente lançamentos postados.
- toda mutation deve ser transacional.

Entradas começam como draft e só recebem `posted_at` depois que os postings estão válidos.

---

# 7. Comandos Tauri

## Bootstrap

- `get_bootstrap`
- `get_settings`
- `update_settings`
- `run_integrity_check`

## Contas

- `list_accounts`
- `create_account`
- `update_account`
- `set_account_archived`
- `delete_account`

## Categorias

- `list_categories`
- `create_category`
- `update_category`
- `set_category_archived`
- `delete_category`

## Movimentações

- `list_movements`
- `create_expense`
- `create_income`
- `create_transfer`
- `create_adjustment`
- `update_movement`
- `delete_movement`

## Cartões / parcelamentos

- `create_card_purchase`
- `list_installment_plans`
- `get_installment_plan`
- `update_installment_plan`
- `delete_installment_plan`
- `create_card_payment`
- `update_card_payment`
- `delete_card_payment`
- `get_card_summary`

## Relatórios

- `get_dashboard`
- `get_spending_report`
- `get_income_report`
- `get_cash_flow_report`
- `get_net_worth_report`
- `get_commitments_report`

## Dados

- `create_backup`
- `list_backups`
- `restore_backup`
- `export_csv`

Toda mutation retorna a entidade atualizada depois do commit.

---

# 8. Erros

Formato:

```text
AppError {
  code,
  message,
  details
}
```

Códigos esperados:

- VALIDATION
- NOT_FOUND
- CONFLICT
- DB
- IO
- INTEGRITY

Nunca mostrar SQL bruto na UI.

Nunca engolir erro.

Frontend deve preservar o formulário quando uma mutation falhar.

Sucesso só depois do retorno bem-sucedido do comando.

---

# 9. UI / design

Princípio:

**Precisão antes de decoração.**

Produto deve parecer software financeiro sério, não dashboard SaaS genérico.

Evitar:

- gradientes
- glassmorphism
- excesso de cards
- sombras grandes
- pills exageradas
- emojis
- gamificação
- decoração sem função
- visual “AI-generated”

Preferir:

- tipografia forte
- números protagonistas
- tabelas densas
- alinhamento rigoroso
- divisórias sutis
- estados sem ambiguidade
- espaços consistentes

Identidade planejada:

- warm paper
- ink
- oxide
- IBM Plex Sans
- IBM Plex Mono para números/metadata

BRL:

`R$ 1.234,56`

Números monetários alinhados à direita e com tabular numerals.

Sem gráficos na V1.

## Rotas

- `/` — visão geral
- `/movimentacoes`
- `/contas`
- `/contas/:id`
- `/cartoes`
- `/cartoes/:id`
- `/parcelamentos`
- `/parcelamentos/:id`
- `/relatorios`
- `/categorias`
- `/configuracoes`

## Nova movimentação

Um único botão com:

- Despesa
- Receita
- Transferência
- Compra no cartão
- Pagamento de cartão
- Ajuste

Data vem preenchida, mas é editável.

Horário é opcional.

Sem horário → período manual obrigatório.

## UX

- erros inline + toast quando apropriado;
- loading/disabled durante mutation;
- confirmação destrutiva com consequência explícita, antes de qualquer mutation;
- cancelar confirmação não altera dados;
- Ctrl+N;
- Enter;
- Esc;
- foco correto;
- acessibilidade;
- labels reais;
- contraste;
- focus visível;
- aria-live para erros/toasts.

---

# 10. Edição e exclusão

## Movimentação simples

Despesa, receita, transferência e ajuste podem ser editados/excluídos atomicamente.

Sem audit/history em V1.

## Compra parcelada

Sem pagamentos:

pode editar:

- total
- quantidade
- primeira data
- categoria
- período
- descrição

Com pagamentos:

- somente descrição.

Delete do plano:

- permitido apenas sem pagamentos;
- caso tenha pagamentos, pagamentos precisam ser removidos primeiro.

## Pagamento de cartão

Editar/excluir recalcula as alocações FIFO.

Excluir pagamento reabre parcelas quando necessário.

## Saldo inicial

Não editar pela lista de movimentações.

Correção deve ser feita via ajuste.

## Entradas internas

Somente comandos agregados próprios podem manipulá-las.

---

# 11. V1 vs V2

## V1

- contas
- saldo inicial
- categorias
- despesa
- receita
- transferência
- ajuste
- cartões
- limite de cartão
- compra parcelada
- pagamento de cartão
- períodos
- parcelamentos
- compromissos futuros
- extratos
- dashboard
- relatórios
- backup
- restore
- CSV
- integrity check
- UI desktop
- teclado
- erros reais
- confirmações destrutivas
- installer Windows offline

## V2

Não implementar antecipadamente.

- audit/version history
- void/replace
- reversals
- ciclo completo de fatura
- vencimento/fechamento automático
- assinatura recorrente como entidade
- retorno/valuation de investimentos
- gráficos avançados
- command palette/polish
- importação CSV/OFX
- criptografia/login
- auto-update
- assinatura de software
- dark mode
- multi-moeda
- budgets
- tags
- anexos
- mobile/web

---

# 12. Fases

## Phase 0 — Foundation

**STATUS: COMPLETA**

Objetivo:

- workspace Rust
- Tauri shell
- Svelte/TS
- SQLite
- migrations
- bootstrap
- settings
- paths
- logging
- IPC
- errors
- CI
- installer base
- single instance

DoD:

- app abre;
- DB é criado/migrado;
- bootstrap retorna dados reais;
- integrity check funciona;
- testes passam;
- build de release funciona.

## Phase 1 — Core Accounting

**STATUS: COMPLETA**

Objetivo: implementar o ledger financeiro básico e tornar a aplicação realmente utilizável sem cartões/parcelamentos.

Sequência recomendada:

1. contas CRUD + archive + opening balance;
2. categorias CRUD + archive;
3. journal entries/postings;
4. despesa;
5. receita;
6. transferência;
7. ajuste;
8. regras de período;
9. lista de movimentações;
10. extrato por conta;
11. saldos derivados;
12. formulários e estados de erro/loading;
13. testes financeiros;
14. substituir a tela temporária de Fundação por navegação real.

Testes mínimos:

- transferência é neutra;
- saldo inicial não é receita;
- período 00:00;
- período 04:59;
- período 05:00;
- período 11:59;
- período 12:00;
- período 18:59;
- período 19:00;
- período manual sem horário;
- renomear conta preserva histórico;
- categoria arquivada preserva histórico;
- ledger sempre balanceia;
- rollback de mutation em erro;
- saldo derivado dos postings.

DoD:

- usuário consegue criar conta;
- informa saldo inicial;
- cria categoria;
- registra despesa;
- registra receita;
- transfere entre contas;
- faz ajuste;
- visualiza movimentações;
- visualiza extrato;
- saldos estão corretos;
- regras financeiras estão cobertas por testes;
- UI não depende de estado financeiro inventado no frontend.

## Phase 2 — Cards & Installments

**STATUS: COMPLETA**

Objetivo:

- cartões;
- limite;
- compras parceladas;
- reconhecimento por parcela;
- compromissos futuros;
- pagamentos FIFO;
- pagamentos parciais;
- telas de cartão;
- telas de parcelamento.

Testes críticos:

- R$900 em 3x R$300;
- total da compra ≠ despesa mensal;
- dívida total correta;
- patrimônio líquido correto;
- pagamento não vira despesa;
- FIFO;
- saldo inicial do cartão antes das parcelas;
- pagamento parcial;
- exclusão de pagamento reabre parcela;
- arredondamento;
- datas de fim de mês;
- over-limit;
- compromissos futuros não duplicam dívida;
- parcela reconhecida uma única vez.

DoD:

- comportamento de cartão/parcelamento consistente com as regras deste documento;
- sem dupla contagem;
- relatórios básicos continuam corretos.

## Phase 3 — Reports & Data

Objetivo:

- dashboard;
- gastos;
- receitas;
- fluxo de caixa;
- patrimônio líquido;
- compromissos;
- filtros;
- CSV;
- backup;
- restore;
- settings;
- integrity tools.

Testes:

- fórmulas;
- filtros;
- encoding CSV;
- backup/restore;
- migration;
- dados restaurados preservam integridade.

DoD:

- usuário consegue entender a situação financeira;
- exportar dados;
- fazer backup;
- restaurar backup;
- verificar integridade.

## Phase 4 — Finish & Distribution

Objetivo:

- refinamento visual;
- componentes;
- tabelas;
- loading/error/empty states;
- teclado;
- acessibilidade;
- onboarding;
- about;
- logs;
- installer final;
- smoke tests em Windows limpo.

Installer:

- NSIS per-user;
- WebView2 offline;
- sem Node/Rust/SQLite para usuário final;
- dados permanecem após uninstall.

Smoke test:

- instalar em Windows limpo;
- abrir pela primeira vez;
- criar dados;
- fechar/reabrir;
- atualizar versão;
- preservar dados;
- desinstalar;
- confirmar preservação de dados.

DoD:

- Tally utilizável como desktop app real;
- installer confiável;
- dados preservados;
- testes automatizados passando;
- checklist manual passando.

---

# 13. Testes financeiros

Usar centavos inteiros.

Não usar floating point para dinheiro.

Um golden ledger útil:

Estado inicial:

- banco: R$1.000
- dívida inicial do cartão: R$200
- patrimônio líquido: R$800

Eventos:

1. despesa em dinheiro R$100:
   - banco R$900
   - dívida cartão R$200
   - patrimônio R$700

2. transferência banco → poupança R$500:
   - banco R$400
   - poupança R$500
   - dívida R$200
   - patrimônio R$700

3. compra no cartão R$500:
   - banco R$400
   - poupança R$500
   - dívida R$700
   - compra reconhecida conforme regra da modalidade

4. pagamento de cartão R$300:
   - banco R$100
   - dívida R$400
   - patrimônio não muda

5. compra parcelada R$1.200 em 12x:
   - dívida aumenta R$1.200
   - caixa não muda
   - reconhecimento ocorre por parcela

6. pagamento de uma parcela de R$100:
   - banco vai a R$0
   - dívida reduz R$100
   - patrimônio não muda pelo pagamento.

Sempre validar:

`sum(postings) = 0`

e:

`net worth = assets - liabilities`

---

# 14. Regras para o agent

Ao continuar o projeto:

1. Leia este `TALLY_ROADMAP.md`.
2. Depois inspecione o código real do repositório.
3. Código existente é a fonte de verdade sobre implementação.
4. Este documento é a fonte de verdade sobre decisões, escopo e roadmap.
5. Não assuma que planejado já existe.
6. Não reabra decisões arquiteturais já fechadas sem motivo concreto.
7. Não implementar V2 antecipadamente.
8. Não adicionar funcionalidades fora do escopo.
9. Não simplificar regras financeiras sem justificar.
10. Rust é autoridade financeira.
11. Frontend não é autoridade sobre saldos.
12. Não usar optimistic success.
13. Toda mutation deve ser transacional.
14. Manter testes para regras financeiras.
15. Validar antes de declarar uma fase concluída.
16. Atualizar o status deste roadmap conforme as fases avançarem.
17. Evitar prompts gigantes e mudanças abrangentes desnecessárias.
18. Preferir mudanças incrementais, testáveis e revisáveis.

## Importante sobre contexto

Não é necessário carregar o histórico completo da sessão do OpenCode para continuar.

Use:

- GitHub/repositório → implementação;
- `TALLY_ROADMAP.md` → decisões e plano;
- export da sessão → backup/consulta, se necessário.

Não copie a sessão inteira para contexto sem necessidade.

---

# 15. Próximo passo exato

**Implementar somente a Phase 2 — Cards & Installments.**

Começar pelo backend/domínio e pelos testes financeiros, depois conectar as telas.

Não começar relatórios avançados, distribuição ou funcionalidades V2 antes de concluir cartões e parcelamentos.

A primeira meta é:

> Tally consegue registrar cartões, compras à vista e parceladas, compromissos futuros, pagamentos FIFO/parciais e reconhecimento por parcela com ledger balanceado, dívida e patrimônio corretos e testes financeiros confiáveis.

