/**
 * Árvore de DEMONSTRAÇÃO para o navegador de desenvolvimento: pastas e arquivos
 * com nomes plausíveis, montados a partir dos totais reais por categoria.
 */
import type { CategoryKey } from "./categories";
import { CATEGORIES } from "./categories";
import type { FileFilter, ItemRow, Page, SortKey } from "./api";

export interface DNode {
  id: number;
  name: string;
  isDir: boolean;
  category: CategoryKey;
  alloc: number;
  modified: number;
  children: DNode[];
  parent: DNode | null;
  drive: string;
}

type Template = [path: string, share: number][];

const C_TEMPLATES: Partial<Record<CategoryKey, Template>> = {
  apps: [
    ["Program Files\\Adobe\\Photoshop 2026", 0.3],
    ["Program Files\\Microsoft Office", 0.25],
    ["Program Files\\NVIDIA Corporation", 0.12],
    ["Program Files\\Google\\Chrome", 0.08],
    ["Program Files (x86)\\Steam", 0.1],
    ["Users\\Ana\\AppData\\Local\\Programs\\Microsoft VS Code", 0.15],
  ],
  jogos: [
    ["Program Files (x86)\\Steam\\steamapps\\common\\Baldur's Gate 3", 0.4],
    ["Program Files (x86)\\Steam\\steamapps\\common\\Cyberpunk 2077", 0.28],
    ["Program Files (x86)\\Steam\\steamapps\\common\\Elden Ring", 0.2],
    ["Riot Games\\League of Legends", 0.12],
  ],
  dev: [
    ["Users\\Ana\\projetos\\jogo\\target", 0.45],
    ["Users\\Ana\\projetos\\site\\node_modules", 0.2],
    ["Users\\Ana\\.cargo", 0.2],
    ["Users\\Ana\\.rustup", 0.15],
  ],
  midia: [
    ["Users\\Ana\\Pictures\\Viagem 2025", 0.5],
    ["Users\\Ana\\Videos\\Capturas", 0.5],
  ],
  documentos: [
    ["Users\\Ana\\Documents\\Faculdade", 0.6],
    ["Users\\Ana\\Documents\\Contratos", 0.4],
  ],
  downloads: [["Users\\Ana\\Downloads", 1]],
  cache: [
    ["Users\\Ana\\AppData\\Local\\Temp", 0.45],
    ["Users\\Ana\\AppData\\Local\\Google\\Chrome\\User Data\\Default\\Cache", 0.3],
    ["$Recycle.Bin", 0.25],
  ],
  dados_apps: [
    ["ProgramData\\Package Cache", 0.45],
    ["Users\\Ana\\AppData\\Local\\Spotify", 0.3],
    ["Users\\Ana\\AppData\\Roaming\\Discord", 0.25],
  ],
  outros: [
    ["Stream", 0.4],
    ["xampp", 0.3],
    ["Users\\Ana\\Desktop", 0.3],
  ],
};

const D_TEMPLATES: Partial<Record<CategoryKey, Template>> = {
  outros: [
    ["Games", 0.46],
    ["Xbox 360", 0.29],
    ["Emulador PS2", 0.25],
  ],
  midia: [["Fotos antigas", 1]],
  documentos: [["Backup\\Documentos", 1]],
  cache: [["$RECYCLE.BIN", 1]],
};

const FILES: Partial<Record<CategoryKey, string[]>> = {
  apps: ["app.exe", "core.dll", "resources.pak", "locales.dat"],
  jogos: ["Data0.pak", "Data1.pak", "Textures.pak", "Game.exe"],
  dev: ["deps.rlib", "build.bin", "incremental.dat", "index.idx"],
  midia: ["IMG_2041.mp4", "IMG_2042.HEIC", "DSC_0101.jpg", "clipe.mov"],
  documentos: ["TCC final.pdf", "Planilha de gastos.xlsx", "Contrato aluguel.pdf", "notas.docx"],
  downloads: ["ubuntu-24.04-desktop.iso", "Instalador do jogo.exe", "fotos-da-festa.zip", "nota-fiscal.pdf"],
  cache: ["f_00a3b1", "cache_data_1", "blob-9c2e.tmp", "index"],
  dados_apps: ["storage.db", "Local State", "logs.txt", "cache.dat"],
  outros: ["disco-virtual.vhdx", "backup-antigo.7z", "dump.bin", "readme.txt"],
};

const SPLIT = [0.52, 0.26, 0.14, 0.08];

/** Arquivos pequenos a mais na Temp do C:. */
const BULK = 3000;
const BULK_WEIGHT = Array.from({ length: BULK }, (_, i) => ((i * 7919) % 1000) + 1).reduce((a, b) => a + b, 0);

let nextId = 1;
export const nodes = new Map<number, DNode>();
const roots = new Map<string, DNode>();

function mk(drive: string, name: string, isDir: boolean, category: CategoryKey, parent: DNode | null, alloc = 0): DNode {
  const n: DNode = { id: nextId++, name, isDir, category, alloc, modified: 0, children: [], parent, drive };
  nodes.set(n.id, n);
  if (parent) parent.children.push(n);
  return n;
}

function dirAt(root: DNode, path: string, category: CategoryKey): DNode {
  let cur = root;
  for (const part of path.split("\\")) {
    let next = cur.children.find((c) => c.isDir && c.name.toLowerCase() === part.toLowerCase());
    if (!next) next = mk(root.drive, part, true, category, cur);
    cur = next;
  }
  return cur;
}

