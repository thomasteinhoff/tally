-- Etapa 5 — corrige o trigger de cofrinhos em bancos já migrados para 0004.
--
-- A regra de abertura foi acrescentada a 0004 durante o desenvolvimento. Um
-- banco que já tinha aplicado 0004 não reaplica o arquivo alterado e mantém o
-- trigger antigo, que rejeita o entry `opening_balance` do saldo inicial.
-- Recria o trigger com a exceção, sem afrouxar as demais regras de cofrinho.

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
