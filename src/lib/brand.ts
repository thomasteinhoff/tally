/**
 * Símbolo oficial do Tally — fonte única do desenho no frontend.
 *
 * Referência: `tally.png` na raiz do projeto (1024 × 1024). O viewBox recorta
 * apenas a área desenhada. Geometria medida no arquivo original; não redesenhar.
 *
 * Para substituir o símbolo no futuro:
 * 1. troque `tally.png` e os ícones em `src-tauri/icons` (npx tauri icon tally.png);
 * 2. se a geometria mudar, atualize as listas abaixo — nenhum outro lugar
 *    desenha o símbolo no frontend;
 * 3. mantenha o viewBox quadrado para preservar a proporção em qualquer tamanho.
 */
export const BRAND_MARK_SOURCE = "tally.png";

export const BRAND_MARK_VIEWBOX = "128 128 768 768";

export type BrandMarkRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

/** Três marcas em warm stone. */
export const BRAND_MARK_STONE: BrandMarkRect[] = [
  { x: 240, y: 128, width: 32, height: 768 },
  { x: 496, y: 128, width: 32, height: 768 },
  { x: 752, y: 128, width: 32, height: 768 },
];

/** Marca terracota e faixa horizontal (desenhadas por último). */
export const BRAND_MARK_ACCENT: BrandMarkRect[] = [
  { x: 317, y: 128, width: 48, height: 768 },
  { x: 128, y: 333, width: 768, height: 16 },
];
