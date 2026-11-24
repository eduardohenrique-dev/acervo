import { describe, it, expect } from "vitest";
import { sharePercent } from "./breakdown";

describe("sharePercent", () => {
  it("é proporcional ao total do Sistema, não ao maior item", () => {
    expect(sharePercent(44, 93.4)).toBeCloseTo(47.1, 1);
    expect(sharePercent(93.4, 93.4)).toBe(100);
  });
  it("não gera NaN nem passa de 100", () => {
    expect(sharePercent(10, 0)).toBe(0);
    expect(sharePercent(120, 100)).toBe(100);
    expect(sharePercent(-5, 100)).toBe(0);
  });
});
