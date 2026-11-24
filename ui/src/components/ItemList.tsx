import type { ItemRow } from "../lib/api";
import { formatBytes } from "../lib/format";
import { IconChevron, IconFile, IconFolder } from "./icons";

interface Props {
  rows: ItemRow[];
  selected: { has: (node: number) => boolean };
  onToggle: (node: number) => void;
  onOpen: (row: ItemRow) => void;
  onContext: (row: ItemRow, x: number, y: number) => void;
  /** Cor da barra (variável CSS da categoria). */
  colorVar: string;
}

/** Lista de pastas e arquivos com barra proporcional. Clique seleciona; duplo clique abre a pasta. */
export function ItemList({ rows, selected, onToggle, onOpen, onContext, colorVar }: Props) {
  const max = Math.max(1, ...rows.map((r) => r.bytes));
  return (
    <ul role="listbox" aria-multiselectable="true" aria-label="Itens" className="flex flex-col">
      {rows.map((r) => {
        const on = selected.has(r.node);
        return (
          <li
            key={r.node}
            role="option"
            aria-selected={on}
            tabIndex={0}
            onClick={() => onToggle(r.node)}
            onDoubleClick={() => r.isDir && onOpen(r)}
            onKeyDown={(e) => {
              if (e.key === " ") {
                e.preventDefault();
                onToggle(r.node);
              }
              if (e.key === "Enter" && r.isDir) onOpen(r);
            }}
            onContextMenu={(e) => {
              e.preventDefault();
              onContext(r, e.clientX, e.clientY);
            }}
            className={`group grid cursor-default grid-cols-[18px_18px_minmax(0,1fr)_minmax(80px,180px)_80px_24px] items-center gap-3 rounded-[var(--radius-control)] px-3 py-1.5 transition-colors duration-150 ${
              on ? "bg-[var(--accent-soft)]" : "hover:bg-[var(--hover)]"
            }`}
          >
            <span
              aria-hidden="true"
              className={`grid h-[16px] w-[16px] place-items-center rounded-[var(--radius-chip)] border transition-colors ${
                on ? "border-[var(--accent)] bg-[var(--accent)] text-white" : "border-[var(--separator)] bg-[var(--glass-fill)]"
              }`}
            >
              {on && (
                <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                  <path d="m2 5.2 2 2L8 3" />
                </svg>
              )}
            </span>
            <span className="text-[var(--text-secondary)]">{r.isDir ? <IconFolder /> : <IconFile />}</span>
            <span className="min-w-0">
              <span className="block truncate text-[length:var(--text-body)] text-[var(--text-primary)]" title={r.path}>
                {r.name}
              </span>
              {r.isDir && (
                <span className="num block text-[length:var(--text-caption)] text-[var(--text-tertiary)]">
                  {r.childCount === 1 ? "1 item" : `${r.childCount} itens`}
                </span>
              )}
            </span>
            <span className="capsule !h-1.5 !p-0">
              <span
                className="block h-full rounded-[inherit]"
                style={{ width: `${(r.bytes / max) * 100}%`, background: `var(${colorVar})` }}
              />
            </span>
            <span className="num text-right text-[length:var(--text-body)] text-[var(--text-primary)]">{formatBytes(r.bytes)}</span>
            {r.isDir ? (
              <button
                type="button"
                aria-label={`Abrir ${r.name}`}
                onClick={(e) => {
                  e.stopPropagation();
                  onOpen(r);
                }}
                className="grid h-6 w-6 place-items-center rounded-[var(--radius-chip)] text-[var(--text-tertiary)] hover:bg-[var(--hover)] hover:text-[var(--text-primary)]"
              >
                <IconChevron width={12} height={12} />
              </button>
            ) : (
              <span />
            )}
          </li>
        );
      })}
    </ul>
  );
}
