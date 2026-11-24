//! Categorias do Acervo e regras de classificação de caminhos.

mod category;
mod ext;
mod known;
mod launchers;
mod rules;
mod standard;

pub use category::{Category, CATEGORY_COUNT};
pub use ext::category_for_extension;
pub use known::{discover, KnownPaths, UserProfile};
pub use launchers::{parse_epic_manifest, parse_riot_installs, parse_steam_libraryfolders};
pub use rules::{Classifier, DirCtx, PathRule, Strength};
