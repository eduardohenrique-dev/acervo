import { motion } from "motion/react";
import { useState } from "react";
import { formatBytes } from "../lib/format";
import { IconCopy, IconMore, IconMove, IconRename, IconReveal, IconTrash } from "./icons";

export type Action = "recycle" | "deletePermanent" | "move" | "copy" | "rename" | "reveal";

interface Props {
  count: number;
  bytes: number;
  onAction: (a: Action) => void;
  onClear: () => void;
}

const btn =
  "flex h-7 items-center gap-1.5 rounded-[var(--radius-control)] px-3 text-[12px] font-medium text-[var(--text-primary)] transition-colors hover:bg-[var(--hover)]";

/** Barra que aparece com itens selecionados. */
export function ActionBar({ count, bytes, onAction, onClear }: Props) {
  const [more, setMore] = useState(false);
  return (
    <motion.div
      role="toolbar"
      aria-label="Ações da seleção"
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: 12 }}
      transition={{ type: "spring", stiffness: 380, damping: 32 }}
      className="glass sticky bottom-4 z-20 mx-auto flex w-fit items-center gap-1 rounded-[var(--radius-panel)] p-1.5 pl-4"
    >
      <span className="num mr-2 text-[12px] text-[var(--text-secondary)]">
        <span className="font-semibold text-[var(--text-primary)]">{count === 1 ? "1 item" : `${count} itens`}</span> · {formatBytes(bytes)}
      </span>
      <button
        type="button"
        onClick={() => onAction("recycle")}
        className="flex h-7 items-center gap-1.5 rounded-[var(--radius-control)] bg-[var(--accent)] px-3 text-[12px] font-semibold text-white shadow-[0_1px_2px_rgba(0,0,0,0.2),inset_0_1px_0_rgba(255,255,255,0.25)] hover:brightness-110"
      >
        <IconTrash width={14} height={14} />
        Lixeira
      </button>
      <button type="button" className={btn} onClick={() => onAction("move")}>
        <IconMove width={14} height={14} />
        Mover para…
      </button>
      <button type="button" className={btn} onClick={() => onAction("copy")}>
        <IconCopy width={14} height={14} />
        Copiar para…
      </button>
      {count === 1 && (
        <button type="button" className={btn} onClick={() => onAction("rename")}>
          <IconRename width={14} height={14} />
          Renomear
        </button>
      )}
      <button type="button" className={btn} onClick={() => onAction("reveal")} title="Mostrar no Explorer">
        <IconReveal width={14} height={14} />
        Explorer
      </button>
      <div className="relative">
        <button type="button" aria-label="Mais ações" aria-expanded={more} className={`${btn} px-2`} onClick={() => setMore((v) => !v)}>
          <IconMore />
        </button>
        {more && (
          <div role="menu" className="glass absolute bottom-10 right-0 w-48 rounded-[var(--radius-panel)] p-1">
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                setMore(false);
                onAction("deletePermanent");
              }}
              className="flex h-7 w-full items-center gap-2 rounded-[var(--radius-chip)] px-2.5 text-left text-[12px] font-medium text-[var(--danger)] hover:bg-[var(--hover)]"
            >
              Apagar de vez…
            </button>
            <button
              type="button"
              role="menuitem"
              onClick={() => {
                setMore(false);
                onClear();
              }}
              className="flex h-7 w-full items-center gap-2 rounded-[var(--radius-chip)] px-2.5 text-left text-[12px] text-[var(--text-primary)] hover:bg-[var(--hover)]"
            >
              Limpar seleção
            </button>
          </div>
        )}
      </div>
    </motion.div>
  );
}
