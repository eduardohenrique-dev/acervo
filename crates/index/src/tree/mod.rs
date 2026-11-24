//! Árvore de arquivos em memória, com totais por categoria.

use acervo_classify::{Category, Classifier, DirCtx, CATEGORY_COUNT};
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::mft::ParsedRecord;

/// Índice nulo.
pub const NONE: u32 = u32::MAX;

/// Bits de `Node::flags`.
pub mod flags {
    pub const DIR: u16 = 1;
    pub const HIDDEN: u16 = 2;
    pub const SYSTEM: u16 = 4;
    pub const LINK: u16 = 8;
    pub const REPARSE: u16 = 16;
    pub const ADS: u16 = 32;
    pub const ORPHAN: u16 = 64;
    /// Nó apagado (está na lista livre e pode ser reutilizado).
    pub const DEAD: u16 = 128;
    /// O arquivo tem mais de um nome (hard links); só esses exigem procurar os outros links.
    pub const HAS_LINKS: u16 = 256;
}

mod apply;
pub use apply::{ApplyReport, FileStat};

static VERSION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn next_version() -> u64 {
    VERSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

const ATTR_HIDDEN: u32 = 0x2;
const ATTR_SYSTEM: u32 = 0x4;
const ATTR_REPARSE: u32 = 0x400;
/// `$BadClus`: o fluxo `$Bad` é esparso e declara o tamanho do volume inteiro.
const RECORD_BADCLUS: u64 = 8;
const MAX_DEPTH: usize = 4096;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub parent: u32,
    pub first_child: u32,
    pub next_sibling: u32,
    pub prev_sibling: u32,
    name_off: u32,
    name_len: u16,
    pub flags: u16,
    pub category: u8,
    pub size: u64,
    /// Arquivos: bytes no disco. Pastas: soma de tudo abaixo (inclusive os próprios índices).
    pub alloc: u64,
    pub modified: i64,
    pub frn: u64,
    /// Índice em `totals` para pastas; `NONE` para arquivos.
    pub slot: u32,
}

impl Node {
    pub fn is_dir(&self) -> bool {
        self.flags & flags::DIR != 0
    }
}

/// Árvore inteira de um disco. Serializável para o snapshot (Fase 2).
#[derive(Serialize, Deserialize)]
pub struct Tree {
    pub drive: char,
    pub root: u32,
    pub skipped_records: u64,
    /// Muda a cada alteração; única entre árvores (inclusive as carregadas de snapshot).
    #[serde(skip, default = "next_version")]
    version: u64,
    nodes: Vec<Node>,
    names: String,
    totals: Vec<[u64; CATEGORY_COUNT]>,
    /// Número do registro → primeiro nó (o que carrega o tamanho).
    by_record: Vec<u32>,
    free_nodes: Vec<u32>,
    free_slots: Vec<u32>,
    /// Registro → todos os nós (nomes) dele; só para arquivos com mais de um nome.
    links: HashMap<u64, Vec<u32>>,
    /// Registro do pai esperado → órfãos que estão na raiz esperando por ele.
    orphans: HashMap<u64, Vec<u32>>,
}

impl std::fmt::Debug for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tree")
            .field("drive", &self.drive)
            .field("nodes", &self.nodes.len())
            .field("skipped_records", &self.skipped_records)
            .finish()
    }
}

impl Tree {
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Confere que todos os índices internos apontam para dentro dos vetores.
    /// Usado ao carregar um snapshot: um arquivo adulterado nunca deve causar pânico.
    pub fn validate(&self) -> bool {
        let n = self.nodes.len();
        let ok_node = |i: u32| i == NONE || (i as usize) < n;
        if (self.root as usize) >= n || self.nodes[self.root as usize].slot == NONE {
            return false;
        }
        for node in &self.nodes {
            if !(ok_node(node.parent) && ok_node(node.first_child) && ok_node(node.next_sibling) && ok_node(node.prev_sibling)) {
                return false;
            }
            if node.slot != NONE && node.slot as usize >= self.totals.len() {
                return false;
            }
            if node.is_dir() && node.slot == NONE && node.flags & flags::DEAD == 0 {
                return false;
            }
            let end = node.name_off as usize + node.name_len as usize;
            if end > self.names.len() || !self.names.is_char_boundary(node.name_off as usize) || !self.names.is_char_boundary(end) {
                return false;
            }
            if node.category as usize >= CATEGORY_COUNT {
                return false;
            }
        }
        let all_in = |v: &[u32]| v.iter().all(|&i| (i as usize) < n);
        self.by_record.iter().all(|&i| ok_node(i))
            && all_in(&self.free_nodes)
            && self.free_slots.iter().all(|&s| (s as usize) < self.totals.len())
            && self.links.values().all(|v| all_in(v))
            && self.orphans.values().all(|v| all_in(v))
    }

