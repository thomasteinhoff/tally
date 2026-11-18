-- v0.2.0 — ícones de categoria.
--
-- O banco guarda apenas uma chave curta. Os paths SVG vivem no frontend, que
-- resolve chaves vazias ou desconhecidas para o ícone genérico.

ALTER TABLE categories
    ADD COLUMN icon TEXT NOT NULL DEFAULT 'generic'
    CHECK (
        icon <> ''
        AND length(icon) <= 32
        AND icon NOT GLOB '*[^a-z0-9_]*'
    );
