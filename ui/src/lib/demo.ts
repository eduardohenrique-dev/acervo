/**
 * Fonte de dados de DEMONSTRAÇÃO, usada só fora do Tauri (navegador de desenvolvimento).
 * Os tamanhos vêm de uma leitura real da MFT feita em 2026-11-05 na máquina do autor;
 * o detalhamento do Sistema é aproximado.
 *
 * `?estado=analisando` mantém os discos em análise; `?estado=falha` faz o D: falhar;
 * `?estado=falha-op` faz o primeiro item de cada operação falhar.
 */
import type {
  BreakdownItem,
  CheckResult,
  DriveSummary,
  DriveView,
  FileFilter,
  ItemRef,
  OpKindTag,
  OpRecord,
  OpRequest,
  ProgressEvent,
  SortKey,
} from "./api";
import type { CategoryKey } from "./categories";
import { buildDemoTree, demoChildren, demoQuery, nodes, pathOf, removeNode } from "./demoTree";

const GB = 1024 ** 3;
const gb = (n: number) => Math.round(n * GB);

const SUMMARIES: Record<string, DriveSummary> = {
  C: {
    drive: "C",
    totalBytes: gb(930.5),
    freeBytes: gb(930.5 - 694.9),
    usedBytes: gb(694.9),
    byCategory: [72.46, 247.84, 21.95, 5.93, 39.07, 0, 38.19, 106.68, 93.41, 64.3].map(gb),
    catalogedBytes: gb(689.82),
    mode: "mft",
  },
  D: {
    drive: "D",
    totalBytes: gb(1863),
    freeBytes: gb(1863 - 478.58),
    usedBytes: gb(478.58),
    byCategory: [0, 0, 0, 4.2, 0.08, 0, 0.09, 0, 0.27, 473.9].map(gb),
    catalogedBytes: gb(478.55),
    mode: "mft",
  },
};

const BREAKDOWN: Record<string, BreakdownItem[]> = {
  C: [
    { key: "windows", label: "Windows", bytes: gb(44.0) },
    { key: "memoria_virtual", label: "Memória virtual", bytes: gb(19.52) },
    { key: "restauracao", label: "Pontos de restauração", bytes: gb(13.49) },
    { key: "hibernacao", label: "Hibernação", bytes: gb(12.77) },
    { key: "ntfs", label: "Estruturas do NTFS", bytes: gb(2.4) },
    { key: "outros", label: "Outros do sistema", bytes: gb(1.23) },
  ],
  D: [
    { key: "ntfs", label: "Estruturas do NTFS", bytes: gb(0.2) },
    { key: "outros", label: "Outros do sistema", bytes: gb(0.07) },
  ],
};

const param = () => new URLSearchParams(window.location.search).get("estado");

type Listener<T> = (e: T) => void;
const progressL: Listener<ProgressEvent>[] = [];
const readyL: Listener<{ drive: string; live: boolean }>[] = [];
const failedL: Listener<{ drive: string; message: string }>[] = [];
let status: Record<string, DriveView["status"]> = {};

function simulate(drive: string, ms: number) {
  status[drive] = { state: "scanning", fraction: 0, mode: "mft" };
  const start = performance.now();
  const tick = () => {
    const f = Math.min(1, (performance.now() - start) / ms);
    if (param() === "analisando") {
      const fraction = 0.42;
      status[drive] = { state: "scanning", fraction, mode: "mft" };
      progressL.forEach((l) => l({ drive, fraction, mode: "mft" }));
      return;
    }
    if (f < 1) {
      status[drive] = { state: "scanning", fraction: f, mode: "mft" };
      progressL.forEach((l) => l({ drive, fraction: f, mode: "mft" }));
      setTimeout(tick, 120);
    } else if (drive === "D" && param() === "falha") {
      const message = "Não foi possível analisar o disco D: — erro de leitura: o dispositivo não está pronto.";
      status[drive] = { state: "failed", message };
      failedL.forEach((l) => l({ drive, message }));
    } else {
      status[drive] = { state: "ready", mode: "mft", live: true };
      readyL.forEach((l) => l({ drive, live: true }));
    }
  };
  setTimeout(tick, 120);
}