    /// Nós vivos (sem os apagados que esperam reuso).
    /// Versão atual (para caches de consulta).
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Marca a árvore como alterada.
    pub(crate) fn bump_version(&mut self) {
        self.version = next_version();
    }

    pub fn live_len(&self) -> usize {
        self.nodes.len() - self.free_nodes.len()
    }

    /// Primeiro nó vivo de um registro da MFT.
    pub fn node_for_frn(&self, frn: u64) -> Option<u32> {
        match self.by_record.get(frn as usize).copied() {
            Some(n) if n != NONE && self.nodes[n as usize].flags & flags::DEAD == 0 => Some(n),
            _ => None,
        }
    }

    pub fn node(&self, i: u32) -> &Node {
        &self.nodes[i as usize]
    }

    pub fn name(&self, i: u32) -> &str {
        let n = &self.nodes[i as usize];
        &self.names[n.name_off as usize..n.name_off as usize + n.name_len as usize]
    }

    pub fn category(&self, i: u32) -> Category {
        Category::from_u8(self.nodes[i as usize].category).unwrap_or(Category::Outros)
    }

    pub fn children(&self, i: u32) -> impl Iterator<Item = u32> + '_ {
        let mut cur = self.nodes[i as usize].first_child;
        std::iter::from_fn(move || {
            if cur == NONE {
                return None;
            }
            let out = cur;
            cur = self.nodes[cur as usize].next_sibling;
            Some(out)
        })
    }

    /// Totais por categoria de uma pasta (arquivos: zeros).
    pub fn totals(&self, dir: u32) -> &[u64; CATEGORY_COUNT] {
        const ZERO: [u64; CATEGORY_COUNT] = [0; CATEGORY_COUNT];
        match self.nodes[dir as usize].slot {
            NONE => &ZERO,
            s => &self.totals[s as usize],
        }
    }

    /// Caminho completo, ex.: `C:\Users\Ana\a.txt`; a raiz é `C:\`.
    pub fn path(&self, i: u32) -> String {
        let mut parts = Vec::new();
        let mut cur = i;
        while cur != self.root && parts.len() <= MAX_DEPTH {
            parts.push(self.name(cur));
            cur = self.nodes[cur as usize].parent;
        }
        let mut s = format!("{}:\\", self.drive);
        for (k, p) in parts.iter().rev().enumerate() {
            if k > 0 {
                s.push('\\');
            }
            s.push_str(p);
        }
        s
    }

    /// Procura um caminho absoluto (sem diferenciar maiúsculas).
    pub fn find(&self, path: &str) -> Option<u32> {
        let mut parts = path.split('\\').filter(|s| !s.is_empty());
        let drive = parts.next()?;
        if !drive.to_lowercase().starts_with(&self.drive.to_lowercase().to_string()) {
            return None;
        }
        let mut cur = self.root;
        for part in parts {
            let lower = part.to_lowercase();
            cur = self.children(cur).find(|&c| self.name(c).to_lowercase() == lower)?;
        }
        Some(cur)
    }
}

/// Recebe registros da MFT (ou da varredura alternativa) e monta a `Tree`.
pub struct TreeBuilder {
    drive: char,
    root_record: u64,
    records: Vec<Option<ParsedRecord>>,
    extensions: Vec<ParsedRecord>,
    skipped: u64,
}

impl TreeBuilder {
    pub fn new(drive: char, root_record: u64, capacity: usize) -> TreeBuilder {
        TreeBuilder {
            drive,
            root_record,
            records: Vec::with_capacity(capacity),
            extensions: Vec::new(),
            skipped: 0,
        }
    }

    pub fn push(&mut self, rec: ParsedRecord) {
        if !rec.in_use {
            return;
        }
        if rec.base_record != 0 {
            self.extensions.push(rec);
            return;
        }
        let i = rec.record_no as usize;
        if i >= self.records.len() {
            self.records.resize_with(i + 1, || None);
        }
        self.records[i] = Some(rec);
    }

    /// Conta um registro descartado (fixup ou estrutura inválida).
    pub fn skip(&mut self) {
        self.skipped += 1;
    }

    fn merge_extensions(&mut self) {
        for ext in std::mem::take(&mut self.extensions) {
            let Some(Some(base)) = self.records.get_mut(ext.base_record as usize) else {
                continue;
            };
            base.names.extend(ext.names);
            if base.size == 0 && base.alloc == 0 {
                base.size = ext.size;
                base.alloc = ext.alloc;
            }
            base.ads_alloc += ext.ads_alloc;
            if base.index_alloc == 0 {
                base.index_alloc = ext.index_alloc;
            }
        }
    }

