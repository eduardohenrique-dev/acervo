import { AnimatePresence } from "motion/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ActionBar } from "../components/ActionBar";
import { IconChevron } from "../components/icons";
import { ItemList } from "../components/ItemList";
import type { DriveEntry } from "../hooks/driveState";
import { useItemActions } from "../hooks/useItemActions";
import type { ItemRow } from "../lib/api";
import { CATEGORIES, type CategoryKey } from "../lib/categories";
import { driveName } from "../lib/drive";
import { formatBytes } from "../lib/format";
import { nearestExisting, type Crumb } from "../lib/nav";
import { source } from "../lib/source";

interface Props {
  entry: DriveEntry;
  category: CategoryKey;
  onBack: () => void;
}

/** O que há dentro de uma categoria num disco, pasta por pasta. */
export function Category({ entry, category, onBack }: Props) {
  const drive = entry.view.info.letter;
  const def = CATEGORIES.find((c) => c.key === category)!;
  const [trail, setTrail] = useState<Crumb[]>([{ node: null, name: `${drive}:`, path: null }]);
  const [rows, setRows] = useState<ItemRow[] | null>(null);
  const missing = useRef(new Set<number>());
  // A seleção guarda a linha como estava quando foi escolhida (caminho e frn viajam juntos):
  // se o nó passar a ser outro arquivo, ele sai da seleção em vez de herdá-la.
  const [selected, setSelected] = useState<Map<number, ItemRow>>(new Map());
  const here = trail[trail.length - 1];

  const load = useCallback(async () => {
    const r = await source.categoryChildren(drive, here.node, here.path, category);
    if (r === null && here.node !== null) {
      missing.current.add(here.node);
      setTrail((t) => nearestExisting(t, (n) => n === null || !missing.current.has(n)));
      return;
    }
    const list = r ?? [];
    setRows(list);
    setSelected((s) => {
      if (s.size === 0) return s;
      const now = new Map(list.map((x) => [x.node, x]));
      const n = new Map([...s].filter(([id, row]) => now.get(id)?.frn === row.frn && now.get(id)?.path === row.path));
      return n.size === s.size ? s : n;
    });
  }, [drive, here.node, here.path, category]);

  useEffect(() => {
    void load();
  }, [load]);

  // Recarrega quando o disco muda (ao vivo) ou uma operação termina.
  useEffect(() => {
    const offs = [
      source.onIndexUpdated((e) => e.drive === drive && void load()),
      // Uma nova análise troca a árvore inteira: os nós da lista antiga não valem mais.
      source.onIndexReady((e) => e.drive === drive && void load()),
      source.onOpDone(() => void load()),
    ];
    return () => offs.forEach((p) => void p.then((off) => off()));
  }, [drive, load]);

  const clear = useCallback(() => setSelected(new Map()), []);
  const toggle = useCallback(
    (node: number) =>
      setSelected((s) => {
        const n = new Map(s);
        const row = rows?.find((r) => r.node === node);
        if (n.has(node)) n.delete(node);
        else if (row) n.set(node, row);
        return n;
      }),
    [rows],
  );
  const setAll = useCallback((list: ItemRow[]) => setSelected(new Map(list.map((r) => [r.node, r]))), []);
  const selectedRows = useMemo(() => [...selected.values()], [selected]);
  const { request, dialog, notice, dismissNotice } = useItemActions(drive, selectedRows, () => void load(), clear);

  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement)?.tagName === "INPUT") return;
      if (e.key === "Delete" && selectedRows.length) {
        e.preventDefault();
        void request(e.shiftKey ? "deletePermanent" : "recycle");
      }
      if (e.key === "a" && e.ctrlKey && rows) {
        e.preventDefault();
        setAll(rows);
      }
      if (e.key === "Backspace" && trail.length > 1) setTrail((t) => t.slice(0, -1));
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, [selectedRows, rows, request, setAll, trail.length]);

  const open = (r: ItemRow) => {
    clear();
    setTrail((t) => [...t, { node: r.node, name: r.name, path: r.path }]);
  };

  const total = entry.summary?.byCategory[def.index] ?? 0;
  const selectedBytes = selectedRows.reduce((a, r) => a + r.alloc, 0);

  return (
    <div className="mx-auto flex max-w-[960px] flex-col gap-3 pb-4">
      <header className="pt-2">
        <nav aria-label="Caminho" className="flex flex-wrap items-center gap-1 text-[12px] text-[var(--text-secondary)]">
          <button type="button" onClick={onBack} className="rounded-[var(--radius-chip)] px-1 hover:text-[var(--text-primary)]">
            Armazenamento
          </button>
          {trail.map((c, i) => (
            <span key={`${c.node}-${i}`} className="flex items-center gap-1">
              <IconChevron width={10} height={10} className="text-[var(--text-tertiary)]" />
              <button
                type="button"
                disabled={i === trail.length - 1}
                onClick={() => setTrail(trail.slice(0, i + 1))}
                className="max-w-[220px] truncate rounded-[var(--radius-chip)] px-1 hover:text-[var(--text-primary)] disabled:font-semibold disabled:text-[var(--text-primary)]"
              >
                {i === 0 ? driveName(entry.view.info) : c.name}
              </button>
            </span>
          ))}
        </nav>
        <div className="mt-2 flex items-baseline gap-3">
          <span className="h-3 w-3 rounded-full" style={{ background: `var(${def.colorVar})` }} />
          <h1 className="text-[length:var(--text-large-title)] font-bold leading-tight tracking-[var(--tracking-title)]">{def.label}</h1>
          <p className="num text-[length:var(--text-body)] text-[var(--text-secondary)]">{formatBytes(total)} neste disco</p>
        </div>
      </header>

      {notice && (
        <div role="status" className="glass-strong flex items-start justify-between gap-3 rounded-[var(--radius-panel)] px-3 py-2 text-[length:var(--text-body)]">
          <span>{notice}</span>
          <button type="button" onClick={dismissNotice} className="text-[12px] text-[var(--text-secondary)] hover:text-[var(--text-primary)]">
            Fechar
          </button>
        </div>
      )}

      <section aria-label={`Conteúdo de ${here.name}`} className="glass rounded-[var(--radius-panel)] p-2">
        {rows === null && <p className="px-3 py-4 text-[length:var(--text-body)] text-[var(--text-secondary)]">Carregando…</p>}
        {rows && rows.length === 0 && (
          <p className="px-3 py-4 text-[length:var(--text-body)] text-[var(--text-secondary)]">Nada de {def.label.toLowerCase()} aqui.</p>
        )}
        {rows && rows.length > 0 && (
          <ItemList
            rows={rows}
            selected={selected}
            onToggle={toggle}
            onOpen={open}
            onContext={(r) => (selected.has(r.node) ? undefined : setAll([r]))}
            colorVar={def.colorVar}
          />
        )}
      </section>

      <AnimatePresence>
        {selectedRows.length > 0 && <ActionBar count={selectedRows.length} bytes={selectedBytes} onAction={(a) => void request(a)} onClear={clear} />}
      </AnimatePresence>
      {dialog}
    </div>
  );
}