/** "Ao vivo" de demonstração: a cada 6 s, Downloads do C: ganha ou perde 1,2 GB. */
const DOWNLOADS = 5;
const LIVE_STEP = gb(1.2);
function simulateLive() {
  let up = true;
  setInterval(() => {
    const s = SUMMARIES.C;
    if (status.C?.state !== "ready" || param() === "analisando") return;
    const delta = up ? LIVE_STEP : -LIVE_STEP;
    s.byCategory = s.byCategory.map((v, i) => (i === DOWNLOADS ? v + delta : v));
    s.usedBytes += delta;
    s.freeBytes -= delta;
    s.catalogedBytes += delta;
    SUMMARIES.C = { ...s };
    up = !up;
    updatedL.forEach((l) => l({ drive: "C" }));
  }, 6000);
}

let started = false;
/** Inscreve e devolve a função que cancela a inscrição (como o `listen` do Tauri). */
function sub<T>(list: Listener<T>[], cb: Listener<T>) {
  list.push(cb);
  return () => {
    const i = list.indexOf(cb);
    if (i >= 0) list.splice(i, 1);
  };
}
const updatedL: Listener<{ drive: string }>[] = [];

// ---- Navegação e operações de demonstração (Fase 3) ----

for (const s of Object.values(SUMMARIES)) buildDemoTree(s.drive, s.byCategory);

const opProgressL: Listener<OpRecord>[] = [];
const opDoneL: Listener<OpRecord>[] = [];
const history: OpRecord[] = [];
let nextOp = 1;
const PROGRAM: CategoryKey[] = ["apps", "dados_apps"];

function verdictFor(cat: CategoryKey, op: OpKindTag): CheckResult["verdict"] {
  if (op === "reveal") return { verdict: "allowed" };
  if (cat === "sistema") return { verdict: "blocked", message: "O Windows precisa disso para funcionar." };
  if (["recycle", "deletePermanent", "move"].includes(op) && PROGRAM.includes(cat))
    return { verdict: "warn", message: "Isto pertence a um programa. Se ele ainda estiver instalado, o certo é desinstalar." };
  return { verdict: "allowed" };
}

function runDemoOp(id: number, items: ItemRef[], op: OpRequest) {
  const rec = history.find((r) => r.id === id)!;
  let step = 0;
  const tick = () => {
    step++;
    if (step < 4) {
      rec.state = { state: "running", done: step, total: 4 };
      opProgressL.forEach((l) => l({ ...rec }));
      setTimeout(tick, 280);
      return;
    }
    const failFirst = param() === "falha-op";
    const succeeded: string[] = [];
    const failed: { path: string; message: string }[] = [];
    items.forEach((it, i) => {
      const n = nodes.get(it.node);
      if (!n) return;
      const path = pathOf(n);
      if (failFirst && i === 0) {
        failed.push({ path, message: "O arquivo está aberto em outro programa" });
        return;
      }
      succeeded.push(path);
      if (op.kind === "rename") n.name = op.newName;
      if (op.kind === "recycle" || op.kind === "deletePermanent" || op.kind === "move") {
        const removed = removeNode(n);
        const s = SUMMARIES[it.drive];
        const freed = removed.reduce((a, b) => a + b, 0);
        SUMMARIES[it.drive] = {
          ...s,
          byCategory: s.byCategory.map((v, k) => v - removed[k]),
          usedBytes: s.usedBytes - freed,
          freeBytes: s.freeBytes + freed,
          catalogedBytes: s.catalogedBytes - freed,
        };
      }
    });
    rec.state = { state: "done" };
    rec.outcome = { succeeded, failed, aborted: false };
    opDoneL.forEach((l) => l({ ...rec }));
    for (const d of new Set(items.map((i) => i.drive))) updatedL.forEach((l) => l({ drive: d }));
  };
  setTimeout(tick, 200);
}

