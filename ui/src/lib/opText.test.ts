import { describe, expect, it } from "vitest";
import type { OpRecord } from "./api";
import { opResult, opRunning } from "./opText";

const GB = 1024 ** 3;
const rec = (over: Partial<OpRecord>): OpRecord => ({
  id: 1,
  kind: "recycle",
  count: 3,
  bytes: 1.2 * GB,
  state: { state: "running", done: 2, total: 5 },
  outcome: null,
  startedAt: 0,
  ...over,
});

describe("textos das operações", () => {
  it("em andamento, com porcentagem", () => {
    expect(opRunning(rec({}))).toBe("Mandando 3 itens para a Lixeira… 40%");
    expect(opRunning(rec({ kind: "deletePermanent", count: 1, state: { state: "queued" } }))).toBe("Apagando 1 item…");
  });

  it("resultado", () => {
    const ok = { succeeded: ["a", "b", "c"], failed: [], aborted: false };
    expect(opResult(rec({ kind: "deletePermanent", state: { state: "done" }, outcome: ok }))).toBe("Liberou 1,2 GB");
    expect(opResult(rec({ state: { state: "done" }, outcome: ok }))).toBe("1,2 GB foram para a Lixeira");
    const bad = { succeeded: ["a"], failed: [{ path: "b", message: "x" }, { path: "c", message: "y" }], aborted: false };
    expect(opResult(rec({ kind: "deletePermanent", state: { state: "done" }, outcome: bad }))).toBe("2 de 3 itens não puderam ser apagados");
    const one = { succeeded: ["a", "b"], failed: [{ path: "c", message: "x" }], aborted: false };
    expect(opResult(rec({ state: { state: "done" }, outcome: one }))).toBe("1 de 3 itens não pôde ser mandado para a Lixeira");
  });
});
