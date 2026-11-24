import { motion } from "motion/react";
import { useState } from "react";
import type { DriveEntry } from "../hooks/driveState";
import { CATEGORIES, type CategoryKey } from "../lib/categories";
import { driveName } from "../lib/drive";
import { formatBytes, formatPercent } from "../lib/format";
import { CategoryLegend } from "./CategoryLegend";
import { IconRetry } from "./icons";
import { ActiveCategory, StorageBar } from "./StorageBar";

interface Props {
  entry: DriveEntry;
  onOpenSystem: () => void;
  onOpenCategory: (key: CategoryKey) => void;
  onRescan: () => void;
}

export function DiskCard({ entry, onOpenSystem, onOpenCategory, onRescan }: Props) {
  const [active, setActive] = useState<CategoryKey | null>(null);
  const open = (key: CategoryKey) => (key === "sistema" ? onOpenSystem() : onOpenCategory(key));
  const { info, status } = entry.view;
  const s = entry.summary;
  const used = s?.usedBytes ?? info.total - info.free;
  const total = s?.totalBytes ?? info.total;
  const unknown = s ? s.usedBytes - s.catalogedBytes : 0;

  return (
    <motion.section
      layout
      aria-label={driveName(info)}
      className="glass rounded-[var(--radius-panel)] px-5 pb-4 pt-4"
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, ease: [0.16, 1, 0.3, 1] }}
    >
      <header className="mb-3 flex items-baseline justify-between gap-3">
        <div className="flex items-center gap-2.5">
          <h2 className="text-[length:var(--text-title)] font-semibold tracking-[-0.012em]">{driveName(info)}</h2>
          {status.state === "ready" && status.mode === "walk" && <WalkBadge />}
          {status.state === "ready" && status.live && <LiveBadge />}
        </div>
        {active && s ? (
          <ActiveCategory categoryKey={active} bytes={s.byCategory[CATEGORIES.find((c) => c.key === active)!.index]} total={s.totalBytes} />
        ) : (
          <p className="num text-[length:var(--text-body)] text-[var(--text-secondary)]">
            <span className="font-semibold text-[var(--text-primary)]">{formatBytes(used)}</span> de {formatBytes(total)} usados
          </p>
        )}
      </header>

      {status.state === "scanning" && (
        <>
          <div className="capsule" role="progressbar" aria-valuenow={Math.round(status.fraction * 100)} aria-valuemin={0} aria-valuemax={100} aria-label="Progresso da análise">
            <motion.div
              className="capsule-liquid bg-[color-mix(in_srgb,var(--accent)_70%,transparent)] bg-[linear-gradient(180deg,rgba(255,255,255,0.3),rgba(255,255,255,0)_50%,rgba(0,0,0,0.1))]"
              animate={{ width: `${Math.max(4, status.fraction * 100)}%` }}
              transition={{ type: "spring", stiffness: 120, damping: 24 }}
            />
            <div className="capsule-scan" />
            <div className="capsule-shine" />
          </div>
          <p className="num mt-2.5 text-[length:var(--text-caption)] text-[var(--text-secondary)]">
            {status.mode === "mft" ? "Lendo o índice do disco" : "Percorrendo as pastas"} · {formatPercent(status.fraction)}
          </p>
        </>
      )}

      {status.state === "ready" && s && (
        <>
          <StorageBar
            byCategory={s.byCategory}
            total={s.totalBytes}
            active={active}
            onActive={setActive}
            onSelect={open}
          />
          <div className="mt-3 flex items-end justify-between gap-5">
            <CategoryLegend byCategory={s.byCategory} active={active} onActive={setActive} onOpen={open} />
            <span className="num flex h-6 shrink-0 items-center text-[length:var(--text-caption)] text-[var(--text-tertiary)]">
              {formatBytes(s.freeBytes)} livres
            </span>
          </div>
          {unknown > s.usedBytes * 0.01 && (
            <p className="num mt-3 text-[length:var(--text-caption)] text-[var(--text-tertiary)]">
              {formatBytes(unknown)} em uso não foram identificados nesta análise.
            </p>
          )}
        </>
      )}

      {status.state === "failed" && (
        <div className="flex items-center justify-between gap-5 rounded-[var(--radius-panel)] bg-[color-mix(in_srgb,var(--danger)_8%,transparent)] px-3 py-2">
          <p className="text-[length:var(--text-body)] text-[var(--text-primary)]">{status.message}</p>
          <button
            type="button"
            onClick={onRescan}
            className="flex h-7 shrink-0 items-center gap-1.5 rounded-[var(--radius-control)] bg-[var(--accent)] px-3 text-[12px] font-semibold text-white shadow-[0_1px_2px_rgba(0,0,0,0.2),inset_0_1px_0_rgba(255,255,255,0.25)] transition-[filter] hover:brightness-110 active:brightness-95"
          >
            <IconRetry width={14} height={14} />
            Tentar de novo
          </button>
        </div>
      )}
    </motion.section>
  );
}

/** O disco está sendo acompanhado: muda sozinho quando algo muda. */
function LiveBadge() {
  return (
    <span
      title="Atualiza sozinho quando algo muda no disco"
      className="flex items-center gap-1.5 text-[length:var(--text-caption)] text-[var(--text-secondary)]"
    >
      <span className="live-dot" aria-hidden="true" />
      Ao vivo
    </span>
  );
}

/** Só a exceção ganha selo: sem administrador, os números são aproximados. */
function WalkBadge() {
  return (
    <span
      title="Sem administrador: o Acervo percorreu pasta por pasta e não entrou nas áreas protegidas"
      className="rounded-[var(--radius-chip)] bg-[var(--hover)] px-2 py-0.5 text-[length:var(--text-caption)] font-medium text-[var(--text-secondary)]"
    >
      Números aproximados
    </span>
  );
}
