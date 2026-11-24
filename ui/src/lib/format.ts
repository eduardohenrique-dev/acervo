const UNITS = ["B", "KB", "MB", "GB", "TB"];

/** Tamanho em base 1024, como o Explorer em pt-BR: "53,1 GB", "931 GB". */
export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n < 1024) return `${Math.max(0, Math.round(n || 0))} B`;
  let i = 0;
  let v = n;
  while (v >= 1024 && i < UNITS.length - 1) {
    v /= 1024;
    i++;
  }
  const rounded = v >= 100 ? Math.round(v) : Math.round(v * 10) / 10;
  return `${String(rounded).replace(".", ",")} ${UNITS[i]}`;
}

/** Fração (0–1) como porcentagem inteira; valores pequenos viram "< 1%". */
export function formatPercent(fraction: number): string {
  if (!fraction || fraction <= 0) return "0%";
  const pct = fraction * 100;
  if (pct < 1) return "< 1%";
  return `${Math.round(pct)}%`;
}

/** Diferença entre 1601 (FILETIME) e 1970 (Unix), em ms. */
const EPOCH_DIFF_MS = 11644473600000;

/** Milissegundos Unix → FILETIME (intervalos de 100 ns desde 1601), como o NTFS guarda. */
export const toFiletime = (ms: number) => (ms + EPOCH_DIFF_MS) * 10000;

const DATE = new Intl.DateTimeFormat("pt-BR", { day: "2-digit", month: "2-digit", year: "numeric" });

/** FILETIME como "06/11/2026"; zero (sem data) vira "—". */
export function formatFiletime(ft: number): string {
  if (!ft) return "—";
  return DATE.format(new Date(ft / 10000 - EPOCH_DIFF_MS));
}
