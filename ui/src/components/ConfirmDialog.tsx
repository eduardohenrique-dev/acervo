import { motion } from "motion/react";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { formatBytes } from "../lib/format";

export type ConfirmMode = "recycle" | "permanent" | "move" | "rename";

interface Props {
  mode: ConfirmMode;
  count: number;
  bytes: number;
  /** Avisos (ex.: "isto pertence a um programa"). */
  warnings: string[];
  /** Destino, no modo mover. */
  dest?: string;
  /** Nome atual, no modo renomear. */
  currentName?: string;
  /** Nomes dos itens (os primeiros aparecem no diálogo). */
  names?: string[];
  onConfirm: (value?: string) => void;
  onCancel: () => void;
}

const what = (count: number, bytes: number) => `${count === 1 ? "1 item" : `${count} itens`} (${formatBytes(bytes)})`;

/** Confirmação de ação destrutiva. "Apagar de vez" exige marcar a caixa. */
/** Quantos nomes aparecem antes de "e mais N". */
const SHOWN = 3;

export function ConfirmDialog({ mode, count, bytes, warnings, dest, currentName, names = [], onConfirm, onCancel }: Props) {
  const [understood, setUnderstood] = useState(false);
  const [name, setName] = useState(currentName ?? "");
  const primary = useRef<HTMLButtonElement>(null);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (mode === "rename") {
      input.current?.focus();
      input.current?.setSelectionRange(0, (currentName ?? "").lastIndexOf(".") > 0 ? (currentName ?? "").lastIndexOf(".") : (currentName ?? "").length);
    } else if (mode !== "permanent") primary.current?.focus();
    const esc = (e: KeyboardEvent) => e.key === "Escape" && onCancel();
    window.addEventListener("keydown", esc);
    return () => window.removeEventListener("keydown", esc);
  }, [mode, currentName, onCancel]);

  const title = {
    recycle: "Mandar para a Lixeira?",
    permanent: "Apagar de vez?",
    move: "Mover?",
    rename: "Renomear",
  }[mode];

  const body = {
    recycle: `${what(count, bytes)} vão para a Lixeira. Dá para restaurar de lá enquanto ela não for esvaziada.`,
    permanent: `${what(count, bytes)} não vão para a Lixeira. Isto não dá para desfazer.`,
    move: `${what(count, bytes)} vão para ${dest ?? ""}.`,
    rename: "Novo nome:",
  }[mode];

  const canConfirm = mode === "permanent" ? understood : mode === "rename" ? name.trim() !== "" && name !== currentName : true;
  const confirmLabel = { recycle: "Mandar para a Lixeira", permanent: "Apagar de vez", move: "Mover", rename: "Renomear" }[mode];

  // No body: dentro de um ancestral com transform, `fixed` ficaria preso a ele.
  return createPortal(
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/25" onMouseDown={(e) => e.target === e.currentTarget && onCancel()}>
      <motion.div
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="confirm-title"
        initial={{ opacity: 0, scale: 0.96 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ type: "spring", stiffness: 420, damping: 32 }}
        className="glass w-[400px] rounded-[var(--radius-panel)] p-5"
      >
        <h2 id="confirm-title" className="text-[length:var(--text-title)] font-semibold tracking-[-0.012em]">
          {title}
        </h2>
        <p className="num mt-2 text-[length:var(--text-body)] text-[var(--text-secondary)]">{body}</p>
        {mode !== "rename" && names.length > 0 && (
          <ul aria-label="Itens" className="mt-3 flex flex-col gap-0.5 rounded-[var(--radius-control)] bg-[var(--hover)] px-3 py-1.5">
            {names.slice(0, SHOWN).map((n, i) => (
              <li key={`${n}-${i}`} className="truncate text-[12px] font-medium text-[var(--text-primary)]" title={n}>
                {n}
              </li>
            ))}
            {names.length > SHOWN && (
              <li className="num text-[12px] text-[var(--text-secondary)]">e mais {names.length - SHOWN}</li>
            )}
          </ul>
        )}
        {mode === "rename" && (
          <input
            ref={input}
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && canConfirm && onConfirm(name.trim())}
            className="mt-3 h-8 w-full rounded-[var(--radius-control)] border border-[var(--separator)] bg-[var(--glass-fill)] px-3 text-[length:var(--text-body)] outline-none focus:border-[var(--accent)]"
          />
        )}
        {warnings.map((w) => (
          <p key={w} className="mt-3 rounded-[var(--radius-control)] bg-[color-mix(in_srgb,var(--cat-documentos)_14%,transparent)] px-3 py-1.5 text-[12px] text-[var(--text-primary)]">
            {w}
          </p>
        ))}
        {mode === "permanent" && (
          <label className="mt-3 flex cursor-pointer items-start gap-2.5 text-[12px] text-[var(--text-primary)]">
            <input type="checkbox" checked={understood} onChange={(e) => setUnderstood(e.target.checked)} className="mt-0.5 h-4 w-4 accent-[var(--danger)]" />
            <span className="num">Entendo que {count === 1 ? "este item" : `estes ${count} itens`} ({formatBytes(bytes)}) somem de vez.</span>
          </label>
        )}
        <div className="mt-5 flex justify-end gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="h-8 rounded-[var(--radius-control)] px-3 text-[12px] font-medium text-[var(--text-primary)] hover:bg-[var(--hover)]"
          >
            Cancelar
          </button>
          <button
            ref={primary}
            type="button"
            disabled={!canConfirm}
            onClick={() => onConfirm(mode === "rename" ? name.trim() : undefined)}
            className={`h-8 rounded-[var(--radius-control)] px-3 text-[12px] font-semibold text-white shadow-[0_1px_2px_rgba(0,0,0,0.2),inset_0_1px_0_rgba(255,255,255,0.25)] transition-[filter,opacity] hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-40 ${
              mode === "permanent" ? "bg-[var(--danger)]" : "bg-[var(--accent)]"
            }`}
          >
            {confirmLabel}
          </button>
        </div>
      </motion.div>
    </div>,
    document.body,
  );
}
