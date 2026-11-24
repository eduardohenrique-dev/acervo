import { useEffect, useReducer } from "react";
import type { OpRecord } from "../lib/api";
import { source } from "../lib/source";

/** Quantas operações a sessão guarda (o backend guarda as mesmas 50). */
export const HISTORY_MAX = 50;

export interface OpsState {
  /** A operação rodando agora (a fila roda uma por vez). */
  running: OpRecord | null;
  /** Terminadas, da mais nova para a mais antiga. */
  history: OpRecord[];
}

export type OpsAction = { type: "loaded"; list: OpRecord[] } | { type: "progress"; rec: OpRecord } | { type: "done"; rec: OpRecord };

const finished = (r: OpRecord) => r.state.state === "done" || r.state.state === "failed";

export function opsReducer(s: OpsState, a: OpsAction): OpsState {
  switch (a.type) {
    case "loaded": {
      const history = a.list.filter(finished).sort((x, y) => y.id - x.id).slice(0, HISTORY_MAX);
      return { running: a.list.find((r) => r.state.state === "running") ?? null, history };
    }
    case "progress":
      return { ...s, running: a.rec };
    case "done":
      return {
        running: s.running?.id === a.rec.id ? null : s.running,
        history: [a.rec, ...s.history.filter((r) => r.id !== a.rec.id)].slice(0, HISTORY_MAX),
      };
  }
}

/** Operação em andamento e histórico da sessão, ao vivo. */
export function useOperations(): OpsState {
  const [state, dispatch] = useReducer(opsReducer, { running: null, history: [] });
  useEffect(() => {
    let alive = true;
    void source.listOperations().then((list) => alive && dispatch({ type: "loaded", list }));
    const offs = [
      source.onOpProgress((rec) => dispatch({ type: "progress", rec })),
      source.onOpDone((rec) => dispatch({ type: "done", rec })),
    ];
    return () => {
      alive = false;
      offs.forEach((p) => void p.then((off) => off()));
    };
  }, []);
  return state;
}
