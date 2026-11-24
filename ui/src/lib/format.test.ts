import { describe, it, expect } from "vitest";
import { formatBytes, formatFiletime, formatPercent, toFiletime } from "./format";

describe("formatBytes", () => {
  it("formata", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1023)).toBe("1023 B");
    expect(formatBytes(1024)).toBe("1 KB");
    expect(formatBytes(1536)).toBe("1,5 KB");
    expect(formatBytes(53.1 * 1024 ** 3)).toBe("53,1 GB");
    expect(formatBytes(930.5 * 1024 ** 3)).toBe("931 GB");
    expect(formatBytes(1.82 * 1024 ** 4)).toBe("1,8 TB");
  });
});

describe("formatPercent", () => {
  it("formata", () => {
    expect(formatPercent(0.42)).toBe("42%");
    expect(formatPercent(0.005)).toBe("< 1%");
    expect(formatPercent(0)).toBe("0%");
  });
});

describe("datas do NTFS", () => {
  it("FILETIME vira data dd/mm/aaaa e volta", () => {
    const ft = toFiletime(new Date(2026, 10, 6, 12).getTime());
    expect(formatFiletime(ft)).toBe("06/11/2026");
    expect(formatFiletime(0)).toBe("—");
  });
});
