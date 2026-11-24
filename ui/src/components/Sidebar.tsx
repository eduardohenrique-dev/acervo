import type { ReactNode } from "react";
import { IconActivity, IconFiles, IconOverview, IconSystem } from "./icons";

export type Screen = "overview" | "files" | "operations" | "system";

const ITEMS: { id: Screen; label: string; icon: ReactNode }[] = [
  { id: "overview", label: "Visão geral", icon: <IconOverview /> },
  { id: "files", label: "Arquivos", icon: <IconFiles /> },
  { id: "operations", label: "Operações", icon: <IconActivity /> },
  { id: "system", label: "Sistema", icon: <IconSystem /> },
];

interface Props {
  current: Screen;
  onChange: (s: Screen) => void;
  /** Uma operação está rodando: o item Operações ganha o ponto aceso. */
  busy: boolean;
}

export function Sidebar({ current, onChange, busy }: Props) {
  return (
    <nav aria-label="Seções" className="glass-strong m-2 mt-0 flex w-[188px] flex-col gap-px rounded-[var(--radius-panel)] p-1.5">
      {ITEMS.map((item) => {
        const active = item.id === current;
        const dot = item.id === "operations" && busy;
        return (
          <button
            key={item.id}
            type="button"
            aria-current={active ? "page" : undefined}
            onClick={() => onChange(item.id)}
            className={`flex h-8 items-center gap-2 rounded-[var(--radius-control)] px-2.5 text-left text-[length:var(--text-body)] transition-colors duration-150 ${
              active
                ? "bg-[var(--selected)] font-semibold text-[var(--text-primary)]"
                : "text-[var(--text-secondary)] hover:bg-[var(--hover)] hover:text-[var(--text-primary)]"
            }`}
          >
            <span className={active ? "text-[var(--accent)]" : ""}>{item.icon}</span>
            <span className="flex-1">{item.label}</span>
            {dot && (
              <span className="live-dot" role="img" aria-label="uma operação em andamento" />
            )}
          </button>
        );
      })}
    </nav>
  );
}
