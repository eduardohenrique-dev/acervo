import type { DriveSummary, DriveView, ScanMode } from "../lib/api";

export interface DriveEntry {
  view: DriveView;
  summary: DriveSummary | null;
}

export type DrivesState = Record<string, DriveEntry>;

export type DrivesAction =
  | { type: "loaded"; views: DriveView[] }
  | { type: "progress"; drive: string; fraction: number; mode: ScanMode }
  | { type: "ready"; drive: string; summary: DriveSummary | null; live: boolean }
  | { type: "updated"; drive: string; summary: DriveSummary | null }
  | { type: "failed"; drive: string; message: string };

export function drivesReducer(state: DrivesState, action: DrivesAction): DrivesState {
  switch (action.type) {
    case "loaded": {
      const next: DrivesState = {};
      for (const view of action.views) {
        next[view.info.letter] = { view, summary: state[view.info.letter]?.summary ?? null };
      }
      return next;
    }
    case "progress":
    case "ready":
    case "updated":
    case "failed": {
      const entry = state[action.drive];
      if (!entry) return state;
      if (action.type === "progress") {
        const status = { state: "scanning" as const, fraction: action.fraction, mode: action.mode };
        return { ...state, [action.drive]: { ...entry, view: { ...entry.view, status } } };
      }
      if (action.type === "ready") {
        const mode = action.summary?.mode ?? (entry.view.status.state !== "failed" ? entry.view.status.mode : "walk");
        return {
          ...state,
          [action.drive]: { summary: action.summary, view: { ...entry.view, status: { state: "ready", mode, live: action.live } } },
        };
      }
      if (action.type === "updated") {
        // Só troca os números de um disco já pronto; durante uma análise o resumo virá no "ready".
        if (entry.view.status.state !== "ready" || !action.summary) return state;
        return { ...state, [action.drive]: { ...entry, summary: action.summary } };
      }
      return {
        ...state,
        [action.drive]: { ...entry, view: { ...entry.view, status: { state: "failed", message: action.message } } },
      };
    }
  }
}
