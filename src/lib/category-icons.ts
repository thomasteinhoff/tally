/**
 * Chaves curtas persistidas no banco. Os paths SVG vivem apenas no frontend:
 * nenhuma imagem entra no SQLite nem nos backups.
 */
export const CATEGORY_ICONS = [
  {
    key: "generic",
    label: "Genérico",
    path: "M4 5h16v14H4z M8 9h8 M8 13h8 M8 17h5",
  },
  {
    key: "cart",
    label: "Compras",
    path: "M3 5h2l2.2 10.4a2 2 0 0 0 2 1.6h7.6a2 2 0 0 0 2-1.6L20 9H6 M10 21h.01 M17 21h.01",
  },
  {
    key: "home",
    label: "Casa",
    path: "M4 11 12 4l8 7v8a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z M9 20v-6h6v6",
  },
  {
    key: "car",
    label: "Carro",
    path: "M5 16v3h2v-3 M17 16v3h2v-3 M4 16h16v-5l-2-5H6l-2 5z M4 11h16 M7.5 14h.01 M16.5 14h.01",
  },
  {
    key: "bus",
    label: "Transporte",
    path: "M5 17V6a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2v11 M5 11h14 M5 17h14 M7 20v-3 M17 20v-3 M8 14h.01 M16 14h.01",
  },
  {
    key: "fuel",
    label: "Combustível",
    path: "M4 21V6a2 2 0 0 1 2-2h6a2 2 0 0 1 2 2v15 M3 21h12 M5 9h8 M15 8l3 3v6a2 2 0 0 0 4 0v-6l-2-3",
  },
  {
    key: "food",
    label: "Alimentação",
    path: "M6 3v8a3 3 0 0 0 6 0V3 M9 3v18 M17 3v18 M17 3c2 2 2 6 0 8",
  },
  {
    key: "coffee",
    label: "Café",
    path: "M4 8h13v7a5 5 0 0 1-5 5H9a5 5 0 0 1-5-5z M17 9h2a2 2 0 0 1 0 4h-2 M7 3v2 M11 3v2",
  },
  {
    key: "heart",
    label: "Saúde",
    path: "M12 20s-8-5-8-10a4.5 4.5 0 0 1 8-2.8A4.5 4.5 0 0 1 20 10c0 5-8 10-8 10z",
  },
  {
    key: "pill",
    label: "Remédios",
    path: "M10.5 20.5a5 5 0 0 1-7-7l7-7a5 5 0 0 1 7 7z M7 7l10 10",
  },
  {
    key: "book",
    label: "Educação",
    path: "M4 5a2 2 0 0 1 2-2h13v16H6a2 2 0 0 0-2 2z M4 19a2 2 0 0 1 2-2h13",
  },
  {
    key: "game",
    label: "Lazer",
    path: "M7 9h10a4 4 0 0 1 4 4l-1 5a2 2 0 0 1-3.5 1L14 16h-4l-2.5 3A2 2 0 0 1 4 18l-1-5a4 4 0 0 1 4-4z M7.5 12v3 M6 13.5h3 M16 12h.01 M18 14h.01",
  },
  {
    key: "film",
    label: "Cinema",
    path: "M4 5h16v14H4z M8 5v14 M16 5v14 M4 10h4 M16 10h4 M4 14h4 M16 14h4",
  },
  {
    key: "music",
    label: "Música",
    path: "M9 18V5l10-2v13 M9 18a3 3 0 1 1-6 0 3 3 0 0 1 6 0 M19 16a3 3 0 1 1-6 0 3 3 0 0 1 6 0",
  },
  {
    key: "plane",
    label: "Viagem",
    path: "M21 15 14 12V6a2 2 0 0 0-4 0v6L3 15v2l7-2v4l-3 2v1l5-1 5 1v-1l-3-2v-4l7 2z",
  },
  {
    key: "gift",
    label: "Presente",
    path: "M4 11h16v9H4z M3 7h18v4H3z M12 7v13 M12 7C9 7 8 3 10 3s2 4 2 4 M12 7c3 0 4-4 2-4s-2 4-2 4",
  },
  {
    key: "paw",
    label: "Pets",
    path: "M12 13c-3 0-5 2-5 4a3 3 0 0 0 3 3h4a3 3 0 0 0 3-3c0-2-2-4-5-4 M7 9a1.5 1.5 0 1 1 3 0 1.5 1.5 0 0 1-3 0 M14 9a1.5 1.5 0 1 1 3 0 1.5 1.5 0 0 1-3 0 M10 5a1.5 1.5 0 1 1 3 0 1.5 1.5 0 0 1-3 0",
  },
  {
    key: "bolt",
    label: "Energia",
    path: "M13 2 4 14h7l-1 8 9-12h-7z",
  },
  {
    key: "droplet",
    label: "Água",
    path: "M12 3s6 7 6 11a6 6 0 0 1-12 0c0-4 6-11 6-11z",
  },
  {
    key: "wifi",
    label: "Internet",
    path: "M4 9a12 12 0 0 1 16 0 M7 12a8 8 0 0 1 10 0 M10 15a4 4 0 0 1 4 0 M12 19h.01",
  },
  {
    key: "briefcase",
    label: "Trabalho",
    path: "M4 8h16v12H4z M9 8V5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v3 M4 13h16",
  },
  {
    key: "piggy",
    label: "Poupança",
    path: "M5 12a7 7 0 0 1 7-7h3a7 7 0 0 1 7 7v3l-2 1v3h-3l-1-2h-4l-1 2H8v-3l-3-1z M16 10h.01 M3 12h2",
  },
  {
    key: "trend",
    label: "Rendimentos",
    path: "M4 19V5 M4 19h16 M7 15l4-5 3 3 5-7 M16 6h3v3",
  },
  {
    key: "star",
    label: "Destaque",
    path: "M12 3l2.8 5.7 6.2.9-4.5 4.4 1 6.2-5.5-2.9-5.5 2.9 1-6.2L3 9.6l6.2-.9z",
  },
  {
    key: "basket",
    label: "Mercado",
    path: "M3 9h18l-2 11H5z M8 9l4-6 4 6 M9 13v4 M15 13v4",
  },
  {
    key: "restaurant",
    label: "Restaurante",
    path: "M6 3v7a3 3 0 0 0 6 0V3 M9 3v18 M17 3v18 M17 3c2 2 2 6 0 8",
  },
  {
    key: "clothes",
    label: "Roupas",
    path: "M8 3 4 6l2 4 2-1v12h8V9l2 1 2-4-4-3a4 4 0 0 1-8 0z",
  },
  {
    key: "gym",
    label: "Academia",
    path: "M2 12h3 M19 12h3 M5 8v8 M8 6v12 M16 6v12 M19 8v8 M8 12h8",
  },
  {
    key: "camera",
    label: "Fotografia",
    path: "M3 8h4l2-3h6l2 3h4v12H3z M12 17a4 4 0 1 0 0-8 4 4 0 0 0 0 8z",
  },
  {
    key: "plant",
    label: "Jardim",
    path: "M12 21v-8 M12 13c-4 0-6-3-6-6 4 0 6 2 6 6 M12 13c4 0 6-3 6-6-4 0-6 2-6 6 M8 17h8l-1 4H9z",
  },
  {
    key: "tools",
    label: "Manutenção",
    path: "M14 4l6 6-3 3-6-6z M11 7 4 14l3 3 7-7 M5 19l-2 2 M18 15a3 3 0 1 0 0 6 3 3 0 0 0 0-6",
  },
  {
    key: "cleaning",
    label: "Limpeza",
    path: "M12 3v12 M8 15h8l2 6H6z M8 15l-4-4 M16 15l4-4",
  },
  {
    key: "furniture",
    label: "Móveis",
    path: "M4 12V8a2 2 0 0 1 4 0v4 M16 12V8a2 2 0 0 1 4 0v4 M3 12h18v6H3z M6 18v2 M18 18v2 M7 12h10",
  },
  {
    key: "phone",
    label: "Telefonia",
    path: "M7 2h10v20H7z M10 5h4 M11 19h2",
  },
  {
    key: "software",
    label: "Software",
    path: "M9 7 4 12l5 5 M15 7l5 5-5 5 M13 5l-2 14",
  },
  {
    key: "mail",
    label: "Correios",
    path: "M3 6h18v12H3z M3 7l9 6 9-6",
  },
  {
    key: "insurance",
    label: "Seguro",
    path: "M12 2 4 5v6c0 5 3.5 9 8 11 4.5-2 8-6 8-11V5z M9 11l2 2 4-4",
  },
  {
    key: "tax",
    label: "Impostos",
    path: "M6 2h9l4 4v16H6z M15 2v4h4 M9 11h.01 M15 17h.01 M9 17l7-7",
  },
  {
    key: "hotel",
    label: "Hotel",
    path: "M3 19v-9 M3 14h18v5 M21 19v-5 M6 10h5a2 2 0 0 1 2 2v2H6z M3 19h18",
  },
  {
    key: "parking",
    label: "Estacionamento",
    path: "M4 3h16v18H4z M9 17V7h4a3 3 0 0 1 0 6H9",
  },
  {
    key: "bicycle",
    label: "Bicicleta",
    path: "M5 18a3 3 0 1 0 0-6 3 3 0 0 0 0 6 M19 18a3 3 0 1 0 0-6 3 3 0 0 0 0 6 M5 15l5-7h4l5 7 M9 8h6 M14 8l-3 7",
  },
  {
    key: "donation",
    label: "Doações",
    path: "M12 3a4 4 0 1 0 0 8 4 4 0 0 0 0-8 M4 16h5l2 5h2l2-5h5 M6 21h12",
  },
  {
    key: "salary",
    label: "Salário",
    path: "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20 M12 6v12 M15 9c0-1.5-1.3-2-3-2s-3 .5-3 2 1.3 2 3 2 3 .5 3 2-1.3 2-3 2-3-.5-3-2",
  },
  {
    key: "bonus",
    label: "Bônus",
    path: "M8 3h8v5a4 4 0 0 1-8 0z M8 5H5v2a3 3 0 0 0 3 3 M16 5h3v2a3 3 0 0 1-3 3 M12 12v4 M8 20h8 M10 16h4l1 4H9z",
  },
  {
    key: "freelance",
    label: "Freelance",
    path: "M5 4h14v11H5z M2 19h20l-2-4H4z M9 8h6",
  },
  {
    key: "business",
    label: "Negócio",
    path: "M4 10h16v10H4z M3 10l2-6h14l2 6 M4 10a2 2 0 0 0 4 0 2 2 0 0 0 4 0 2 2 0 0 0 4 0 2 2 0 0 0 4 0 M9 20v-6h6v6",
  },
  {
    key: "sales",
    label: "Vendas",
    path: "M3 12 12 3h9v9l-9 9z M16 8h.01 M9 15l6-6",
  },
  {
    key: "interest",
    label: "Juros",
    path: "M6 6a2 2 0 1 0 0 4 2 2 0 0 0 0-4 M18 14a2 2 0 1 0 0 4 2 2 0 0 0 0-4 M19 5 5 19",
  },
  {
    key: "coins",
    label: "Dividendos",
    path: "M12 3a9 4 0 1 0 0 8 9 4 0 0 0 0-8 M3 7v5c0 2.2 4 4 9 4s9-1.8 9-4V7 M3 12v5c0 2.2 4 4 9 4s9-1.8 9-4v-5",
  },
  {
    key: "refund",
    label: "Reembolso",
    path: "M20 12a8 8 0 1 1-2.3-5.7 M20 4v4h-4 M12 8v8 M15 10c0-1.3-1.3-2-3-2s-3 .7-3 2 1.3 2 3 2 3 .7 3 2-1.3 2-3 2-3-.7-3-2",
  },
] as const;

export type CategoryIconKey = (typeof CATEGORY_ICONS)[number]["key"];

const GENERIC_ICON_PATH = CATEGORY_ICONS[0].path;

const paths = new Map<string, string>(
  CATEGORY_ICONS.map((icon) => [icon.key, icon.path]),
);

const labels = new Map<string, string>(
  CATEGORY_ICONS.map((icon) => [icon.key, icon.label]),
);

export function categoryIconPath(key?: string | null): string {
  return (key && paths.get(key)) || GENERIC_ICON_PATH;
}

export function categoryIconLabel(key?: string | null): string {
  return (key && labels.get(key)) || "Genérico";
}

export function isCategoryIconKey(key: string): boolean {
  return paths.has(key);
}
