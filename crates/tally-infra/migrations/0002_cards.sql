-- Phase 2 — cartões e parcelamentos.
--
-- O pagamento FIFO precisa registrar quanto abateu a dívida inicial do cartão,
-- que não é uma parcela. As parcelas abatidas ficam em installment_allocations.
-- Excluir o pagamento remove as alocações e reabre a dívida automaticamente.

ALTER TABLE card_payments
    ADD COLUMN initial_balance_cents INTEGER NOT NULL DEFAULT 0
    CHECK (initial_balance_cents >= 0);
