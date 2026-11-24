import { AnimatePresence } from "motion/react";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { ActionBar } from "../components/ActionBar";
import { IconFile } from "../components/icons";
import type { DriveEntry } from "../hooks/driveState";
import { useItemActions } from "../hooks/useItemActions";
import type { FileFilter, ItemRow, SortKey } from "../lib/api";
import { CATEGORIES, type CategoryKey } from "../lib/categories";
import { driveName } from "../lib/drive";
import { formatBytes, formatFiletime, toFiletime } from "../lib/format";
import { PAGE, PageCache, pagesFor } from "../lib/paging";
import { source } from "../lib/source";

/** Altura fixa de cada linha: a lista só põe no DOM as que aparecem. */
const ROW = 30;
/** Linhas a mais desenhadas acima e abaixo da janela. */
const OVERSCAN = 8;

const MB = 1024 * 1024;
const SIZES = [
  { label: "Qualquer tamanho", value: 0 },
  { label: "Maior que 1 MB", value: MB },
  { label: "Maior que 100 MB", value: 100 * MB },
  { label: "Maior que 1 GB", value: 1024 * MB },
];
const AGES = [
  { label: "Qualquer data", months: 0 },
  { label: "Sem mexer há mais de 6 meses", months: 6 },
  { label: "Sem mexer há mais de 1 ano", months: 12 },
];

const COLS = "grid-cols-[18px_18px_minmax(0,1.5fr)_minmax(0,1fr)_150px_92px_84px]";
const COUNT = new Intl.NumberFormat("pt-BR");

