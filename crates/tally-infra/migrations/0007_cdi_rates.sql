-- Cache local das taxas CDI publicadas pelo SGS 12 (% ao dia).
--
-- Nenhuma requisição acontece na abertura: o refresh é manual e a substituição
-- do cache só acontece depois que a série inteira foi baixada e validada. Uma
-- falha de rede registra o erro, mas preserva as taxas e o último sucesso.

CREATE TABLE cdi_rates (
    series       TEXT NOT NULL,
    rate_date    TEXT NOT NULL CHECK (date(rate_date) IS NOT NULL AND date(rate_date) = rate_date),
    raw_value    TEXT NOT NULL,
    daily_rate   REAL NOT NULL CHECK (daily_rate > 0),
    fetched_at   TEXT NOT NULL,
    PRIMARY KEY (series, rate_date)
);

CREATE TABLE cdi_sync_status (
    singleton           INTEGER PRIMARY KEY CHECK (singleton = 1),
    series              TEXT NOT NULL,
    last_success_at     TEXT,
    last_attempt_at     TEXT,
    last_error          TEXT,
    first_rate_date     TEXT,
    last_rate_date      TEXT
);

INSERT INTO cdi_sync_status (
    singleton, series, last_success_at, last_attempt_at, last_error,
    first_rate_date, last_rate_date
) VALUES (1, '12', NULL, NULL, NULL, NULL, NULL);

-- Proveniência da taxa usada em cada rendimento. Colunas nullable para que
-- snapshots antigos continuem válidos: não há taxa publicada gravada neles.
ALTER TABLE savings_yields ADD COLUMN rate_date TEXT;
ALTER TABLE savings_yields ADD COLUMN rate_series TEXT;
ALTER TABLE savings_yields ADD COLUMN cdi_daily_rate REAL;
ALTER TABLE savings_yields ADD COLUMN rate_fetched_at TEXT;
