-- Revisão pós-Etapa 5 — ajustes de saldo em cofrinhos.
--
-- 0005 corrigiu a exceção de `opening_balance`. Agora o mesmo trigger precisa
-- aceitar `adjustment`: uma correção de saldo é postagem no cofrinho contra a
-- conta interna de patrimônio, não receita, despesa, transferência ou
-- rendimento. Rendimentos/estornos continuam reconhecidos pelo vínculo em
-- `savings_yields` e imutáveis pelos triggers específicos.

DROP TRIGGER IF EXISTS trg_savings_entry_kind;

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
AND NEW.kind <> 'opening_balance'
AND NEW.kind <> 'adjustment'
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
        'savings accounts only accept transfers or adjustments with a regular account');
END;
