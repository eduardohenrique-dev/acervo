//! O que pode ser feito com cada item (spec 8: bloqueios e avisos).

use std::path::{Component, Path, PathBuf};

use acervo_classify::Category;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OpKindTag {
    Recycle,
    DeletePermanent,
    Copy,
    Move,
    Rename,
    Reveal,
}

impl OpKindTag {
    /// Nome curto em português, usado no log.
    pub fn verb(self) -> &'static str {
        match self {
            OpKindTag::Recycle => "lixeira",
            OpKindTag::DeletePermanent => "apagar",
            OpKindTag::Copy => "copiar",
            OpKindTag::Move => "mover",
            OpKindTag::Rename => "renomear",
            OpKindTag::Reveal => "mostrar",
        }
    }
}

/// O que se sabe de um item para decidir.
#[derive(Clone, Debug)]
pub struct ItemFacts {
    pub path: PathBuf,
    pub category: Category,
    pub is_dir: bool,
    pub is_reparse: bool,
    pub is_system: bool,
}

/// Pastas que o Acervo nunca deixa mexer (instalação e dados dele).
#[derive(Clone, Debug, Default)]
pub struct Protected {
    pub app_dirs: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "verdict", rename_all = "camelCase")]
pub enum Verdict {
    Allowed,
    Warn { message: String },
    Blocked { message: String },
}

fn parts(p: &Path) -> Vec<String> {
    p.components()
        .filter(|c| !matches!(c, Component::RootDir | Component::CurDir))
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect()
}

fn is_within(path: &Path, dir: &Path) -> bool {
    let (p, d) = (parts(path), parts(dir));
    !d.is_empty() && p.len() >= d.len() && p[..d.len()] == d[..]
}

/// O destino de copiar ou mover. `category` é a da pasta de destino, se o Acervo a conhece.
/// Como o app roda como administrador, o Windows não avisaria nada: a recusa é daqui.
pub fn check_dest(dest: &Path, category: Option<Category>, protected: &Protected) -> Verdict {
    if category == Some(Category::Sistema) {
        return Verdict::Blocked { message: "Não dá para pôr itens numa pasta do Windows.".into() };
    }
    if protected.app_dirs.iter().any(|d| is_within(dest, d)) {
        return Verdict::Blocked { message: "Esta é uma pasta do próprio Acervo.".into() };
    }
    Verdict::Allowed
}

pub fn check_item(item: &ItemFacts, op: OpKindTag, protected: &Protected) -> Verdict {
    if op == OpKindTag::Reveal {
        return Verdict::Allowed;
    }
    let blocked = |m: &str| Verdict::Blocked { message: m.into() };
    if item.category == Category::Sistema {
        return blocked("O Windows precisa disso para funcionar.");
    }
    if parts(&item.path).len() <= 1 {
        return blocked("Não dá para mexer na raiz do disco.");
    }
    if protected.app_dirs.iter().any(|d| is_within(&item.path, d)) {
        return blocked("Esta é uma pasta do próprio Acervo.");
    }
    if item.is_dir && protected.app_dirs.iter().any(|d| is_within(d, &item.path)) {
        return blocked("Esta pasta contém os dados do próprio Acervo.");
    }
    if item.is_dir && item.is_reparse && item.is_system {
        return blocked("Isto é um atalho de pasta do Windows.");
    }
    let destructive = matches!(op, OpKindTag::Recycle | OpKindTag::DeletePermanent | OpKindTag::Move);
    if destructive && matches!(item.category, Category::Apps | Category::DadosApps) {
        return Verdict::Warn { message: "Isto pertence a um programa. Se ele ainda estiver instalado, o certo é desinstalar.".into() };
    }
    Verdict::Allowed
}
