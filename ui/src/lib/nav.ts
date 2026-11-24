/** Um passo da trilha "C: › Jogos › Steam". `node: null` é a raiz do disco. */
export interface Crumb {
  node: number | null;
  name: string;
  /** Caminho completo da pasta (`null` na raiz): confere que o nó ainda é a mesma pasta. */
  path: string | null;
}

/** Corta a trilha no último passo que ainda existe (a raiz sempre existe). */
export function nearestExisting(trail: Crumb[], exists: (node: number | null) => boolean): Crumb[] {
  const out: Crumb[] = [];
  for (const c of trail) {
    if (c.node !== null && !exists(c.node)) break;
    out.push(c);
  }
  return out.length ? out : trail.slice(0, 1);
}
