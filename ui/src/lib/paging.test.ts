import { describe, expect, it } from "vitest";
import type { ItemRow } from "./api";
import { PageCache, pagesFor } from "./paging";

describe("paging", () => {
  it("pagesFor cobre a janela com folga", () => {
    expect(pagesFor(0, 30)).toEqual([0, 1]);
    expect(pagesFor(450, 470)).toEqual([1, 2, 3]);
  });

  it("PageCache limpa", () => {
    const c = new PageCache();
    c.set(0, [1, 2] as unknown as ItemRow[]);
    expect(c.get(0)).toHaveLength(2);
    c.clear();
    expect(c.get(0)).toBeUndefined();
  });
});
