import { useCallback, useState } from "react";
import type { Action } from "../components/ActionBar";
import { ConfirmDialog, type ConfirmMode } from "../components/ConfirmDialog";
import type { ItemRef, ItemRow, OpKindTag, OpRequest } from "../lib/api";
import { source } from "../lib/source";

interface Pending {
  mode: ConfirmMode;
  refs: ItemRef[];
  bytes: number;
  warnings: string[];
  dest?: string;
  currentName?: string;
  /** Nomes atuais dos itens, para a pessoa conferir o que vai ser feito. */
  names: string[];
}

/**
 * Fluxo de uma ação sobre os itens selecionados: confere no backend (bloqueios,
 * avisos, item que mudou), pede confirmação quando é destrutiva e põe na fila.
 */
export function useItemActions(drive: string, selectedRows: ItemRow[], onStale: () => void, onStarted: () => void) {
  const [pending, setPending] = useState<Pending | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const start = useCallback(
    async (refs: ItemRef[], req: OpRequest) => {
      try {
        await source.startOperation(refs, req);
        onStarted();
      } catch (e) {
        setNotice(String(e));
        onStale();
      }
    },
    [onStarted, onStale],
  );

  const request = useCallback(
    async (action: Action) => {
      setNotice(null);
      const rows = selectedRows;
      if (rows.length === 0) return;
      const refs: ItemRef[] = rows.map((r) => ({ drive, node: r.node, frn: r.frn, path: r.path }));
      if (action === "reveal") {
        source.revealItem(refs[0]).catch((e) => setNotice(String(e)));
        return;
      }
      const tag: OpKindTag = action;
      const checks = await source.checkItems(refs, tag);
      if (checks.some((c) => c.path === null)) {
        setNotice("A lista mudou desde que foi mostrada. Atualizei para você; selecione de novo.");
        onStale();
        return;
      }
      const blocked = checks.filter((c) => c.verdict.verdict === "blocked");
      const allowed = checks.filter((c) => c.verdict.verdict !== "blocked");
      if (blocked.length) {
        const first = blocked[0];
        const reason = first.verdict.verdict === "blocked" ? first.verdict.message : "";
        const name = first.path?.split("\\").pop() ?? "";
        setNotice(
          blocked.length === 1
            ? `"${name}" ficou de fora: ${reason}`
            : `${blocked.length} itens ficaram de fora (ex.: "${name}": ${reason})`,
        );
      }
      if (!allowed.length) return;
      const okRefs = allowed.map((c) => c.item);
      const okNodes = new Set(okRefs.map((x) => x.node));
      const bytes = rows.filter((r) => okNodes.has(r.node)).reduce((a, r) => a + r.alloc, 0);
      // Os nomes que vão aparecer no diálogo vêm do backend (o caminho atual), não da tela.
      const names = allowed.map((c) => c.path!.split("\\").pop() ?? c.path!);
      const warnings = [...new Set(allowed.flatMap((c) => (c.verdict.verdict === "warn" ? [c.verdict.message] : [])))];

      if (action === "copy" || action === "move") {
        const dest = await source.pickFolder();
        if (!dest) return;
        if (action === "copy") return void start(okRefs, { kind: "copy", dest });
        return setPending({ mode: "move", refs: okRefs, bytes, warnings, dest, names });
      }
      if (action === "rename") {
        return setPending({ mode: "rename", refs: okRefs, bytes, warnings: [], currentName: rows[0].name, names });
      }
      setPending({ mode: action === "recycle" ? "recycle" : "permanent", refs: okRefs, bytes, warnings, names });
    },
    [drive, selectedRows, onStale, start],
  );

  const dialog = pending ? (
    <ConfirmDialog
      mode={pending.mode}
      count={pending.refs.length}
      bytes={pending.bytes}
      warnings={pending.warnings}
      dest={pending.dest}
      currentName={pending.currentName}
      names={pending.names}
      onCancel={() => setPending(null)}
      onConfirm={(value) => {
        const p = pending;
        setPending(null);
        const req: OpRequest =
          p.mode === "recycle"
            ? { kind: "recycle" }
            : p.mode === "permanent"
              ? { kind: "deletePermanent" }
              : p.mode === "move"
                ? { kind: "move", dest: p.dest! }
                : { kind: "rename", newName: value! };
        void start(p.refs, req);
      }}
    />
  ) : null;

  return { request, dialog, notice, dismissNotice: () => setNotice(null) };
}
