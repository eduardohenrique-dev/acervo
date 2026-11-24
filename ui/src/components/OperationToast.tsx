import { AnimatePresence, motion } from "motion/react";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { OpRecord } from "../lib/api";
import { failedCount, opResult, opRunning } from "../lib/opText";

interface Props {
  running: OpRecord | null;
  /** A última operação terminada. */
  last: OpRecord | null;
  onDetails: (id: number) => void;
}

/** Quanto o aviso de sucesso fica na tela. */
const HIDE_MS = 6000;

/**
 * Cartão no canto inferior direito: o progresso da operação e, no fim, o resultado.
 * Some sozinho se deu tudo certo; com falha, fica até a pessoa fechar ou ver os detalhes.
 */
export function OperationToast({ running, last, onDetails }: Props) {
  // Só mostra resultados de operações terminadas depois que o app abriu.
  const seen = useRef<number | null>(null);
  const [mountedAt] = useState(() => Date.now() / 1000);
  const [result, setResult] = useState<OpRecord | null>(null);

  useEffect(() => {
    if (!last || last.id === seen.current || last.startedAt < mountedAt - 1) return;
    seen.current = last.id;
    setResult(last);
    if (failedCount(last) === 0) {
      const t = setTimeout(() => setResult((r) => (r?.id === last.id ? null : r)), HIDE_MS);
      return () => clearTimeout(t);
    }
  }, [last, mountedAt]);

  const show = running ?? result;
  const failed = !running && result ? failedCount(result) > 0 : false;
  const fraction = running?.state.state === "running" && running.state.total > 0 ? running.state.done / running.state.total : running ? 0 : 1;

  return createPortal(
    <div className="pointer-events-none fixed bottom-5 right-5 z-40" aria-live="polite">
      <AnimatePresence>
        {show && (
          <motion.div
            key="toast"
            role="status"
            initial={{ opacity: 0, y: 12, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 8 }}
            transition={{ type: "spring", stiffness: 380, damping: 32 }}
            className="glass-strong pointer-events-auto w-[300px] rounded-[var(--radius-panel)] px-3 pb-3 pt-3"
          >
            <div className="flex items-start justify-between gap-3">
              <p className="num text-[12px] font-medium text-[var(--text-primary)]">{running ? opRunning(running) : opResult(result!)}</p>
              {!running && (
                <button
                  type="button"
                  aria-label="Fechar aviso"
                  onClick={() => setResult(null)}
                  className="-mr-1 grid h-5 w-5 shrink-0 place-items-center rounded-[var(--radius-chip)] text-[var(--text-tertiary)] hover:bg-[var(--hover)] hover:text-[var(--text-primary)]"
                >
                  <svg width="8" height="8" viewBox="0 0 8 8" aria-hidden="true">
                    <path d="m1 1 6 6M7 1 1 7" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
                  </svg>
                </button>
              )}
            </div>
            <div className="mt-2.5 h-1 overflow-hidden rounded-full bg-[var(--hover)]">
              <motion.div
                className={`h-full rounded-full ${failed ? "bg-[var(--danger)]" : "bg-[var(--accent)]"}`}
                animate={{ width: `${Math.max(4, fraction * 100)}%` }}
                transition={{ type: "spring", stiffness: 160, damping: 26 }}
              />
            </div>
            {failed && (
              <button
                type="button"
                onClick={() => {
                  onDetails(result!.id);
                  setResult(null);
                }}
                className="mt-2.5 text-[12px] font-medium text-[var(--accent)] hover:underline"
              >
                Ver detalhes
              </button>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </div>,
    document.body,
  );
}