/** Todos os arquivos de um disco, do maior ao menor (ou do menor ao maior), com busca e filtros. */
export function Files({ drives }: { drives: DriveEntry[] }) {
  const ready = drives.filter((d) => d.view.status.state === "ready");
  const [chosen, setChosen] = useState<string | null>(null);
  const drive = ready.find((d) => d.view.info.letter === chosen)?.view.info.letter ?? ready[0]?.view.info.letter ?? null;

  const [sort, setSort] = useState<SortKey>("sizeDesc");
  const [typed, setTyped] = useState("");
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState<CategoryKey | "">("");
  const [minSize, setMinSize] = useState(0);
  const [months, setMonths] = useState(0);

  useEffect(() => {
    const t = setTimeout(() => setSearch(typed.trim()), 200);
    return () => clearTimeout(t);
  }, [typed]);

  const filter = useMemo<FileFilter>(() => {
    const f: FileFilter = {};
    if (category) f.category = category;
    if (minSize) f.minSize = minSize;
    if (search) f.search = search;
    if (months) {
      const d = new Date();
      d.setMonth(d.getMonth() - months);
      f.modifiedBefore = toFiletime(d.getTime());
    }
    return f;
  }, [category, minSize, search, months]);

  // Tudo o que muda o resultado. Trocar a chave descarta as páginas e volta ao topo.
  const key = JSON.stringify([drive, filter.category, filter.minSize, filter.search, months, sort]);

  const cache = useRef(new PageCache());
  /** Páginas da versão anterior, mostradas enquanto as novas chegam (evita piscar no tique ao vivo). */
  const stale = useRef<PageCache | null>(null);
  const inflight = useRef(new Set<number>());
  const generation = useRef(0);
  const [total, setTotal] = useState<number | null>(null);
  const [, setTick] = useState(0);

  const scroller = useRef<HTMLDivElement>(null);
  const [view, setView] = useState({ top: 0, height: 600 });

  const [selected, setSelected] = useState<Map<number, ItemRow>>(new Map());
  const clearSel = useCallback(() => setSelected(new Map()), []);

  // Filtro, ordenação ou disco mudaram: começa do zero.
  useLayoutEffect(() => {
    generation.current++;
    cache.current = new PageCache();
    stale.current = null;
    inflight.current.clear();
    setTotal(null);
    setSelected(new Map());
    if (scroller.current) scroller.current.scrollTop = 0;
    setView((v) => ({ ...v, top: 0 }));
  }, [key]);

  /** O disco mudou (ao vivo ou por uma operação): busca de novo, mostrando o que havia até chegar. */
  const refresh = useCallback(() => {
    generation.current++;
    stale.current = cache.current;
    cache.current = new PageCache();
    inflight.current.clear();
    setTick((t) => t + 1);
  }, []);

  useEffect(() => {
    if (!drive) return;
    const offs = [
      source.onIndexUpdated((e) => e.drive === drive && refresh()),
      // Uma nova análise troca a árvore inteira: a seleção feita sobre a lista antiga não vale mais.
      source.onIndexReady((e) => {
        if (e.drive !== drive) return;
        setSelected(new Map());
        refresh();
      }),
      source.onOpDone(() => refresh()),
    ];
    return () => offs.forEach((p) => void p.then((off) => off()));
  }, [drive, refresh]);

  useLayoutEffect(() => {
    const el = scroller.current;
    if (!el) return;
    const measure = () => setView({ top: el.scrollTop, height: el.clientHeight });
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, [drive]);

  const first = Math.floor(view.top / ROW);
  const last = Math.ceil((view.top + view.height) / ROW);

  // Busca as páginas que cobrem a janela visível.
  useEffect(() => {
    if (!drive) return;
    const gen = generation.current;
    const maxPage = total === null ? 0 : Math.max(0, Math.ceil(total / PAGE) - 1);
    for (const p of pagesFor(first, last)) {
      if (p > maxPage || cache.current.has(p) || inflight.current.has(p)) continue;
      inflight.current.add(p);
      source
        .queryFiles(drive, filter, sort, p * PAGE, PAGE)
        .then((page) => {
          if (gen !== generation.current) return;
          inflight.current.delete(p);
          cache.current.set(p, page?.rows ?? []);
          setTotal(page?.total ?? 0);
          setTick((t) => t + 1);
        })
        .catch(() => {
          if (gen === generation.current) inflight.current.delete(p);
        });
    }
  });

  const rowAt = (i: number): ItemRow | undefined => {
    const p = Math.floor(i / PAGE);
    return (cache.current.get(p) ?? stale.current?.get(p))?.[i % PAGE];
  };

  const toggle = (r: ItemRow) =>
    setSelected((s) => {
      const n = new Map(s);
      if (n.has(r.node)) n.delete(r.node);
      else n.set(r.node, r);
      return n;
    });

  const selectedRows = useMemo(() => [...selected.values()], [selected]);
  const { request, dialog, notice, dismissNotice } = useItemActions(drive ?? "", selectedRows, refresh, clearSel);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement)?.tagName === "INPUT" || (e.target as HTMLElement)?.tagName === "SELECT") return;
      if (e.key === "Delete" && selectedRows.length) {
        e.preventDefault();
        void request(e.shiftKey ? "deletePermanent" : "recycle");
      }
      if (e.key === "Escape" && selectedRows.length && !document.querySelector("[role=alertdialog]")) clearSel();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [selectedRows, request, clearSel]);

  const sortBy = (asc: SortKey, desc: SortKey, startDesc: boolean) =>
    setSort((s) => (s === desc ? asc : s === asc ? desc : startDesc ? desc : asc));

  const count = total ?? 0;
  const from = Math.max(0, first - OVERSCAN);
  const to = Math.min(count, last + OVERSCAN);
  const selectedBytes = selectedRows.reduce((a, r) => a + r.alloc, 0);
  const filtered = !!(category || minSize || search || months);

  return (
    <div className="mx-auto flex h-full max-w-[1080px] flex-col gap-3">
      <header className="flex items-end justify-between gap-3 pt-2">
        <div>
          <h1 className="text-[length:var(--text-large-title)] font-bold leading-tight tracking-[var(--tracking-title)]">Arquivos</h1>
          <p className="num mt-1 text-[length:var(--text-body)] text-[var(--text-secondary)]">
            {drive === null
              ? "Esperando a análise dos discos…"
              : total === null
                ? "Contando…"
                : `${COUNT.format(total)} ${total === 1 ? "arquivo" : "arquivos"}${filtered ? " com esses filtros" : ""}`}
          </p>
        </div>
        {ready.length > 1 && (
          <div role="radiogroup" aria-label="Disco" className="glass-strong flex gap-0.5 rounded-[var(--radius-panel)] p-1">
            {ready.map((d) => {
              const on = d.view.info.letter === drive;
              return (
                <button
                  key={d.view.info.letter}
                  type="button"
                  role="radio"
                  aria-checked={on}
                  onClick={() => setChosen(d.view.info.letter)}
                  className={`h-6 rounded-[var(--radius-chip)] px-3 text-[12px] transition-colors ${
                    on ? "bg-[var(--selected)] font-semibold text-[var(--text-primary)]" : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                  }`}
                >
                  {driveName(d.view.info)}
                </button>
              );
            })}
          </div>
        )}
      </header>

      <div className="flex items-center gap-2">
        <input
          type="search"
          value={typed}
          onChange={(e) => setTyped(e.target.value)}
          placeholder="Buscar pelo nome"
          aria-label="Buscar pelo nome"
          className={`${field} min-w-[140px] max-w-[260px] flex-1 px-3 placeholder:text-[var(--text-tertiary)]`}
        />
        <Select label="Categoria" value={category} onChange={(v) => setCategory(v as CategoryKey | "")}>
          <option value="">Todas as categorias</option>
          {CATEGORIES.filter((c) => c.key !== "sistema").map((c) => (
            <option key={c.key} value={c.key}>
              {c.label}
            </option>
          ))}
        </Select>
        <Select label="Tamanho" value={String(minSize)} onChange={(v) => setMinSize(Number(v))}>
          {SIZES.map((s) => (
            <option key={s.value} value={s.value}>
              {s.label}
            </option>
          ))}
        </Select>
        <Select label="Data de modificação" value={String(months)} onChange={(v) => setMonths(Number(v))}>
          {AGES.map((a) => (
            <option key={a.months} value={a.months}>
              {a.label}
            </option>
          ))}
        </Select>
        {filtered && (
          <button
            type="button"
            onClick={() => {
              setTyped("");
              setSearch("");
              setCategory("");
              setMinSize(0);
              setMonths(0);
            }}
            className="h-7 shrink-0 rounded-[var(--radius-control)] px-3 text-[12px] text-[var(--accent)] hover:bg-[var(--hover)]"
          >
            Limpar filtros
          </button>
        )}
      </div>

      {notice && (
        <div role="status" className="glass-strong flex items-start justify-between gap-3 rounded-[var(--radius-panel)] px-3 py-2 text-[length:var(--text-body)]">
          <span>{notice}</span>
          <button type="button" onClick={dismissNotice} className="text-[12px] text-[var(--text-secondary)] hover:text-[var(--text-primary)]">
            Fechar
          </button>
        </div>
      )}

      <section aria-label="Lista de arquivos" className="glass flex min-h-0 flex-1 flex-col rounded-[var(--radius-panel)] p-2">
        <div role="row" className={`grid ${COLS} items-center gap-3 border-b border-[var(--separator)] px-3 pb-2 pt-1 text-[11px] font-medium text-[var(--text-secondary)]`}>
          <span />
          <span />
          <SortHeader label="Nome" active={sort === "nameAsc" || sort === "nameDesc"} asc={sort === "nameAsc"} onClick={() => sortBy("nameAsc", "nameDesc", false)} />
          <span>Pasta</span>
          <span>Categoria</span>
          <SortHeader label="Modificado" active={sort === "modifiedAsc" || sort === "modifiedDesc"} asc={sort === "modifiedAsc"} onClick={() => sortBy("modifiedAsc", "modifiedDesc", true)} />
          <SortHeader label="Tamanho" align="right" active={sort === "sizeAsc" || sort === "sizeDesc"} asc={sort === "sizeAsc"} onClick={() => sortBy("sizeAsc", "sizeDesc", true)} />
        </div>

        <div
          ref={scroller}
          onScroll={(e) => setView({ top: e.currentTarget.scrollTop, height: e.currentTarget.clientHeight })}
          className="relative min-h-0 flex-1 overflow-y-auto"
        >
          {total === 0 && (
            <p className="px-3 py-4 text-[length:var(--text-body)] text-[var(--text-secondary)]">
              {filtered ? "Nenhum arquivo com esses filtros." : "Nenhum arquivo neste disco."}
            </p>
          )}
          <ul role="listbox" aria-multiselectable="true" aria-label="Arquivos" aria-rowcount={count} style={{ height: count * ROW }} className="relative">
            {Array.from({ length: Math.max(0, to - from) }, (_, k) => {
              const i = from + k;
              const r = rowAt(i);
              return r ? (
                <FileRow key={r.node} row={r} top={i * ROW} index={i} on={selected.has(r.node)} onToggle={() => toggle(r)} onReveal={() => void source.revealItem({ drive: drive!, node: r.node, frn: r.frn, path: r.path }).catch(() => refresh())} />
              ) : (
                <li key={`p${i}`} aria-hidden="true" style={{ top: i * ROW, height: ROW }} className="absolute inset-x-0 flex items-center px-3">
                  <span className="h-2.5 w-1/3 rounded-full bg-[var(--hover)]" />
                </li>
              );
            })}
          </ul>
        </div>
      </section>

      <AnimatePresence>
        {selectedRows.length > 0 && <ActionBar count={selectedRows.length} bytes={selectedBytes} onAction={(a) => void request(a)} onClear={clearSel} />}
      </AnimatePresence>
      {dialog}
    </div>
  );
}

