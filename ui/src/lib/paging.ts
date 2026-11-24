import type { ItemRow } from "./api";

/** Linhas por pedido ao backend. */
export const PAGE = 200;

/** Páginas que cobrem as linhas `first..last`, com uma de folga antes e depois. */
export function pagesFor(first: number, last: number): number[] {
  const from = Math.max(0, Math.floor(first / PAGE) - 1);
  const to = Math.floor(Math.max(first, last) / PAGE) + 1;
  const out: number[] = [];
  for (let p = from; p <= to; p++) out.push(p);
  return out;
}

/** Páginas já buscadas. Limpa quando filtro, ordenação ou disco mudam, ou o disco muda ao vivo. */
export class PageCache {
  private pages = new Map<number, ItemRow[]>();
  get(page: number): ItemRow[] | undefined {
    return this.pages.get(page);
  }
  set(page: number, rows: ItemRow[]): void {
    this.pages.set(page, rows);
  }
  has(page: number): boolean {
    return this.pages.has(page);
  }
  clear(): void {
    this.pages.clear();
  }
}
