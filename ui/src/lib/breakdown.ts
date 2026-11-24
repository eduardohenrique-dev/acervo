/** Parte de `bytes` em `total`, em % (0–100), segura contra total zero e valores fora da faixa. */
export function sharePercent(bytes: number, total: number): number {
  if (!(total > 0) || !(bytes > 0)) return 0;
  return Math.min(100, (bytes / total) * 100);
}
