//! Navegação: filhos de uma pasta numa categoria e a lista de todos os arquivos.
//! O Sistema nunca aparece aqui (só na tela Sistema, como tamanho).

use acervo_classify::Category;
use serde::{Deserialize, Serialize};

use crate::{flags, Tree, NONE};

const SYS: usize = Category::Sistema as usize;
/// Máximo de linhas por página.
pub const MAX_LIMIT: usize = 500;
const MAX_DEPTH: usize = 4096;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemRow {
    pub node: u32,
    pub frn: u64,
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub alloc: u64,
    /// Quanto ocupa na categoria pedida (ou no total, sem o Sistema).
    pub bytes: u64,
    pub modified: i64,
    pub category: Category,
    pub child_count: u32,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileFilter {
    pub category: Option<Category>,
    pub under: Option<u32>,
    pub extension: Option<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub search: Option<String>,
    /// FILETIME, como `Node::modified`.
    pub modified_before: Option<i64>,
    pub modified_after: Option<i64>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SortKey {
    SizeDesc,
    SizeAsc,
    NameAsc,
    NameDesc,
    ModifiedDesc,
    ModifiedAsc,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub rows: Vec<ItemRow>,
    pub total: usize,
}

/// Vetor de todos os arquivos listáveis, do maior para o menor, por versão da árvore.
#[derive(Default)]
pub struct QueryCache {
    version: u64,
    by_size: Vec<u32>,
}

fn alive(tree: &Tree, i: u32) -> bool {
    (i as usize) < tree.len() && tree.node(i).flags & flags::DEAD == 0
}

/// Bytes de um item na categoria (ou no total sem o Sistema).
fn bytes_of(tree: &Tree, i: u32, cat: Option<Category>) -> u64 {
    let n = tree.node(i);
    if n.is_dir() {
        let t = tree.totals(i);
        match cat {
            Some(c) => t[c as usize],
            None => t.iter().enumerate().filter(|(k, _)| *k != SYS).map(|(_, v)| *v).sum(),
        }
    } else {
        let c = tree.category(i);
        match cat {
            Some(want) if want != c => 0,
            _ if c == Category::Sistema => 0,
            _ => n.alloc,
        }
    }
}

fn row(tree: &Tree, i: u32, cat: Option<Category>) -> ItemRow {
    let n = tree.node(i);
    ItemRow {
        node: i,
        frn: n.frn,
        name: tree.name(i).to_string(),
        path: tree.path(i),
        is_dir: n.is_dir(),
        size: n.size,
        alloc: n.alloc,
        bytes: bytes_of(tree, i, cat),
        modified: n.modified,
        category: tree.category(i),
        child_count: if n.is_dir() { tree.children(i).count() as u32 } else { 0 },
    }
}

fn listable(tree: &Tree, i: u32) -> bool {
    let n = tree.node(i);
    n.flags & (flags::DEAD | flags::LINK) == 0 && tree.category(i) != Category::Sistema
}

/// Filhos vivos de `dir` que ocupam algo na categoria, do maior para o menor.
/// `None` se a pasta não existe mais.
pub fn children_in(tree: &Tree, dir: u32, category: Option<Category>) -> Option<Vec<ItemRow>> {
    if !alive(tree, dir) || !tree.node(dir).is_dir() {
        return None;
    }
    let mut rows: Vec<ItemRow> = tree
        .children(dir)
        .filter(|&c| listable(tree, c))
        .filter(|&c| bytes_of(tree, c, category) > 0)
        .map(|c| row(tree, c, category))
        .collect();
    rows.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Some(rows)
}

fn is_under(tree: &Tree, mut i: u32, top: u32) -> bool {
    for _ in 0..MAX_DEPTH {
        if i == top {
            return true;
        }
        if i == tree.root || i == NONE {
            return false;
        }
        i = tree.node(i).parent;
    }
    false
}

fn extension(name: &str) -> &str {
    match name.rfind('.') {
        Some(k) if k > 0 => &name[k + 1..],
        _ => "",
    }
}

fn matches(tree: &Tree, i: u32, f: &FileFilter, search: Option<&str>, ext: Option<&str>) -> bool {
    let n = tree.node(i);
    if f.category.is_some_and(|c| c != tree.category(i)) {
        return false;
    }
    if f.min_size.is_some_and(|m| n.alloc < m) || f.max_size.is_some_and(|m| n.alloc > m) {
        return false;
    }
    if f.modified_before.is_some_and(|t| n.modified >= t) || f.modified_after.is_some_and(|t| n.modified <= t) {
        return false;
    }
    let name = tree.name(i);
    if let Some(e) = ext {
        if !extension(name).eq_ignore_ascii_case(e) {
            return false;
        }
    }
    if let Some(s) = search {
        if !name.to_lowercase().contains(s) {
            return false;
        }
    }
    if let Some(top) = f.under {
        if !is_under(tree, i, top) {
            return false;
        }
    }
    true
}

impl QueryCache {
    fn refresh(&mut self, tree: &Tree) {
        if self.version == tree.version() && !self.by_size.is_empty() {
            return;
        }
        let mut v: Vec<u32> = (0..tree.len() as u32).filter(|&i| !tree.node(i).is_dir() && listable(tree, i)).collect();
        v.sort_unstable_by(|&a, &b| tree.node(b).alloc.cmp(&tree.node(a).alloc).then(a.cmp(&b)));
        self.by_size = v;
        self.version = tree.version();
    }
}

/// Uma página da lista de arquivos (sem pastas e sem o Sistema).
pub fn query_files(tree: &Tree, cache: &mut QueryCache, filter: &FileFilter, sort: SortKey, offset: usize, limit: usize) -> Page {
    cache.refresh(tree);
    let limit = limit.min(MAX_LIMIT);
    let search = filter.search.as_ref().map(|s| s.to_lowercase()).filter(|s| !s.is_empty());
    let ext = filter.extension.as_deref().map(|e| e.trim_start_matches('.')).filter(|e| !e.is_empty());
    let unfiltered = filter.category.is_none()
        && filter.under.is_none()
        && ext.is_none()
        && search.is_none()
        && filter.min_size.is_none()
        && filter.max_size.is_none()
        && filter.modified_before.is_none()
        && filter.modified_after.is_none();

    let ordered: Box<dyn Iterator<Item = u32>> = match sort {
        SortKey::SizeAsc => Box::new(cache.by_size.iter().rev().copied()),
        _ => Box::new(cache.by_size.iter().copied()),
    };
    let mut picked: Vec<u32> = if unfiltered && matches!(sort, SortKey::SizeAsc | SortKey::SizeDesc) {
        let total = cache.by_size.len();
        let rows = ordered.skip(offset).take(limit).map(|i| row(tree, i, None)).collect();
        return Page { rows, total };
    } else {
        ordered.filter(|&i| matches(tree, i, filter, search.as_deref(), ext)).collect()
    };
    match sort {
        SortKey::NameAsc | SortKey::NameDesc => {
            picked.sort_by_cached_key(|&i| tree.name(i).to_lowercase());
            if sort == SortKey::NameDesc {
                picked.reverse();
            }
        }
        SortKey::ModifiedAsc => picked.sort_by_key(|&i| tree.node(i).modified),
        SortKey::ModifiedDesc => picked.sort_by_key(|&i| std::cmp::Reverse(tree.node(i).modified)),
        _ => {}
    }
    let total = picked.len();
    let rows = picked.into_iter().skip(offset).take(limit).map(|i| row(tree, i, None)).collect();
    Page { rows, total }
}
