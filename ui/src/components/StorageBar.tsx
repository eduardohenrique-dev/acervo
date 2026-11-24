import { motion } from "motion/react";
import { segmentWidths } from "../lib/bar";
import { CATEGORIES, type CategoryKey } from "../lib/categories";
import { formatBytes, formatPercent } from "../lib/format";

interface Props {
  byCategory: number[];
  total: number;
  active: CategoryKey | null;
  onActive: (key: CategoryKey | null) => void;
  /** Chamado ao clicar num segmento (abre a categoria; o Sistema abre o detalhamento). */
  onSelect?: (key: CategoryKey) => void;
}

const SPRING = { type: "spring" as const, stiffness: 170, damping: 26, mass: 0.9 };
const label = (key: CategoryKey) => CATEGORIES.find((c) => c.key === key)!.label;

/** Tubo de vidro com o líquido das categorias, que termina num menisco antes da parte livre. */
export function StorageBar({ byCategory, total, active, onActive, onSelect }: Props) {
  const segments = segmentWidths(byCategory, total);
  const liquid = segments.reduce((a, s) => a + s.percent, 0);
  const summary = segments.map((s) => `${label(s.key)} ${formatBytes(s.bytes)}`).join(", ");

  return (
    <div
      role="img"
      aria-label={`Uso do disco: ${summary}`}
      className="capsule"
      data-hovering={active ? "true" : "false"}
      onMouseMove={(e) => {
        const r = e.currentTarget.getBoundingClientRect();
        e.currentTarget.style.setProperty("--mx", `${((e.clientX - r.left) / r.width) * 100}%`);
      }}
      onMouseLeave={() => onActive(null)}
    >
      <motion.div className="capsule-liquid" initial={{ width: 0 }} animate={{ width: `${liquid}%` }} transition={SPRING}>
        {segments.map((s) => {
          const clickable = Boolean(onSelect);
          return (
            <motion.div
              key={s.key}
              className={`capsule-segment ${clickable ? "cursor-pointer" : ""}`}
              data-active={s.key === active}
              style={{ ["--seg-color" as string]: `var(--cat-${s.key})` }}
              animate={{ width: `${liquid > 0 ? (s.percent / liquid) * 100 : 0}%` }}
              transition={SPRING}
              onMouseEnter={() => onActive(s.key)}
              onClick={onSelect ? () => onSelect(s.key) : undefined}
            />
          );
        })}
      </motion.div>
      <div className="capsule-shine" />
    </div>
  );
}

/** Linha do cabeçalho com a categoria em destaque (substitui o "X de Y usados"). */
export function ActiveCategory({ categoryKey, bytes, total }: { categoryKey: CategoryKey; bytes: number; total: number }) {
  return (
    <p role="status" className="num flex items-center gap-2 text-[length:var(--text-body)] text-[var(--text-secondary)]">
      <span className="h-2.5 w-2.5 rounded-full" style={{ background: `var(--cat-${categoryKey})` }} />
      <span className="font-semibold text-[var(--text-primary)]">{label(categoryKey)}</span>
      {formatBytes(bytes)} · {formatPercent(bytes / total)} do disco
    </p>
  );
}