/** O nó do ref, se ele ainda for o mesmo caminho que a tela mostrou. */
function sameItem(item: ItemRef) {
  const n = nodes.get(item.node);
  return n && pathOf(n).toLowerCase() === item.path.toLowerCase() ? n : undefined;
}

const demoOps = {
  async categoryChildren(drive: string, node: number | null, path: string | null, category: CategoryKey | null) {
    // Como o backend: a pasta da trilha tem de continuar sendo o mesmo caminho.
    if (node !== null && path !== null) {
      const n = nodes.get(node);
      if (!n || pathOf(n).toLowerCase() !== path.toLowerCase()) return null;
    }
    return demoChildren(drive, node, category);
  },
  async queryFiles(drive: string, filter: FileFilter, sort: SortKey, offset: number, limit: number) {
    return demoQuery(drive, filter, sort, offset, limit);
  },
  async checkItems(items: ItemRef[], op: OpKindTag): Promise<CheckResult[]> {
    return items.map((item) => {
      const n = sameItem(item);
      if (!n) return { item, verdict: { verdict: "allowed" }, bytes: 0, path: null };
      return { item, verdict: verdictFor(n.category, op), bytes: n.isDir ? 0 : n.alloc, path: pathOf(n) };
    });
  },
  async startOperation(items: ItemRef[], op: OpRequest): Promise<number> {
    if (items.some((i) => !sameItem(i))) throw "Algum item mudou desde que a lista foi mostrada. Atualize e tente de novo.";
    const kind: OpKindTag = op.kind;
    const bytes = items.reduce((a, i) => {
      const n = nodes.get(i.node)!;
      const sum = (x: typeof n): number => (x.isDir ? x.children.reduce((s, c) => s + sum(c), 0) : x.alloc);
      return a + sum(n);
    }, 0);
    const rec: OpRecord = { id: nextOp++, kind, count: items.length, bytes, state: { state: "queued" }, outcome: null, startedAt: Date.now() / 1000 };
    history.push(rec);
    runDemoOp(rec.id, items, op);
    return rec.id;
  },
  async revealItem(_item: ItemRef) {},
  async listOperations() {
    return history.map((r) => ({ ...r }));
  },
  async onOpProgress(cb: Listener<OpRecord>) {
    return sub(opProgressL, cb);
  },
  async onOpDone(cb: Listener<OpRecord>) {
    return sub(opDoneL, cb);
  },
  async pickFolder(): Promise<string | null> {
    return "D:\\Backup";
  },
};



export const demo = {
  async listDrives(): Promise<DriveView[]> {
    if (!started) {
      started = true;
      simulate("C", 1400);
      simulate("D", 2200);
      simulateLive();
    }
    return Object.values(SUMMARIES).map((s) => ({
      info: { letter: s.drive, label: "", filesystem: "NTFS", total: s.totalBytes, free: s.freeBytes },
      status: status[s.drive],
    }));
  },
  async driveSummary(drive: string) {
    return status[drive]?.state === "ready" ? SUMMARIES[drive] : null;
  },
  async systemBreakdown(drive: string) {
    return status[drive]?.state === "ready" ? BREAKDOWN[drive] : null;
  },
  async rescanDrive(drive: string) {
    simulate(drive, 1600);
    if (param() === "falha") window.history.replaceState(null, "", window.location.pathname);
  },
  async appIsElevated() {
    return true;
  },
  async onIndexProgress(cb: Listener<ProgressEvent>) {
    return sub(progressL, cb);
  },
  async onIndexReady(cb: Listener<{ drive: string; live: boolean }>) {
    return sub(readyL, cb);
  },
  async onIndexUpdated(cb: Listener<{ drive: string }>) {
    return sub(updatedL, cb);
  },
  async onIndexFailed(cb: Listener<{ drive: string; message: string }>) {
    return sub(failedL, cb);
  },
  ...demoOps,
};