    pub fn finish(mut self, classifier: &Classifier) -> Tree {
        self.merge_extensions();
        let mut names = String::new();
        let mut nodes: Vec<Node> = Vec::new();
        let mut parent_rec: Vec<u64> = Vec::new();
        let mut by_record: Vec<u32> = vec![NONE; self.records.len().max(self.root_record as usize + 1)];

        let new_node = |names: &mut String, name: &str, rec: &ParsedRecord, first: bool| -> Node {
            let off = names.len() as u32;
            let name = if name.len() > u16::MAX as usize { &name[..u16::MAX as usize] } else { name };
            names.push_str(name);
            let mut f = 0u16;
            if rec.is_dir {
                f |= flags::DIR;
            }
            if rec.attributes & ATTR_HIDDEN != 0 {
                f |= flags::HIDDEN;
            }
            if rec.attributes & ATTR_SYSTEM != 0 {
                f |= flags::SYSTEM;
            }
            if rec.attributes & ATTR_REPARSE != 0 {
                f |= flags::REPARSE;
            }
            let ads = if rec.record_no == RECORD_BADCLUS { 0 } else { rec.ads_alloc };
            if ads > 0 {
                f |= flags::ADS;
            }
            let (size, alloc) = match (first, rec.is_dir) {
                (false, _) => {
                    f |= flags::LINK;
                    (0, 0)
                }
                (true, true) => (rec.index_alloc, rec.index_alloc),
                (true, false) => (rec.size, rec.alloc + ads),
            };
            Node {
                parent: NONE,
                first_child: NONE,
                next_sibling: NONE,
                prev_sibling: NONE,
                name_off: off,
                name_len: name.len() as u16,
                flags: f,
                category: Category::Outros as u8,
                size,
                alloc,
                modified: rec.modified,
                frn: rec.record_no,
                slot: NONE,
            }
        };

        // Raiz primeiro (sintética se faltar).
        let root_rec = self
            .records
            .get_mut(self.root_record as usize)
            .and_then(Option::take)
            .unwrap_or(ParsedRecord {
                record_no: self.root_record,
                in_use: true,
                is_dir: true,
                ..Default::default()
            });
        let mut root = new_node(&mut names, "", &root_rec, true);
        root.flags |= flags::DIR;
        nodes.push(root);
        parent_rec.push(self.root_record);
        by_record[self.root_record as usize] = 0;

        let mut links: HashMap<u64, Vec<u32>> = HashMap::new();
        for rec in self.records.iter().flatten() {
            for (k, n) in rec.names.iter().enumerate() {
                let i = nodes.len() as u32;
                let mut node = new_node(&mut names, &n.name, rec, k == 0);
                if rec.names.len() > 1 {
                    node.flags |= flags::HAS_LINKS;
                    links.entry(rec.record_no).or_default().push(i);
                }
                nodes.push(node);
                parent_rec.push(n.parent);
                if k == 0 {
                    by_record[rec.record_no as usize] = i;
                }
            }
        }

        // Pai provisório; pai inexistente ou que não é pasta → raiz (órfão).
        let count = nodes.len();
        let mut orphans: HashMap<u64, Vec<u32>> = HashMap::new();
        for i in 1..count {
            let p = by_record.get(parent_rec[i] as usize).copied().unwrap_or(NONE);
            if p == NONE || p as usize == i || !nodes[p as usize].is_dir() {
                nodes[i].parent = 0;
                nodes[i].flags |= flags::ORPHAN;
                if p == NONE {
                    orphans.entry(parent_rec[i]).or_default().push(i as u32);
                }
            } else {
                nodes[i].parent = p;
            }
        }
        break_cycles(&mut nodes);

        // Listas de filhos, preservando a ordem.
        for i in (1..count).rev() {
            let p = nodes[i].parent as usize;
            let next = nodes[p].first_child;
            nodes[i].next_sibling = next;
            if next != NONE {
                nodes[next as usize].prev_sibling = i as u32;
            }
            nodes[p].first_child = i as u32;
        }

        let mut tree = Tree {
            drive: self.drive,
            root: 0,
            skipped_records: self.skipped,
            nodes,
            names,
            totals: Vec::new(),
            by_record,
            free_nodes: Vec::new(),
            free_slots: Vec::new(),
            version: next_version(),
            links,
            orphans,
        };
        let order = classify(&mut tree, classifier);
        compute_totals(&mut tree, &order);
        tree
    }
}

