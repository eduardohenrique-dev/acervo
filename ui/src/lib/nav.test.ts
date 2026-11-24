import { describe, it, expect } from "vitest";
import { nearestExisting } from "./nav";

describe("nearestExisting", () => {
  it("corta no último que existe", () => {
    const trail = [
      { node: null, name: "C:", path: null },
      { node: 10, name: "Jogos", path: String.raw`C:\Jogos` },
      { node: 11, name: "Steam", path: String.raw`C:\Jogos\Steam` },
      { node: 12, name: "Hades", path: String.raw`C:\Jogos\Steam\Hades` },
    ];
    expect(nearestExisting(trail, (n) => n !== 11 && n !== 12).map((c) => c.name)).toEqual(["C:", "Jogos"]);
    expect(nearestExisting(trail, () => false).map((c) => c.name)).toEqual(["C:"]);
    expect(nearestExisting(trail, () => true)).toHaveLength(4);
  });

  it("um buraco no meio corta tudo depois dele", () => {
    const trail = [{ node: null, name: "C:", path: null }, { node: 1, name: "a", path: String.raw`C:\a` }, { node: 2, name: "b", path: String.raw`C:\a\b` }];
    expect(nearestExisting(trail, (n) => n !== 1).map((c) => c.name)).toEqual(["C:"]);
  });
});
