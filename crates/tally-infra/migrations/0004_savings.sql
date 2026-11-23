-- Etapa 5 — cofrinhos, rendimentos e estornos.
--
-- Um cofrinho é uma conta de ativo marcada por `accounts.is_savings`.
-- A modalidade de cálculo e o percentual contratado são persistidos na própria
-- conta; os parâmetros usados em cada rendimento são copiados no snapshot, de
-- modo que rendimentos já confirmados nunca dependem da configuração atual.

ALTER TABLE accounts ADD COLUMN is_savings INTEGER NOT NULL DEFAULT 0;
ALTER TABLE accounts ADD COLUMN yield_mode TEXT;
ALTER TABLE accounts ADD COLUMN cdi_percent_bps INTEGER;

-- Só cofrinhos podem ter modalidade/percentual; um cofrinho é sempre ativo,
-- não cartão, e precisa das duas configurações.
CREATE TRIGGER trg_accounts_savings_shape_insert
BEFORE INSERT ON accounts
WHEN (NEW.is_savings = 1 AND (
        NEW.class <> 'asset'
        OR NEW.type IS NULL
        OR NEW.type = 'credit_card'
        OR NEW.yield_mode NOT IN ('calendar', 'business')
        OR NEW.cdi_percent_bps IS NULL
        OR NEW.cdi_percent_bps <= 0
     ))
  OR (NEW.is_savings = 0 AND (
        NEW.yield_mode IS NOT NULL OR NEW.cdi_percent_bps IS NOT NULL
     ))
BEGIN
    SELECT RAISE(ABORT, 'invalid savings account settings');
END;

CREATE TRIGGER trg_accounts_savings_shape_update
BEFORE UPDATE OF is_savings, yield_mode, cdi_percent_bps, class, type ON accounts
WHEN (NEW.is_savings = 1 AND (
        NEW.class <> 'asset'
        OR NEW.type IS NULL
        OR NEW.type = 'credit_card'
        OR NEW.yield_mode NOT IN ('calendar', 'business')
        OR NEW.cdi_percent_bps IS NULL
        OR NEW.cdi_percent_bps <= 0
     ))
  OR (NEW.is_savings = 0 AND (
        NEW.yield_mode IS NOT NULL OR NEW.cdi_percent_bps IS NOT NULL
     ))
BEGIN
    SELECT RAISE(ABORT, 'invalid savings account settings');
END;

-- Um cofrinho nunca vira conta comum, nem uma conta comum vira cofrinho.
CREATE TRIGGER trg_accounts_savings_immutable
BEFORE UPDATE OF is_savings ON accounts
WHEN OLD.is_savings <> NEW.is_savings
BEGIN
    SELECT RAISE(ABORT, 'savings flag is immutable');
END;

CREATE TABLE savings_yields (
    id                     TEXT PRIMARY KEY NOT NULL,
    account_id             TEXT NOT NULL REFERENCES accounts(id),
    as_of                  TEXT NOT NULL CHECK (
                               date(as_of) IS NOT NULL AND date(as_of) = as_of
                           ),
    mode                   TEXT NOT NULL CHECK (mode IN ('calendar', 'business')),
    cdi_annual_bps         INTEGER NOT NULL CHECK (cdi_annual_bps > 0),
    contracted_percent_bps INTEGER NOT NULL CHECK (contracted_percent_bps > 0),
    daily_rate             REAL NOT NULL CHECK (daily_rate > 0),
    days                   INTEGER NOT NULL CHECK (days >= 1),
    base_cents             INTEGER NOT NULL,
    yield_cents            INTEGER NOT NULL CHECK (yield_cents > 0),
    status                 TEXT NOT NULL CHECK (status IN ('confirmed', 'reversed')),
    -- FKs deferidas: o snapshot é gravado antes do lançamento para que os
    -- triggers de cofrinho reconheçam os lançamentos de rendimento/estorno.
    entry_id               TEXT NOT NULL UNIQUE REFERENCES journal_entries(id)
                               DEFERRABLE INITIALLY DEFERRED,
    reversal_entry_id      TEXT UNIQUE REFERENCES journal_entries(id)
                               DEFERRABLE INITIALLY DEFERRED,
    created_at             TEXT NOT NULL,
    updated_at             TEXT NOT NULL,

    CHECK (status = 'confirmed' OR reversal_entry_id IS NOT NULL)
);