/** Monta a árvore do disco a partir dos bytes por categoria (ordem do Rust). */
export function buildDemoTree(drive: string, byCategory: number[]): DNode {
  const root = mk(drive, `${drive}:`, true, "outros", null);
  roots.set(drive, root);
  const templates = drive === "C" ? C_TEMPLATES : D_TEMPLATES;
  // Datas espalhadas pelos últimos dois anos (FILETIME).
  const now = (Date.now() + 11644473600000) * 10000;
  for (const c of CATEGORIES) {
    const total = byCategory[c.index] ?? 0;
    const tpl = templates[c.key];
    if (!tpl || total <= 0) continue;
    for (const [path, share] of tpl) {
      const folder = dirAt(root, path, c.key);
      const names = FILES[c.key] ?? ["arquivo.bin"];
      // A Temp do C: ganha milhares de arquivos pequenos, para a lista da tela Arquivos ser longa de verdade.
      const bulk = drive === "C" && path.endsWith("Temp") ? BULK : 0;
      const budget = total * share;
      const bulkBytes = bulk ? budget * 0.04 : 0;
      names.forEach((name, i) => {
        const f = mk(drive, name, false, c.key, folder, Math.round((budget - bulkBytes) * SPLIT[i]));
        f.modified = now - (f.id % 730) * 864_000_000_000;
      });
      for (let i = 0; i < bulk; i++) {
        // Tamanhos variados (de poucos KB a alguns MB) que somam `bulkBytes`.
        const weight = ((i * 7919) % 1000) + 1;
        const f = mk(drive, `tmp${(i * 2654435761 >>> 0).toString(16).slice(0, 6)}.tmp`, false, c.key, folder, Math.round((bulkBytes * weight) / BULK_WEIGHT));
        f.modified = now - (f.id % 730) * 864_000_000_000;
      }
    }
  }
  return root;
}

export const rootOf = (drive: string) => roots.get(drive);

function bytesOf(n: DNode, cat: CategoryKey | null): number {
  if (!n.isDir) {
    if (n.category === "sistema") return 0;
    return cat === null || cat === n.category ? n.alloc : 0;
  }
  return n.children.reduce((a, c) => a + bytesOf(c, cat), 0);
}

export function pathOf(n: DNode): string {
  const parts: string[] = [];
  for (let c: DNode | null = n; c && c.parent; c = c.parent) parts.unshift(c.name);
  return `${n.drive}:\\${parts.join("\\")}`;
}

function toRow(n: DNode, cat: CategoryKey | null): ItemRow {
  const all = bytesOf(n, null);
  return {
    node: n.id,
    frn: n.id,
    name: n.name,
    path: pathOf(n),
    isDir: n.isDir,
    size: all,
    alloc: all,
    bytes: bytesOf(n, cat),
    modified: n.modified,
    category: n.category,
    childCount: n.children.length,
  };
}

export function demoChildren(drive: string, node: number | null, cat: CategoryKey | null): ItemRow[] | null {
  const n = node === null ? roots.get(drive) : nodes.get(node);
  if (!n || !n.isDir || n.drive !== drive) return null;
  return n.children
    .map((c) => toRow(c, cat))
    .filter((r) => r.bytes > 0)
    .sort((a, b) => b.bytes - a.bytes || a.name.localeCompare(b.name));
}

function allFiles(n: DNode, out: DNode[] = []): DNode[] {
  for (const c of n.children) {
    if (c.isDir) allFiles(c, out);
    else if (c.category !== "sistema") out.push(c);
  }
  return out;
}

export function demoQuery(drive: string, f: FileFilter, sort: SortKey, offset: number, limit: number): Page | null {
  const root = roots.get(drive);
  if (!root) return null;
  const under = f.under !== undefined ? nodes.get(f.under) : undefined;
  const search = f.search?.toLowerCase();
  let files = allFiles(under ?? root).filter(
    (n) =>
      (!f.category || n.category === f.category) &&
      (!f.minSize || n.alloc >= f.minSize) &&
      (!f.modifiedBefore || n.modified < f.modifiedBefore) &&
      (!search || n.name.toLowerCase().includes(search)),
  );
  const cmp: Record<SortKey, (a: DNode, b: DNode) => number> = {
    sizeDesc: (a, b) => b.alloc - a.alloc,
    sizeAsc: (a, b) => a.alloc - b.alloc,
    nameAsc: (a, b) => a.name.localeCompare(b.name),
    nameDesc: (a, b) => b.name.localeCompare(a.name),
    modifiedDesc: (a, b) => b.modified - a.modified,
    modifiedAsc: (a, b) => a.modified - b.modified,
  };
  files = files.sort(cmp[sort]);
  return { total: files.length, rows: files.slice(offset, offset + limit).map((n) => toRow(n, null)) };
}

/** Tira um nó da árvore; devolve os bytes removidos por categoria (índice do Rust). */
export function removeNode(n: DNode): number[] {
  const out = Array(10).fill(0);
  const walk = (x: DNode) => {
    if (x.isDir) x.children.forEach(walk);
    else out[CATEGORIES.find((c) => c.key === x.category)!.index] += x.alloc;
    nodes.delete(x.id);
  };
  walk(n);
  if (n.parent) n.parent.children = n.parent.children.filter((c) => c !== n);
  return out;
}
