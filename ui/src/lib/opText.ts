import type { OpKindTag, OpRecord } from "./api";
import { formatBytes } from "./format";

const items = (n: number) => (n === 1 ? "1 item" : `${n} itens`);

/** Nome curto do tipo de operação, para listas. */
export const OP_LABEL: Record<OpKindTag, string> = {
  recycle: "Lixeira",
  deletePermanent: "Apagar de vez",
  copy: "Copiar",
  move: "Mover",
  rename: "Renomear",
  reveal: "Explorer",
};

const RUNNING: Record<OpKindTag, (n: number) => string> = {
  recycle: (n) => `Mandando ${items(n)} para a Lixeira…`,
  deletePermanent: (n) => `Apagando ${items(n)}…`,
  copy: (n) => `Copiando ${items(n)}…`,
  move: (n) => `Movendo ${items(n)}…`,
  rename: () => "Renomeando…",
  reveal: () => "Abrindo no Explorer…",
};

/** Particípio no plural, para "não puderam ser …". */
const PAST: Record<OpKindTag, string> = {
  recycle: "mandados para a Lixeira",
  deletePermanent: "apagados",
  copy: "copiados",
  move: "movidos",
  rename: "renomeados",
  reveal: "abertos",
};

const PAST_ONE: Record<OpKindTag, string> = {
  recycle: "mandado para a Lixeira",
  deletePermanent: "apagado",
  copy: "copiado",
  move: "movido",
  rename: "renomeado",
  reveal: "aberto",
};

/** "Mandando 3 itens para a Lixeira… 40%". */
export function opRunning(r: OpRecord): string {
  const base = RUNNING[r.kind](r.count);
  if (r.state.state !== "running" || r.state.total === 0) return base;
  return `${base} ${Math.round((r.state.done / r.state.total) * 100)}%`;
}

/** Quantos dos itens pedidos falharam. */
export const failedCount = (r: OpRecord) => (r.outcome ? r.outcome.failed.length : r.state.state === "failed" ? r.count : 0);

/** "Liberou 1,2 GB" ou "2 de 3 itens não puderam ser apagados". */
export function opResult(r: OpRecord): string {
  const failed = failedCount(r);
  if (failed > 0) {
    const total = r.count;
    if (total === 1) return `O item não pôde ser ${PAST_ONE[r.kind]}`;
    // Concorda com o número que falhou: "1 de 3 itens não pôde ser apagado".
    return failed === 1
      ? `1 de ${items(total)} não pôde ser ${PAST_ONE[r.kind]}`
      : `${failed} de ${items(total)} não puderam ser ${PAST[r.kind]}`;
  }
  if (r.outcome?.aborted) return "Cancelada";
  switch (r.kind) {
    // A Lixeira fica no mesmo disco: o espaço só volta quando ela é esvaziada.
    case "recycle":
      return `${formatBytes(r.bytes)} foram para a Lixeira`;
    case "deletePermanent":
      return `Liberou ${formatBytes(r.bytes)}`;
    case "copy":
      return `Copiou ${items(r.count)} (${formatBytes(r.bytes)})`;
    case "move":
      return `Moveu ${items(r.count)} (${formatBytes(r.bytes)})`;
    case "rename":
      return "Renomeado";
    case "reveal":
      return "Aberto no Explorer";
  }
}
