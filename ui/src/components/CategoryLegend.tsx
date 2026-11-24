import { CATEGORIES, type CategoryKey } from "../lib/categories";
import { formatBytes } from "../lib/format";
import { IconChevron } from "./icons";

interface Props {
  byCategory: number[];
  active: CategoryKey | null;
  onActive: (key: CategoryKey | null) => void;
  /** Abre a categoria (o Sistema abre o detalhamento). */
  onOpen?: (key: CategoryKey) => void;
}

/** Legenda da cápsula. Cada item destaca seu segmento e abre o que há dentro da categoria. */
export function CategoryLegend({ byCategory, active, onActive, onOpen }: Props) {
  const items = CATEGORIES.filter((c) => (byCategory[c.index] ?? 0) > 0);
  return (
    <ul className="-mx-1.5 flex flex-wrap gap-x-1.5 gap-y-1" onMouseLeave={() => onActive(null)}>
      {items.map((c) => (
        <li key={c.key} className={`transition-opacity duration-150 ${active && active !== c.key ? "opacity-45" : ""}`}>
          <button
            type="button"
            onMouseEnter={() => onActive(c.key)}
            onFocus={() => onActive(c.key)}
            onBlur={() => onActive(null)}
            onClick={() => onOpen?.(c.key)}
            aria-label={`${c.label}: ${formatBytes(byCategory[c.index])}. Ver o que tem dentro`}
            className="group flex h-6 cursor-pointer items-center gap-1.5 rounded-[var(--radius-chip)] px-1.5 text-[length:var(--text-caption)] hover:bg-[var(--hover)]"
          >
            <span
              className="h-2.5 w-2.5 shrink-0 rounded-full shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.15)]"
              style={{ background: `var(${c.colorVar})` }}
            />
            <span className="text-[var(--text-primary)]">{c.label}</span>
            <span className="num text-[var(--text-tertiary)]">{formatBytes(byCategory[c.index])}</span>
            <IconChevron className="text-[var(--text-tertiary)] opacity-60 transition-opacity group-hover:opacity-100" />
          </button>
        </li>
      ))}
    </ul>
  );
}
