export type CategoryKey =
  | "apps"
  | "jogos"
  | "dev"
  | "midia"
  | "documentos"
  | "downloads"
  | "cache"
  | "dados_apps"
  | "outros"
  | "sistema";

export interface CategoryDef {
  key: CategoryKey;
  label: string;
  /** Índice em `DriveSummary.byCategory` (ordem do Rust). */
  index: number;
  colorVar: string;
}

const def = (key: CategoryKey, label: string, index: number): CategoryDef => ({
  key,
  label,
  index,
  colorVar: `--cat-${key}`,
});

/** Ordem de exibição: o Sistema por último, em cinza, como no iPhone. */
export const CATEGORIES: CategoryDef[] = [
  def("apps", "Apps", 0),
  def("jogos", "Jogos", 1),
  def("dev", "Desenvolvimento", 2),
  def("midia", "Mídia", 3),
  def("documentos", "Documentos", 4),
  def("downloads", "Downloads", 5),
  def("cache", "Cache e temporários", 6),
  def("dados_apps", "Dados de apps", 7),
  def("outros", "Outros", 9),
  def("sistema", "Sistema", 8),
];