const field =
  "h-7 rounded-[var(--radius-control)] border border-[var(--separator)] bg-[var(--glass-fill)] text-[12px] text-[var(--text-primary)] outline-none focus:border-[var(--accent)]";

function Select({ label, value, onChange, children }: { label: string; value: string; onChange: (v: string) => void; children: ReactNode }) {
  return (
    <select aria-label={label} value={value} onChange={(e) => onChange(e.target.value)} className={`${field} shrink-0 cursor-pointer px-2.5`}>
      {children}
    </select>
  );
}

function SortHeader({ label, active, asc, onClick, align }: { label: string; active: boolean; asc: boolean; onClick: () => void; align?: "right" }) {
  return (
    <button
      type="button"
      role="columnheader"
      aria-sort={active ? (asc ? "ascending" : "descending") : "none"}
      onClick={onClick}
      className={`flex items-center gap-1 rounded-[var(--radius-chip)] hover:text-[var(--text-primary)] ${align === "right" ? "justify-end" : ""} ${active ? "text-[var(--text-primary)]" : ""}`}
    >
      {label}
      {active && (
        <svg width="8" height="8" viewBox="0 0 8 8" aria-hidden="true" className={asc ? "" : "rotate-180"}>
          <path d="M1 5.5 4 2.5l3 3" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      )}
    </button>
  );
}

