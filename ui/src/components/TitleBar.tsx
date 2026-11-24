import { getCurrentWindow } from "@tauri-apps/api/window";
import { isTauri } from "../lib/env";
import { IconClose, IconMaximize, IconMinimize } from "./icons";

function act(fn: (w: ReturnType<typeof getCurrentWindow>) => Promise<void>) {
  if (isTauri()) void fn(getCurrentWindow());
}

const btn =
  "grid h-7 w-10 place-items-center rounded-[var(--radius-chip)] text-[var(--text-secondary)] transition-colors duration-150 hover:bg-[var(--hover)] hover:text-[var(--text-primary)]";

export function TitleBar() {
  return (
    <header data-tauri-drag-region className="flex items-center justify-between px-2 pl-4">
      <span
        data-tauri-drag-region
        className="flex items-center gap-2 text-[12px] font-semibold tracking-[-0.01em] text-[var(--text-secondary)]"
      >
        <img src="/logo.svg" alt="" width={18} height={18} draggable={false} className="pointer-events-none" />
        Acervo
      </span>
      <div className="flex items-center gap-0.5">
        <button type="button" aria-label="Minimizar" className={btn} onClick={() => act((w) => w.minimize())}>
          <IconMinimize />
        </button>
        <button
          type="button"
          aria-label="Maximizar"
          className={btn}
          onClick={() => act((w) => w.toggleMaximize())}
        >
          <IconMaximize />
        </button>
        <button
          type="button"
          aria-label="Fechar"
          className={`${btn} hover:!bg-[#e81123] hover:!text-white`}
          onClick={() => act((w) => w.close())}
        >
          <IconClose />
        </button>
      </div>
    </header>
  );
}