-- Um rendimento ativo por cofrinho por data. Estornar muda o status para
-- `reversed` e libera a data para um novo rendimento.
CREATE UNIQUE INDEX ux_savings_yields_confirmed
    ON savings_yields(account_id, as_of)
    WHERE status = 'confirmed';

CREATE INDEX ix_savings_yields_account
    ON savings_yields(account_id, as_of);

CREATE INDEX ix_savings_yields_entry
    ON savings_yields(entry_id);

CREATE INDEX ix_savings_yields_reversal
    ON savings_yields(reversal_entry_id);

-- Cofrinhos só aceitam transferências com uma conta convencional. Rendimentos
-- e estornos são reconhecidos pelo vínculo em `savings_yields`.
CREATE TRIGGER trg_savings_entry_kind
BEFORE INSERT ON journal_entries
WHEN (
        (SELECT is_savings FROM accounts WHERE id = NEW.primary_account_id) = 1
        OR (
            NEW.counterparty_account_id IS NOT NULL
            AND (SELECT is_savings FROM accounts
                 WHERE id = NEW.counterparty_account_id) = 1
        )
     )
AND NOT EXISTS (
        SELECT 1 FROM savings_yields sy
        WHERE sy.entry_id = NEW.id OR sy.reversal_entry_id = NEW.id
     )
AND (
        NEW.kind <> 'transfer'
        OR NEW.counterparty_account_id IS NULL
        OR (
             (SELECT is_savings FROM accounts
              WHERE id = NEW.primary_account_id) = 1
            AND (SELECT is_savings FROM accounts
              WHERE id = NEW.counterparty_account_id) = 1
        )
        OR EXISTS (
            SELECT 1 FROM accounts
            WHERE id IN (NEW.primary_account_id, NEW.counterparty_account_id)
              AND is_savings = 1 AND archived_at IS NOT NULL
        )
     )
BEGIN
    SELECT RAISE(ABORT,
        'savings accounts only accept transfers with a regular account');
END;

-- Nenhum lançamento pode deixar um cofrinho com saldo negativo. A publicação
-- é o ponto seguro: todas as postings já foram gravadas nesse momento.
CREATE TRIGGER trg_savings_balance_nonnegative
BEFORE UPDATE OF posted_at ON journal_entries
WHEN NEW.posted_at IS NOT NULL AND OLD.posted_at IS NULL
BEGIN
    SELECT CASE
        WHEN EXISTS (
            SELECT 1 FROM accounts a
            WHERE a.is_savings = 1
              AND (
                  SELECT COALESCE(SUM(p.amount_cents), 0)
                  FROM postings p
                  JOIN journal_entries e ON e.id = p.entry_id
                  WHERE p.account_id = a.id
                    AND (e.posted_at IS NOT NULL OR e.id = NEW.id)
              ) < 0
        )
        THEN RAISE(ABORT, 'savings balance cannot be negative')
    END;
END;

-- Lançamentos de rendimento/estorno não podem ser despublicados, editados ou
-- excluídos pelo caminho genérico de movimentações.
CREATE TRIGGER trg_savings_yield_entry_update_guard
BEFORE UPDATE ON journal_entries
WHEN OLD.posted_at IS NOT NULL
 AND EXISTS (
        SELECT 1 FROM savings_yields sy
        WHERE sy.entry_id = OLD.id OR sy.reversal_entry_id = OLD.id
     )
BEGIN
    SELECT RAISE(ABORT, 'savings yield entries are immutable');
END;

CREATE TRIGGER trg_savings_yield_entry_delete_guard
BEFORE DELETE ON journal_entries
WHEN EXISTS (
        SELECT 1 FROM savings_yields sy
        WHERE sy.entry_id = OLD.id OR sy.reversal_entry_id = OLD.id
     )
BEGIN
    SELECT RAISE(ABORT, 'savings yield entries cannot be deleted');
END;