/// Garante que toda cadeia de pais termina na raiz.
fn break_cycles(nodes: &mut [Node]) {
    // 0 = desconhecido, 1 = chega à raiz, 2 = em visita
    let mut state = vec![0u8; nodes.len()];
    state[0] = 1;
    let mut chain = Vec::new();
    for start in 0..nodes.len() {
        if state[start] != 0 {
            continue;
        }
        chain.clear();
        let mut cur = start;
        let cycle = loop {
            match state[cur] {
                1 => break false,
                2 => break true,
                _ => {
                    state[cur] = 2;
                    chain.push(cur);
                    if chain.len() > MAX_DEPTH {
                        break true;
                    }
                    cur = nodes[cur].parent as usize;
                }
            }
        };
        if cycle {
            let last = *chain.last().unwrap();
            nodes[last].parent = 0;
            nodes[last].flags |= flags::ORPHAN;
        }
        for &c in &chain {
            state[c] = 1;
        }
    }
}

/// Atribui categorias descendo da raiz; devolve a ordem de visita (pré-ordem).
fn classify(tree: &mut Tree, classifier: &Classifier) -> Vec<u32> {
    let mut cats = vec![Category::Outros as u8; tree.nodes.len()];
    let mut order = Vec::with_capacity(tree.nodes.len());
    let root_ctx = classifier.root(tree.drive);
    cats[0] = classifier.dir_category(&root_ctx) as u8;
    let mut stack: Vec<(u32, DirCtx)> = vec![(0, root_ctx)];
    {
        let t: &Tree = tree;
        let has_named_child = |dir: u32, name: &str| t.children(dir).any(|c| t.name(c).eq_ignore_ascii_case(name));
        while let Some((dir, ctx)) = stack.pop() {
            order.push(dir);
            for child in t.children(dir) {
                let name = t.name(child);
                if t.node(child).is_dir() {
                    let has_child = |n: &str| has_named_child(child, n);
                    let has_sibling = |n: &str| has_named_child(dir, n);
                    let child_ctx = classifier.enter_dir(&ctx, name, &has_child, &has_sibling);
                    cats[child as usize] = classifier.dir_category(&child_ctx) as u8;
                    stack.push((child, child_ctx));
                } else {
                    cats[child as usize] = classifier.file_category(&ctx, name) as u8;
                    order.push(child);
                }
            }
        }
    }
    for (n, c) in tree.nodes.iter_mut().zip(cats) {
        n.category = c;
    }
    order
}

/// Soma os bytes de baixo para cima. `order` lista pais antes dos filhos.
fn compute_totals(tree: &mut Tree, order: &[u32]) {
    let mut slots = 0u32;
    for n in tree.nodes.iter_mut().filter(|n| n.is_dir()) {
        n.slot = slots;
        slots += 1;
    }
    let mut totals = vec![[0u64; CATEGORY_COUNT]; slots as usize];
    for &i in order.iter().rev() {
        let n = &tree.nodes[i as usize];
        let cat = n.category as usize;
        if n.is_dir() {
            let s = n.slot as usize;
            totals[s][cat] += n.alloc;
            if i != tree.root {
                let ps = tree.nodes[n.parent as usize].slot as usize;
                let own = totals[s];
                for (dst, v) in totals[ps].iter_mut().zip(own) {
                    *dst += v;
                }
            }
        } else {
            let ps = tree.nodes[n.parent as usize].slot as usize;
            totals[ps][cat] += n.alloc;
        }
    }
    for n in tree.nodes.iter_mut().filter(|n| n.is_dir()) {
        n.alloc = totals[n.slot as usize].iter().sum();
    }
    tree.totals = totals;
}

#[cfg(test)]
mod validate_tests {
    use super::*;
    use crate::mft::FileName;

    fn small() -> Tree {
        let mut b = TreeBuilder::new('C', 5, 16);
        let rec = |no: u64, parent: u64, name: &str, dir: bool| ParsedRecord {
            record_no: no,
            in_use: true,
            is_dir: dir,
            names: vec![FileName { parent, name: name.into(), namespace: 1 }],
            alloc: 4096,
            size: 4096,
            ..Default::default()
        };
        b.push(rec(5, 5, ".", true));
        b.push(rec(30, 5, "a", true));
        b.push(rec(31, 30, "x.bin", false));
        b.finish(&Classifier::new(vec![]))
    }

    #[test]
    fn valid_tree_passes() {
        assert!(small().validate());
    }

    #[test]
    fn out_of_range_indices_fail() {
        let mut t = small();
        t.nodes[2].parent = 999;
        assert!(!t.validate());
        let mut t = small();
        t.nodes[1].slot = 999;
        assert!(!t.validate());
        let mut t = small();
        t.nodes[2].name_len = 60_000;
        assert!(!t.validate());
        let mut t = small();
        t.root = 50;
        assert!(!t.validate());
    }
}
