import { CATEGORIES, type CategoryKey } from "./categories";

export interface Segment {
  key: CategoryKey;
  bytes: number;
  /** Largura em % da barra inteira (0–100). */
  percent: number;
}

/** Largura mínima de qualquer categoria com bytes, para ela não sumir da barra. */
export const MIN_PERCENT = 0.6;

/**
 * Larguras dos segmentos da barra, na ordem de exibição.
 * Omite categorias vazias, garante a largura mínima e nunca passa de 100%.
 */
export function segmentWidths(byCategory: number[], total: number): Segment[] {
  if (!(total > 0)) return [];
  const items = CATEGORIES.map((c) => ({ key: c.key, bytes: Math.max(0, byCategory[c.index] ?? 0) })).filter(
    (x) => x.bytes > 0,
  );
  const sum = items.reduce((a, x) => a + x.bytes, 0);
  const denom = Math.max(total, sum);
  const raw = items.map((x) => (x.bytes / denom) * 100);

  // As pequenas sobem para o mínimo. Só se isso passar de 100% as grandes cedem o excesso,
  // proporcionalmente e sem descer abaixo do mínimo; senão o espaço sai da parte livre.
  const lifted = raw.map((r) => Math.max(r, MIN_PERCENT));
  const excess = Math.max(0, lifted.reduce((a, r) => a + r, 0) - 100);
  const bigSum = raw.reduce((a, r) => a + (r > MIN_PERCENT ? r : 0), 0);

  return items.map((x, i) => {
    const r = lifted[i];
    const percent = r > MIN_PERCENT && bigSum > 0 ? Math.max(MIN_PERCENT, r - (excess * r) / bigSum) : r;
    return { ...x, percent };
  });
}
