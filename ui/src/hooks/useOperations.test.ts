import { describe, expect, it } from "vitest";
import type { OpRecord } from "../lib/api";
import { HISTORY_MAX, opsReducer, type OpsState } from "./useOperations";

const rec = (id: number, state: OpRecord["state"]): OpRecord => ({
  id,
  kind: "recycle",
  count: 1,
  bytes: 10,
  state,
  outcome: state.state === "done" ? { succeeded: ["C:\\a"], failed: [], aborted: false } : null,
  startedAt: id,
});

const empty: OpsState = { running: null, history: [] };

describe("opsReducer", () => {
  it("progress atualiza o running", () => {
    const s = opsReducer(empty, { type: "progress", rec: rec(1, { state: "running", done: 1, total: 4 }) });
    expect(s.running?.id).toBe(1);
    const s2 = opsReducer(s, { type: "progress", rec: rec(1, { state: "running", done: 3, total: 4 }) });
    expect(s2.running?.state).toEqual({ state: "running", done: 3, total: 4 });
  });

  it("done vai para o topo do histórico e zera o running", () => {
    let s = opsReducer(empty, { type: "done", rec: rec(1, { state: "done" }) });
    s = opsReducer(s, { type: "progress", rec: rec(2, { state: "running", done: 1, total: 4 }) });
    s = opsReducer(s, { type: "done", rec: rec(2, { state: "done" }) });
    expect(s.running).toBeNull();
    expect(s.history.map((r) => r.id)).toEqual([2, 1]);
  });

  it("guarda no máximo 50 no histórico, descartando os mais antigos", () => {
    let s = empty;
    for (let i = 1; i <= HISTORY_MAX + 5; i++) s = opsReducer(s, { type: "done", rec: rec(i, { state: "done" }) });
    expect(s.history).toHaveLength(HISTORY_MAX);
    expect(s.history[0].id).toBe(HISTORY_MAX + 5);
    expect(s.history.at(-1)!.id).toBe(6);
  });

  it("loaded separa o que roda do que terminou, mais novo primeiro", () => {
    const s = opsReducer(empty, {
      type: "loaded",
      list: [rec(1, { state: "done" }), rec(2, { state: "done" }), rec(3, { state: "running", done: 1, total: 2 })],
    });
    expect(s.running?.id).toBe(3);
    expect(s.history.map((r) => r.id)).toEqual([2, 1]);
  });
});
