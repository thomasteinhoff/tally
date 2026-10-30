-- Tally V1 — schema inicial.
--
-- Regras estruturais:
--   * todo lançamento é um JournalEntry com duas ou mais postings;
--   * postings somam zero quando o lançamento é publicado;
--   * contas e categorias usam IDs estáveis;
--   * saldos, dívida de cartão e patrimônio são projeções, nunca colunas persistidas.

CREATE TABLE accounts (
    id                  TEXT PRIMARY KEY NOT NULL,
    name                TEXT NOT NULL COLLATE NOCASE,
    class               TEXT NOT NULL CHECK (class IN ('asset', 'liability', 'equity', 'income', 'expense')),
    type                TEXT CHECK (type IN ('bank', 'cash', 'savings', 'investment', 'digital_wallet', 'other_asset', 'credit_card')),
    is_system           INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
    credit_limit_cents  INTEGER CHECK (credit_limit_cents IS NULL OR credit_limit_cents >= 0),
    archived_at         TEXT,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL,

    CHECK (
        (is_system = 0 AND class IN ('asset', 'liability') AND type IS NOT NULL)
        OR
        (is_system = 1 AND class IN ('asset', 'equity', 'income', 'expense') AND type IS NULL)
    ),
    CHECK (type IS NULL OR type <> 'credit_card' OR class = 'liability'),
    CHECK (credit_limit_cents IS NULL OR type = 'credit_card'),
    CHECK (is_system = 0 OR archived_at IS NULL)
);

CREATE UNIQUE INDEX ux_accounts_name
    ON accounts(name)
    WHERE is_system = 0;

CREATE INDEX ix_accounts_class
    ON accounts(class, type);

