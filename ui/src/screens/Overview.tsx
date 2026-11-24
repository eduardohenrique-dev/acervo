import { DiskCard } from "../components/DiskCard";
import type { DriveEntry } from "../hooks/driveState";
import type { CategoryKey } from "../lib/categories";
import { formatBytes } from "../lib/format";

interface Props {
  drives: DriveEntry[];
  elevated: boolean;
  onOpenSystem: (drive: string) => void;
  onOpenCategory: (drive: string, key: CategoryKey) => void;
  onRescan: (drive: string) => void;
}

export function Overview({ drives, elevated, onOpenSystem, onOpenCategory, onRescan }: Props) {
  const used = drives.reduce((a, d) => a + (d.view.info.total - d.view.info.free), 0);
  const total = drives.reduce((a, d) => a + d.view.info.total, 0);

  return (
    <div className="mx-auto flex max-w-[960px] flex-col gap-4">
      <header className="pb-1 pt-2">
        <h1 className="text-[length:var(--text-large-title)] font-bold leading-tight tracking-[var(--tracking-title)]">
          Armazenamento
        </h1>
        {drives.length > 0 && (
          <p className="num mt-1 text-[length:var(--text-body)] text-[var(--text-secondary)]">
            {drives.length === 1 ? "1 disco" : `${drives.length} discos`} · {formatBytes(used)} usados de {formatBytes(total)}
          </p>
        )}
      </header>

      {!elevated && (
        <p className="glass-strong rounded-[var(--radius-panel)] px-3 py-2 text-[length:var(--text-body)] text-[var(--text-secondary)]">
          O Acervo está rodando sem permissão de administrador: a análise percorre pasta por pasta, é mais lenta e
          não enxerga as áreas protegidas do Windows.
        </p>
      )}

      {drives.length === 0 && (
        <p className="text-[length:var(--text-body)] text-[var(--text-secondary)]">Procurando os discos deste computador…</p>
      )}

      {drives.map((d) => (
        <DiskCard
          key={d.view.info.letter}
          entry={d}
          onOpenSystem={() => onOpenSystem(d.view.info.letter)}
          onOpenCategory={(key) => onOpenCategory(d.view.info.letter, key)}
          onRescan={() => onRescan(d.view.info.letter)}
        />
      ))}
    </div>
  );
}
