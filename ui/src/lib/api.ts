import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type ScanMode = "mft" | "walk";

export interface DriveInfo {
  letter: string;
  label: string;
  filesystem: string;
  total: number;
  free: number;
}

export type DriveStatus =
  | { state: "scanning"; fraction: number; mode: ScanMode }
  | { state: "ready"; mode: ScanMode; live: boolean }
  | { state: "failed"; message: string };

export interface DriveView {
  info: DriveInfo;
  status: DriveStatus;
}

export interface DriveSummary {
  drive: string;
  totalBytes: number;
  freeBytes: number;
  usedBytes: number;
  /** Bytes por categoria, na ordem do índice do Rust (0 = Apps … 9 = Outros). */
  byCategory: number[];
  catalogedBytes: number;
  mode: ScanMode;
}

export interface BreakdownItem {
  key: string;
  label: string;
  bytes: number;
}

export interface ProgressEvent {
  drive: string;
  fraction: number;
  mode: ScanMode;
}

export const listDrives = () => invoke<DriveView[]>("list_drives");
export const driveSummary = (drive: string) => invoke<DriveSummary | null>("drive_summary", { drive });
export const systemBreakdown = (drive: string) => invoke<BreakdownItem[] | null>("system_breakdown", { drive });
export const rescanDrive = (drive: string) => invoke<void>("rescan_drive", { drive });
export const appIsElevated = () => invoke<boolean>("app_is_elevated");

export const onIndexProgress = (cb: (e: ProgressEvent) => void): Promise<UnlistenFn> =>
  listen<ProgressEvent>("index-progress", (e) => cb(e.payload));
export const onIndexReady = (cb: (e: { drive: string; live: boolean }) => void): Promise<UnlistenFn> =>
  listen<{ drive: string; live: boolean }>("index-ready", (e) => cb(e.payload));
/** O disco mudou com o app aberto (no máximo 1×/s por disco). */
export const onIndexUpdated = (cb: (e: { drive: string }) => void): Promise<UnlistenFn> =>
  listen<{ drive: string }>("index-updated", (e) => cb(e.payload));
export const onIndexFailed = (cb: (e: { drive: string; message: string }) => void): Promise<UnlistenFn> =>
  listen<{ drive: string; message: string }>("index-failed", (e) => cb(e.payload));

// ---- Fase 3: navegação e operações ----

import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type { CategoryKey } from "./categories";

export interface ItemRef {
  drive: string;
  node: number;
  frn: number;
  /** O caminho mostrado na tela: o backend recusa se o nó agora for outro item. */
  path: string;
}

export interface ItemRow {
  node: number;
  frn: number;
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  alloc: number;
  /** Quanto ocupa na categoria pedida. */
  bytes: number;
  modified: number;
  category: CategoryKey;
  childCount: number;
}

export interface FileFilter {
  category?: CategoryKey;
  under?: number;
  extension?: string;
  minSize?: number;
  maxSize?: number;
  search?: string;
  modifiedBefore?: number;
  modifiedAfter?: number;
}

export type SortKey = "sizeDesc" | "sizeAsc" | "nameAsc" | "nameDesc" | "modifiedDesc" | "modifiedAsc";

export interface Page {
  rows: ItemRow[];
  total: number;
}

export type OpKindTag = "recycle" | "deletePermanent" | "copy" | "move" | "rename" | "reveal";

export type Verdict =
  | { verdict: "allowed" }
  | { verdict: "warn"; message: string }
  | { verdict: "blocked"; message: string };

export interface CheckResult {
  item: ItemRef;
  verdict: Verdict;
  bytes: number;
  /** `null` = o item mudou desde que a lista foi mostrada. */
  path: string | null;
}

export type OpRequest =
  | { kind: "recycle" }
  | { kind: "deletePermanent" }
  | { kind: "copy"; dest: string }
  | { kind: "move"; dest: string }
  | { kind: "rename"; newName: string };

export interface ItemFailure {
  path: string;
  message: string;
}

export interface OpOutcome {
  succeeded: string[];
  failed: ItemFailure[];
  aborted: boolean;
}

export type OpState = { state: "queued" } | { state: "running"; done: number; total: number } | { state: "done" } | { state: "failed" };

export interface OpRecord {
  id: number;
  kind: OpKindTag;
  count: number;
  bytes: number;
  state: OpState;
  outcome: OpOutcome | null;
  startedAt: number;
}

/** `path` é o caminho da pasta como a trilha a conhece; `null` se ela não é mais a mesma. */
export const categoryChildren = (drive: string, node: number | null, path: string | null, category: CategoryKey | null) =>
  invoke<ItemRow[] | null>("category_children", { drive, node, path, category });
export const queryFiles = (drive: string, filter: FileFilter, sort: SortKey, offset: number, limit: number) =>
  invoke<Page | null>("query_files", { drive, filter, sort, offset, limit });
export const checkItems = (items: ItemRef[], op: OpKindTag) => invoke<CheckResult[]>("check_items", { items, op });
export const startOperation = (items: ItemRef[], op: OpRequest) => invoke<number>("start_operation", { items, op });
export const revealItem = (item: ItemRef) => invoke<void>("reveal_item", { item });
export const listOperations = () => invoke<OpRecord[]>("list_operations");
export const onOpProgress = (cb: (r: OpRecord) => void): Promise<UnlistenFn> => listen<OpRecord>("op-progress", (e) => cb(e.payload));
export const onOpDone = (cb: (r: OpRecord) => void): Promise<UnlistenFn> => listen<OpRecord>("op-done", (e) => cb(e.payload));
/** Diálogo nativo para escolher a pasta de destino. */
export const pickFolder = async (): Promise<string | null> => {
  const r = await openDialog({ directory: true, multiple: false, title: "Escolha a pasta de destino" });
  return typeof r === "string" ? r : null;
};
