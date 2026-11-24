import { describe, it, expect } from "vitest";
import { segmentWidths } from "./bar";

const arr = (o: Record<number, number>) => Array.from({ length: 10 }, (_, i) => o[i] ?? 0);

describe("segmentWidths", () => {
  it("ordena e calcula percentuais", () => {
    const s = segmentWidths(arr({ 0: 50, 8: 30, 3: 20 }), 100);
    expect(s.map((x) => x.key)).toEqual(["apps", "midia", "sistema"]);
    expect(s.map((x) => x.percent)).toEqual([50, 20, 30]);
  });

  it("garante largura mínima", () => {
    const s = segmentWidths(arr({ 0: 999_000, 6: 1 }), 1_000_000);
    expect(s.find((x) => x.key === "cache")!.percent).toBeCloseTo(0.6);
    expect(s.reduce((a, x) => a + x.percent, 0)).toBeLessThanOrEqual(100.0001);
  });

  it("segmentWidths handles zero total", () => {
    expect(segmentWidths(arr({}), 0)).toEqual([]);
    expect(segmentWidths(arr({ 0: 10 }), 0)).toEqual([]);
  });

  it("disco quase vazio: largura mínima vem do espaço livre, sem encolher as outras", () => {
    const GB = 1024 ** 3;
    const s = segmentWidths(arr({ 9: 7 * GB, 8: 1 * GB, 6: 0.01 * GB }), 1024 * GB);
    for (const x of s) expect(x.percent).toBeGreaterThanOrEqual(0.6);
    const outros = s.find((x) => x.key === "outros")!;
    expect(outros.percent).toBeCloseTo((7 / 1024) * 100, 5);
  });

  it("nunca devolve NaN nem negativo", () => {
    const s = segmentWidths(arr({ 0: 5, 1: 5, 2: 5 }), 10); // soma acima do total
    for (const x of s) {
      expect(Number.isNaN(x.percent)).toBe(false);
      expect(x.percent).toBeGreaterThanOrEqual(0);
    }
    expect(s.reduce((a, x) => a + x.percent, 0)).toBeLessThanOrEqual(100.0001);
  });
});
