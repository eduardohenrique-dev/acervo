import { motion } from "motion/react";
import { useEffect, useState } from "react";
import type { DriveEntry } from "../hooks/driveState";
import type { BreakdownItem } from "../lib/api";
import { sharePercent } from "../lib/breakdown";
import { driveName } from "../lib/drive";
import { formatBytes } from "../lib/format";
import { source } from "../lib/source";

const SISTEMA = 8;

const HINTS: Record<string, string> = {
  windows: "Os arquivos do próprio Windows. Cresce com atualizações.",
  memoria_virtual: "Usada quando a memória RAM acaba (pagefile.sys).",
  hibernacao: "Guarda a memória quando o PC hiberna (hiberfil.sys).",
  restauracao: "Cópias para voltar o Windows a um estado anterior.",
  recuperacao: "Ferramentas para consertar o Windows se ele não iniciar.",
  ntfs: "O índice e o diário do próprio disco.",
  outros: "Outros arquivos protegidos do sistema.",
};

interface Props {
  drives: DriveEntry[];
  drive: string | null;
  onDrive: (drive: string) => void;
}

export function System({ drives, drive, onDrive }: Props) {
  const ready = drives.filter((d) => d.view.status.state === "ready" && d.summary);
  const current = ready.find((d) => d.view.info.letter === drive) ?? ready[0];
  const letter = current?.view.info.letter;
  const [items, setItems] = useState<BreakdownItem[] | null>(null);

  // Trocar de disco mostra "Carregando…"; uma mudança ao vivo só troca os números.
  useEffect(() => {
    setItems(null);
  }, [letter]);

  const summary = current?.summary;
  useEffect(() => {
    if (!letter) return;
    let alive = true;
    void source.systemBreakdown(letter).then((b) => alive && setItems(b ?? []));
    return () => {
      alive = false;
    };
  }, [letter, summary]);

  const total = current?.summary?.byCategory[SISTEMA] ?? 0;

  return (
    <div className="mx-auto flex max-w-[960px] flex-col gap-4">
      <header className="flex items-end justify-between gap-5 pb-1 pt-2">
        <div>
          <h1 className="text-[length:var(--text-large-title)] font-bold leading-tight tracking-[var(--tracking-title)]">
            Sistema
          </h1>
          <p className="mt-1 max-w-[56ch] text-[length:var(--text-body)] text-[var(--text-secondary)]">
            Espaço que o Windows usa para funcionar. Ele aparece aqui só para você saber o tamanho: o Acervo não mexe
            nesses arquivos.
          </p>
        </div>
        {ready.length > 1 && (
          <div role="tablist" aria-label="Disco" className="glass-strong flex shrink-0 rounded-[9px] p-0.5">
            {ready.map((d) => {
              const l = d.view.info.letter;
              const on = l === letter;
              return (
                <button
                  key={l}
                  role="tab"
                  type="button"
                  aria-selected={on}
                  onClick={() => onDrive(l)}
                  className={`relative h-6 min-w-14 rounded-[var(--radius-chip)] px-3 text-[12px] font-semibold transition-colors ${
                    on ? "text-[var(--text-primary)]" : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
                  }`}
                >
                  {on && (
                    <motion.span
                      layoutId="drive-tab"
                      className="absolute inset-0 rounded-[var(--radius-chip)] bg-[var(--glass-fill)] shadow-[0_1px_3px_rgba(0,0,0,0.12),inset_0_1px_0_var(--glass-shine)]"
                      transition={{ type: "spring", stiffness: 380, damping: 32 }}
                    />
                  )}
                  <span className="relative">{l}:</span>
                </button>
              );
            })}
          </div>
        )}
      </header>

      {!current && (
        <p className="text-[length:var(--text-body)] text-[var(--text-secondary)]">
          O detalhamento aparece assim que a análise de um disco terminar.
        </p>
      )}

      {current && (
        <section aria-label={`Sistema em ${driveName(current.view.info)}`} className="glass rounded-[var(--radius-panel)] px-5 py-4">
          <header className="mb-2 flex items-baseline justify-between">
            <h2 className="text-[length:var(--text-title)] font-semibold tracking-[-0.012em]">{driveName(current.view.info)}</h2>
            <p className="num text-[length:var(--text-body)] text-[var(--text-secondary)]">
              <span className="font-semibold text-[var(--text-primary)]">{formatBytes(total)}</span> do sistema
            </p>
          </header>

          {items === null && <p className="py-4 text-[length:var(--text-body)] text-[var(--text-secondary)]">Carregando…</p>}
          {items && items.length === 0 && (
            <p className="py-4 text-[length:var(--text-body)] text-[var(--text-secondary)]">Nada do sistema neste disco.</p>
          )}

          <ul>
            {(items ?? []).map((item, i) => (
              <li
                key={item.key}
                className="grid grid-cols-[minmax(0,1fr)_minmax(120px,240px)_88px] items-center gap-4 border-t border-[var(--separator)] py-2 first:border-t-0"
              >
                <div className="min-w-0">
                  <p className="text-[length:var(--text-body)] font-medium text-[var(--text-primary)]">{item.label}</p>
                  <p className="truncate text-[length:var(--text-caption)] text-[var(--text-tertiary)]">{HINTS[item.key] ?? ""}</p>
                </div>
                <div className="capsule !h-2">
                  <motion.div
                    className="capsule-segment rounded-[inherit]"
                    style={{ ["--seg-color" as string]: "var(--cat-sistema)" }}
                    initial={{ width: 0 }}
                    animate={{ width: `${sharePercent(item.bytes, total)}%` }}
                    transition={{ type: "spring", stiffness: 170, damping: 26, delay: i * 0.03 }}
                  />
                </div>
                <p className="num text-right text-[length:var(--text-body)] text-[var(--text-primary)]">{formatBytes(item.bytes)}</p>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