CREATE TABLE categories (
    id          TEXT PRIMARY KEY NOT NULL,
    name        TEXT NOT NULL COLLATE NOCASE,
    kind        TEXT NOT NULL CHECK (kind IN ('expense', 'income')),
    archived_at TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE UNIQUE INDEX ux_categories_name_kind
    ON categories(name, kind);

CREATE TABLE journal_entries (
    id                      TEXT PRIMARY KEY NOT NULL,
    kind                    TEXT NOT NULL CHECK (kind IN (
                                'expense', 'income', 'transfer', 'card_purchase',
                                'card_payment', 'opening_balance', 'adjustment'
                            )),
    occurred_on             TEXT NOT NULL CHECK (
                                date(occurred_on) IS NOT NULL
                                AND date(occurred_on) = occurred_on
                            ),
    occurred_time           TEXT CHECK (
                                occurred_time IS NULL
                                OR (
                                    occurred_time GLOB '[0-2][0-9]:[0-5][0-9]'
                                    AND occurred_time < '24:00'
                                )
                            ),
    manual_period           TEXT CHECK (
                                manual_period IS NULL
                                OR manual_period IN ('madrugada', 'manha', 'tarde', 'noite', 'custom')
                            ),
    period                  TEXT GENERATED ALWAYS AS (
                                CASE
                                    WHEN occurred_time IS NULL THEN manual_period
                                    WHEN occurred_time < '05:00' THEN 'madrugada'
                                    WHEN occurred_time < '12:00' THEN 'manha'
                                    WHEN occurred_time < '19:00' THEN 'tarde'
                                    ELSE 'noite'
                                END
                            ) STORED,
    description             TEXT,
    primary_account_id      TEXT NOT NULL REFERENCES accounts(id),
    counterparty_account_id TEXT REFERENCES accounts(id),
    category_id             TEXT REFERENCES categories(id),
    payment_method          TEXT CHECK (
                                payment_method IS NULL
                                OR payment_method IN ('pix', 'debit', 'credit', 'subscription_credit', 'cash')
                            ),
    posted_at               TEXT,
    created_at              TEXT NOT NULL,
    updated_at              TEXT NOT NULL,

    CHECK (
        (
            kind IN ('expense', 'income', 'transfer', 'card_payment')
            AND (
                (occurred_time IS NULL AND manual_period IS NOT NULL)
                OR
                (occurred_time IS NOT NULL AND manual_period IS NULL)
            )
        )
        OR
        (
            kind IN ('card_purchase', 'opening_balance', 'adjustment')
            AND occurred_time IS NULL
            AND manual_period IS NULL
        )
    ),
    CHECK ((kind IN ('transfer', 'card_payment')) = (counterparty_account_id IS NOT NULL)),
    CHECK ((kind IN ('expense', 'income')) = (category_id IS NOT NULL)),
    CHECK ((kind IN ('expense', 'income', 'card_purchase')) = (payment_method IS NOT NULL)),
    CHECK (kind <> 'card_purchase' OR payment_method IN ('credit', 'subscription_credit')),
    CHECK (kind <> 'transfer' OR primary_account_id <> counterparty_account_id),
    CHECK (kind <> 'card_payment' OR primary_account_id <> counterparty_account_id)
);

CREATE INDEX ix_entries_date
    ON journal_entries(occurred_on)
    WHERE posted_at IS NOT NULL;

CREATE INDEX ix_entries_kind_date
    ON journal_entries(kind, occurred_on)
    WHERE posted_at IS NOT NULL;

CREATE INDEX ix_entries_account_date
    ON journal_entries(primary_account_id, occurred_on)
    WHERE posted_at IS NOT NULL;

CREATE INDEX ix_entries_category_date
    ON journal_entries(category_id, occurred_on)
    WHERE posted_at IS NOT NULL;

CREATE INDEX ix_entries_method_date
    ON journal_entries(payment_method, occurred_on)
    WHERE posted_at IS NOT NULL;

CREATE INDEX ix_entries_period_date
    ON journal_entries(period, occurred_on)
    WHERE posted_at IS NOT NULL;

CREATE TABLE postings (
    id           TEXT PRIMARY KEY NOT NULL,
    entry_id     TEXT NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
    account_id   TEXT NOT NULL REFERENCES accounts(id),
    amount_cents INTEGER NOT NULL CHECK (amount_cents <> 0),
    created_at   TEXT NOT NULL
);

CREATE INDEX ix_postings_entry
    ON postings(entry_id);

CREATE INDEX ix_postings_account
    ON postings(account_id);

CREATE TABLE installment_plans (
    id                TEXT PRIMARY KEY NOT NULL,
    card_account_id   TEXT NOT NULL REFERENCES accounts(id),
    purchase_entry_id TEXT NOT NULL UNIQUE REFERENCES journal_entries(id),
    category_id       TEXT NOT NULL REFERENCES categories(id),
    description       TEXT,
    total_cents       INTEGER NOT NULL CHECK (total_cents > 0),
    installment_count INTEGER NOT NULL CHECK (installment_count BETWEEN 1 AND 60),
    first_due_on      TEXT NOT NULL CHECK (
                          date(first_due_on) IS NOT NULL
                          AND date(first_due_on) = first_due_on
                      ),
    period            TEXT NOT NULL CHECK (period IN ('madrugada', 'manha', 'tarde', 'noite', 'custom')),
    payment_method    TEXT NOT NULL CHECK (payment_method IN ('credit', 'subscription_credit')),
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,

    CHECK (installment_count <= total_cents)
);

CREATE INDEX ix_installment_plans_card
    ON installment_plans(card_account_id);

CREATE TABLE installments (
    id                  TEXT PRIMARY KEY NOT NULL,
    plan_id             TEXT NOT NULL REFERENCES installment_plans(id) ON DELETE CASCADE,
    seq                 INTEGER NOT NULL CHECK (seq >= 1),
    due_on              TEXT NOT NULL CHECK (
                            date(due_on) IS NOT NULL
                            AND date(due_on) = due_on
                        ),
    amount_cents        INTEGER NOT NULL CHECK (amount_cents > 0),
    recognition_entry_id TEXT NOT NULL UNIQUE REFERENCES journal_entries(id),

    UNIQUE (plan_id, seq),
    UNIQUE (plan_id, due_on)
);

CREATE INDEX ix_installments_due
    ON installments(due_on);

CREATE INDEX ix_installments_plan
    ON installments(plan_id, seq);

CREATE TABLE card_payments (
    id              TEXT PRIMARY KEY NOT NULL,
    card_account_id TEXT NOT NULL REFERENCES accounts(id),
    bank_account_id TEXT NOT NULL REFERENCES accounts(id),
    entry_id        TEXT NOT NULL UNIQUE REFERENCES journal_entries(id),
    amount_cents    INTEGER NOT NULL CHECK (amount_cents > 0),
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,

    CHECK (card_account_id <> bank_account_id)
);

CREATE INDEX ix_card_payments_card
    ON card_payments(card_account_id);

CREATE TABLE installment_allocations (
    payment_id     TEXT NOT NULL REFERENCES card_payments(id) ON DELETE CASCADE,
    installment_id TEXT NOT NULL REFERENCES installments(id) ON DELETE CASCADE,
    amount_cents   INTEGER NOT NULL CHECK (amount_cents > 0),

    PRIMARY KEY (payment_id, installment_id)
);

CREATE INDEX ix_allocations_installment
    ON installment_allocations(installment_id);

CREATE TABLE settings (
    key        TEXT PRIMARY KEY NOT NULL,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Contas de sistema. IDs fixos e estáveis.
INSERT INTO accounts (
    id, name, class, type, is_system, credit_limit_cents, archived_at, created_at, updated_at
) VALUES
    (
        '00000000-0000-7000-8000-000000000001',
        'Patrimônio',
        'equity',
        NULL,
        1,
        NULL,
        NULL,
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    ),
    (
        '00000000-0000-7000-8000-000000000002',
        'Receitas',
        'income',
        NULL,
        1,
        NULL,
        NULL,
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    ),
    (
        '00000000-0000-7000-8000-000000000003',
        'Despesas',
        'expense',
        NULL,
        1,
        NULL,
        NULL,
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    ),
    (
        '00000000-0000-7000-8000-000000000004',
        'Despesa a reconhecer',
        'asset',
        NULL,
        1,
        NULL,
        NULL,
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
    );

INSERT INTO settings (key, value, updated_at) VALUES
    ('backup_retention', '10', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    ('first_run_completed', 'false', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

-- Lançamentos nascem como draft. O app grava as postings e depois publica.
CREATE TRIGGER trg_journal_entries_insert_draft
BEFORE INSERT ON journal_entries
WHEN NEW.posted_at IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'journal entry must be inserted as draft');
END;

-- Um lançamento só pode ser publicado se tiver pelo menos duas postings e soma zero.
CREATE TRIGGER trg_journal_entries_post_balance
BEFORE UPDATE OF posted_at ON journal_entries
WHEN NEW.posted_at IS NOT NULL AND OLD.posted_at IS NULL
BEGIN
    SELECT CASE
        WHEN (SELECT COUNT(*) FROM postings WHERE entry_id = NEW.id) < 2
          OR (SELECT COALESCE(SUM(amount_cents), 0) FROM postings WHERE entry_id = NEW.id) <> 0
        THEN RAISE(ABORT, 'journal entry is not balanced')
    END;
END;

-- Lançamentos publicados são imutáveis no nível das postings.
CREATE TRIGGER trg_journal_entries_posted_update_guard
BEFORE UPDATE OF
    kind, occurred_on, occurred_time, manual_period, description,
    primary_account_id, counterparty_account_id, category_id, payment_method
ON journal_entries
WHEN OLD.posted_at IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'cannot modify a posted journal entry');
END;

CREATE TRIGGER trg_postings_insert_guard
BEFORE INSERT ON postings
WHEN (SELECT posted_at FROM journal_entries WHERE id = NEW.entry_id) IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'cannot add posting to a posted entry');
END;

CREATE TRIGGER trg_postings_update_guard
BEFORE UPDATE ON postings
WHEN (SELECT posted_at FROM journal_entries WHERE id = OLD.entry_id) IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'cannot update posting of a posted entry');
END;

CREATE TRIGGER trg_postings_delete_guard
BEFORE DELETE ON postings
WHEN (SELECT posted_at FROM journal_entries WHERE id = OLD.entry_id) IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'cannot delete posting of a posted entry');
END;