interface RowProps {
  row: ItemRow;
  top: number;
  index: number;
  on: boolean;
  onToggle: () => void;
  onReveal: () => void;
}

function FileRow({ row: r, top, index, on, onToggle, onReveal }: RowProps) {
  const def = CATEGORIES.find((c) => c.key === r.category);
  const folder = r.path.slice(0, Math.max(0, r.path.length - r.name.length - 1));
  return (
    <li
      role="option"
      aria-selected={on}
      aria-rowindex={index + 1}
      tabIndex={0}
      onClick={onToggle}
      onDoubleClick={onReveal}
      onKeyDown={(e) => {
        if (e.key === " ") {
          e.preventDefault();
          onToggle();
        }
        if (e.key === "Enter") onReveal();
      }}
      style={{ top, height: ROW }}
      className={`absolute inset-x-0 grid ${COLS} cursor-default items-center gap-3 rounded-[var(--radius-chip)] px-3 transition-colors duration-100 ${
        on ? "bg-[var(--accent-soft)]" : "hover:bg-[var(--hover)]"
      }`}
    >
      <span
        aria-hidden="true"
        className={`grid h-4 w-4 place-items-center rounded-[4px] border ${
          on ? "border-[var(--accent)] bg-[var(--accent)] text-white" : "border-[var(--separator)] bg-[var(--glass-fill)]"
        }`}
      >
        {on && (
          <svg width="9" height="9" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
            <path d="m2 5.2 2 2L8 3" />
          </svg>
        )}
      </span>
      <span className="text-[var(--text-tertiary)]">
        <IconFile width={14} height={14} />
      </span>
      <span className="truncate text-[length:var(--text-body)] text-[var(--text-primary)]" title={r.name}>
        {r.name}
      </span>
      {/* Corta o começo do caminho: o fim ("…\common\Elden Ring") é o que diz onde o arquivo está. */}
      <span dir="rtl" className="truncate text-left text-[12px] text-[var(--text-secondary)]" title={folder}>
        <bdi dir="ltr">{folder}</bdi>
      </span>
      <span className="flex min-w-0 items-center gap-1.5 text-[12px] text-[var(--text-secondary)]">
        <span className="h-2 w-2 shrink-0 rounded-full" style={{ background: def ? `var(${def.colorVar})` : undefined }} />
        <span className="truncate">{def?.label ?? r.category}</span>
      </span>
      <span className="num text-[12px] text-[var(--text-secondary)]">{formatFiletime(r.modified)}</span>
      <span className="num text-right text-[length:var(--text-body)] text-[var(--text-primary)]">{formatBytes(r.alloc)}</span>
    </li>
  );
}
