-- Vários rendimentos confirmados podem coexistir na mesma data. O histórico
-- existente não é alterado: apenas a restrição antiga de um por dia é removida.
-- Os índices por entry continuam únicos para não duplicar a listagem de movimentos.
DROP INDEX IF EXISTS ux_savings_yields_confirmed;
