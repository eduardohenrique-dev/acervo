import { useState } from "react";
import { IconActivity } from "../components/icons";
import type { OpRecord } from "../lib/api";
import { formatBytes } from "../lib/format";
import { failedCount, OP_LABEL, opResult, opRunning } from "../lib/opText";

interface Props {
  running: OpRecord | null;
  history: OpRecord[];
  /** Operação a mostrar aberta (vinda do "Ver detalhes"). */
  focus: number | null;
}

const TIME = new Intl.DateTimeFormat("pt-BR", { hour: "2-digit", minute: "2-digit" });
const items = (n: number) => (n === 1 ? "1 item" : `${n} itens`);

/** O que o Acervo fez nesta sessão: tipo, itens, tamanho, quando e o resultado. */
export function Operations({ running, history, focus }: Props) {
  const [open, setOpen] = useState<Set<number>>(() => new Set(focus === null ? [] : [focus]));
  const toggle = (id: number) =>
    setOpen((s) => {
      const n = new Set(s);
      if (n.has(id)) n.delete(id);
      else n.add(id);
      return n;
    });

  return (
    <div className="mx-auto flex max-w-[960px] flex-col gap-3 pb-4">
      <header className="pt-2">
        <h1 className="text-[length:var(--text-large-title)] font-bold leading-tight tracking-[var(--tracking-title)]">Operações</h1>
        <p className="mt-1 text-[length:var(--text-body)] text-[var(--text-secondary)]">
          O que foi feito desde que o Acervo abriu. O registro completo fica em ProgramData\Acervo.
        </p>
      </header>

      {!running && history.length === 0 && (
        <div className="glass flex flex-col items-center gap-2 rounded-[var(--radius-panel)] px-5 py-8 text-center">
          <IconActivity width={22} height={22} className="text-[var(--text-tertiary)]" />
          <p className="text-[length:var(--text-body)] text-[var(--text-primary)]">Nenhuma operação ainda</p>
          <p className="max-w-[360px] text-[12px] text-[var(--text-secondary)]">
            Quando você mandar algo para a Lixeira, mover, copiar ou renomear, aparece aqui, com o que deu certo e o que não deu.
          </p>
        </div>
      )}

      {(running || history.length > 0) && (
        <ul aria-label="Operações da sessão" className="glass flex flex-col rounded-[var(--radius-panel)] p-2">
          {running && (
            <li className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-3 rounded-[var(--radius-control)] px-3 py-1.5">
              <div className="min-w-0">
                <p className="num text-[length:var(--text-body)] font-medium text-[var(--text-primary)]">{opRunning(running)}</p>
                <p className="num text-[12px] text-[var(--text-secondary)]">
                  {items(running.count)} · {formatBytes(running.bytes)}
                </p>
              </div>
              <span className="flex items-center gap-1.5 text-[12px] text-[var(--text-secondary)]">
                <span className="live-dot" aria-hidden="true" />
                Em andamento
              </span>
            </li>
          )}
          {history.map((r) => {
            const failed = failedCount(r);
            const expanded = open.has(r.id);
            return (
              <li key={r.id} className="rounded-[var(--radius-control)]">
                <button
                  type="button"
                  aria-expanded={failed ? expanded : undefined}
                  disabled={!failed}
                  onClick={() => toggle(r.id)}
                  className="grid w-full grid-cols-[120px_minmax(0,1fr)_auto_56px] items-center gap-3 rounded-[var(--radius-control)] px-3 py-1.5 text-left enabled:hover:bg-[var(--hover)]"
                >
                  <span className="text-[length:var(--text-body)] font-medium text-[var(--text-primary)]">{OP_LABEL[r.kind]}</span>
                  <span className="num truncate text-[12px] text-[var(--text-secondary)]">
                    {items(r.count)} · {formatBytes(r.bytes)}
                  </span>
                  <span className={`num text-[12px] ${failed ? "font-medium text-[var(--danger)]" : "text-[var(--text-secondary)]"}`}>
                    {failed ? opResult(r) : "Concluída"}
                  </span>
                  <span className="num text-right text-[12px] text-[var(--text-tertiary)]">{TIME.format(new Date(r.startedAt * 1000))}</span>
                </button>
                {failed > 0 && expanded && r.outcome && (
                  <ul aria-label="Itens que falharam" className="mx-3 mb-2.5 flex flex-col gap-1.5 rounded-[var(--radius-control)] bg-[color-mix(in_srgb,var(--danger)_7%,transparent)] px-3 py-1.5">
                    {r.outcome.failed.map((f) => (
                      <li key={f.path} className="min-w-0">
                        <p className="truncate text-[12px] text-[var(--text-primary)]" title={f.path}>
                          {f.path}
                        </p>
                        <p className="text-[11px] text-[var(--text-secondary)]">{f.message}</p>
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
