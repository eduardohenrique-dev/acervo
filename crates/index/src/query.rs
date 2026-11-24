//! Consultas sobre a árvore: resumo do disco e detalhamento do Sistema.

use acervo_classify::{Category, CATEGORY_COUNT};
use serde::Serialize;

use crate::{ScanMode, Tree};

/// Espaço total e livre do disco, como o Windows informa.
#[derive(Clone, Copy, Debug)]
pub struct DiskSpace {
    pub total: u64,
    pub free: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveSummary {
    pub drive: char,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    /// Bytes por categoria, na ordem do índice `u8` de `Category`.
    pub by_category: [u64; CATEGORY_COUNT],
    pub cataloged_bytes: u64,
    pub mode: ScanMode,
}

pub fn summary(tree: &Tree, space: DiskSpace, mode: ScanMode) -> DriveSummary {
    let by_category = *tree.totals(tree.root);
    DriveSummary {
        drive: tree.drive,
        total_bytes: space.total,
        free_bytes: space.free,
        used_bytes: space.total.saturating_sub(space.free),
        cataloged_bytes: by_category.iter().sum(),
        by_category,
        mode,
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownItem {
    pub key: &'static str,
    pub label: &'static str,
    pub bytes: u64,
}

const ITEMS: [(&str, &str); 7] = [
    ("windows", "Windows"),
    ("memoria_virtual", "Memória virtual"),
    ("hibernacao", "Hibernação"),
    ("restauracao", "Pontos de restauração"),
    ("recuperacao", "Recuperação"),
    ("ntfs", "Estruturas do NTFS"),
    ("outros", "Outros do sistema"),
];

fn bucket(name: &str) -> Option<usize> {
    let n = name.to_lowercase();
    match n.as_str() {
        "windows" => Some(0),
        "pagefile.sys" | "swapfile.sys" => Some(1),
        "hiberfil.sys" => Some(2),
        "system volume information" => Some(3),
        "recovery" => Some(4),
        _ if n.starts_with('$') => Some(5),
        _ => None,
    }
}

/// Quanto o Sistema ocupa, por parte. Itens com 0 bytes são omitidos;
/// ordenado por tamanho, do maior para o menor.
pub fn system_breakdown(tree: &Tree) -> Vec<BreakdownItem> {
    let sys = Category::Sistema as usize;
    let mut bytes = [0u64; ITEMS.len()];
    for child in tree.children(tree.root) {
        let node = tree.node(child);
        let own = if node.is_dir() {
            tree.totals(child)[sys]
        } else if tree.category(child) == Category::Sistema {
            node.alloc
        } else {
            0
        };
        if let Some(b) = bucket(tree.name(child)) {
            bytes[b] += own;
        }
    }
    let known: u64 = bytes.iter().sum();
    bytes[6] = tree.totals(tree.root)[sys].saturating_sub(known);

    let mut out: Vec<BreakdownItem> = ITEMS
        .iter()
        .zip(bytes)
        .filter(|(_, b)| *b > 0)
        .map(|(&(key, label), bytes)| BreakdownItem { key, label, bytes })
        .collect();
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    out
}
