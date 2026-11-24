//! Mudanças incrementais na árvore (vindas do USN Journal).
//!
//! Os totais nunca são recalculados por inteiro: cada mudança soma ou subtrai a
//! sua parte ao longo dos ancestrais.

use std::collections::HashMap;

use acervo_classify::{Classifier, DirCtx, CATEGORY_COUNT};

use super::{flags, Node, Tree, MAX_DEPTH, NONE};
use crate::usn::Change;

/// Tamanho atual de um arquivo, lido do disco.
#[derive(Clone, Copy, Debug)]
pub struct FileStat {
    pub size: u64,
    pub alloc: u64,
    pub modified: i64,
    pub attributes: u32,
}

#[derive(Default, Debug, PartialEq)]
pub struct ApplyReport {
    pub changed: bool,
    pub orphaned: usize,
}

type Vec10 = [u64; CATEGORY_COUNT];

impl Tree {
    /// Aplica um lote de mudanças. `stat` recebe a referência completa (`frn_full`).
    pub fn apply(&mut self, changes: &[Change], stat: &dyn Fn(u64) -> Option<FileStat>, classifier: &Classifier) -> ApplyReport {
        let mut report = ApplyReport::default();
        let mut ctx_cache: HashMap<u32, DirCtx> = HashMap::new();
        for change in changes {
            match change {
                Change::Created { frn, frn_full, parent, name, is_dir, attributes } => {
                    if let Some(existing) = self.node_for_frn(*frn) {
                        if !self.is_at(existing, *parent, name) {
                            // Já existe em outro lugar (renomeação cujo nome antigo se perdeu).
                            self.move_node(existing, *parent, name, classifier, &mut ctx_cache);
                            report.changed = true;
                        }
                        continue;
                    }
                    let st = if *is_dir { None } else { stat(*frn_full) };
                    let orphan = self.create(*frn, *parent, name, *is_dir, *attributes, st, classifier, &mut ctx_cache);
                    report.changed = true;
                    if orphan {
                        report.orphaned += 1;
                    }
                }
                Change::Deleted { frn, parent, name } => {
                    if let Some(n) = self.locate(*frn, *parent, name) {
                        self.remove(n);
                        ctx_cache.clear();
                        report.changed = true;
                    }
                }
                Change::Modified { frn, frn_full } => {
                    if let (Some(n), Some(st)) = (self.node_for_frn(*frn), stat(*frn_full)) {
                        if self.resize(n, st) {
                            report.changed = true;
                        }
                    }
                }
                Change::Moved { frn, frn_full, old_parent, old_name, new_parent, new_name, is_dir, attributes } => {
                    if let Some(n) = self.locate(*frn, *old_parent, old_name) {
                        let was_orphan = self.nodes[n as usize].flags & flags::ORPHAN != 0;
                        let now_orphan = self.move_node(n, *new_parent, new_name, classifier, &mut ctx_cache);
                        if now_orphan && !was_orphan {
                            report.orphaned += 1;
                        }
                    } else {
                        // Nunca visto (criado e renomeado antes de o índice saber): é uma criação.
                        let st = if *is_dir { None } else { stat(*frn_full) };
                        if self.create(*frn, *new_parent, new_name, *is_dir, *attributes, st, classifier, &mut ctx_cache) {
                            report.orphaned += 1;
                        }
                    }
                    report.changed = true;
                }
                Change::LinkToggled { frn, frn_full, parent, name } => {
                    if self.toggle_link(*frn, *frn_full, *parent, name, stat, classifier, &mut ctx_cache) {
                        report.changed = true;
                    }
                }
            }
        }
        if report.changed {
            self.bump_version();
        }
        report
    }

    /// Recalcula todas as categorias e todos os totais (as regras mudaram).
    pub fn reclassify_all(&mut self, classifier: &Classifier) {
        let ctx = classifier.root(self.drive);
        let root = self.root;
        self.nodes[root as usize].category = classifier.dir_category(&ctx) as u8;
        self.recompute_subtree(root, ctx, classifier);
        self.bump_version();
    }

    // ---------- mover e links ----------

    /// Muda um nó de lugar (ou de nome), reclassificando o que vai junto.
    /// Devolve `true` se ele ficou órfão.
    ///
    /// Limitação aceita: criar ou apagar um `.git`/`Cargo.toml` não reclassifica a
    /// pasta-mãe; isso só acontece na próxima análise completa.
    fn move_node(&mut self, n: u32, new_parent: u64, new_name: &str, classifier: &Classifier, cache: &mut HashMap<u32, DirCtx>) -> bool {
        let old_parent = self.nodes[n as usize].parent;
        let v_old = self.subtree_vec(n);
        if old_parent != NONE {
            self.add_to_ancestors(old_parent, &v_old, true);
        }
        self.unlink(n);
        let (np, orphan) = match self.node_for_frn(new_parent) {
            Some(p) if self.nodes[p as usize].is_dir() && !self.is_ancestor_or_self(n, p) => (p, false),
            _ => (self.root, true),
        };
        if orphan {
            self.orphans.entry(new_parent).or_default().push(n);
        }
        {
            let node = &mut self.nodes[n as usize];
            if orphan {
                node.flags |= flags::ORPHAN;
            } else {
                node.flags &= !flags::ORPHAN;
            }
        }
        self.set_name(n, new_name);
        self.link_child(np, n);
        cache.clear();
        let parent_ctx = self.ctx_for(np, classifier, cache);
        if self.nodes[n as usize].is_dir() {
            let ctx = {
                let has_child = |x: &str| self.children(n).any(|c| self.name(c).eq_ignore_ascii_case(x));
                let has_sibling = |x: &str| self.children(np).any(|c| c != n && self.name(c).eq_ignore_ascii_case(x));
                classifier.enter_dir(&parent_ctx, new_name, &has_child, &has_sibling)
            };
            self.nodes[n as usize].category = classifier.dir_category(&ctx) as u8;
            self.recompute_subtree(n, ctx, classifier);
        } else {
            self.nodes[n as usize].category = classifier.file_category(&parent_ctx, new_name) as u8;
        }
        let v_new = self.subtree_vec(n);
        self.add_to_ancestors(np, &v_new, false);
        orphan
    }

    /// `a` é `b` ou está acima de `b`? (mover `a` para dentro de `b` criaria um ciclo)
    fn is_ancestor_or_self(&self, a: u32, b: u32) -> bool {
        let mut cur = b;
        for _ in 0..=MAX_DEPTH {
            if cur == a {
                return true;
            }
            if cur == self.root || cur == NONE {
                return false;
            }
            cur = self.nodes[cur as usize].parent;
        }
        true
    }

    /// Outros nomes vivos do mesmo arquivo, pelo índice de links.
    fn other_links(&self, frn: u64, except: u32) -> Vec<u32> {
        self.links
            .get(&frn)
            .map(|v| {
                v.iter()
                    .copied()
                    .filter(|&i| i != except && self.nodes[i as usize].flags & flags::DEAD == 0 && self.nodes[i as usize].frn == frn)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Religa os órfãos que esperavam pela pasta de registro `frn`, recém-criada.
    fn relink_orphans(&mut self, frn: u64, classifier: &Classifier, cache: &mut HashMap<u32, DirCtx>) {
        let Some(waiting) = self.orphans.remove(&frn) else { return };
        for o in waiting {
            let node = &self.nodes[o as usize];
            if node.flags & flags::DEAD == 0 && node.flags & flags::ORPHAN != 0 && node.parent == self.root {
                let name = self.name(o).to_string();
                self.move_node(o, frn, &name, classifier, cache);
            }
        }
    }

    /// Um link foi criado ou removido. Devolve `true` se algo mudou.
    #[allow(clippy::too_many_arguments)]
    fn toggle_link(
        &mut self,
        frn: u64,
        frn_full: u64,
        parent: u64,
        name: &str,
        stat: &dyn Fn(u64) -> Option<FileStat>,
        classifier: &Classifier,
        cache: &mut HashMap<u32, DirCtx>,
    ) -> bool {
        let Some(primary) = self.node_for_frn(frn) else {
            // Nunca visto: é um arquivo novo.
            let st = stat(frn_full);
            self.create(frn, parent, name, false, 0x20, st, classifier, cache);
            return true;
        };
        let existing = if self.is_at(primary, parent, name) {
            Some(primary)
        } else {
            self.node_for_frn(parent).and_then(|p| {
                self.children(p).find(|&c| self.nodes[c as usize].frn == frn && self.name(c).eq_ignore_ascii_case(name))
            })
        };
        match existing {
            Some(n) if n == primary => {
                // Removeu o nome principal: outro link herda o tamanho.
                let others = self.other_links(frn, n);
                let Some(&heir) = others.first() else {
                    self.remove(n);
                    cache.clear();
                    return true;
                };
                let v = self.subtree_vec(n);
                let old_parent = self.nodes[n as usize].parent;
                self.add_to_ancestors(old_parent, &v, true);
                let (size, alloc) = (self.nodes[n as usize].size, self.nodes[n as usize].alloc);
                self.unlink(n);
                self.kill_subtree(n);
                {
                    let h = &mut self.nodes[heir as usize];
                    h.size = size;
                    h.alloc = alloc;
                    h.flags &= !flags::LINK;
                    if others.len() == 1 {
                        h.flags &= !flags::HAS_LINKS;
                    }
                }
                self.by_record[frn as usize] = heir;
                let v = self.subtree_vec(heir);
                let hp = self.nodes[heir as usize].parent;
                self.add_to_ancestors(hp, &v, false);
                cache.clear();
                true
            }
            Some(n) => {
                // Removeu um link secundário (não carrega tamanho).
                self.unlink(n);
                self.kill_subtree(n);
                cache.clear();
                true
            }
            None => {
                // Link novo: tamanho 0, o principal continua com os bytes.
                let (pn, orphan) = match self.node_for_frn(parent) {
                    Some(p) if self.nodes[p as usize].is_dir() => (p, false),
                    _ => (self.root, true),
                };
                let ctx = self.ctx_for(pn, classifier, cache);
                let category = classifier.file_category(&ctx, name) as u8;
                let mut f = flags::LINK | flags::HAS_LINKS;
                if orphan {
                    f |= flags::ORPHAN;
                }
                let link = self.insert_node(pn, name, f, category, 0, 0, 0, frn);
                self.nodes[primary as usize].flags |= flags::HAS_LINKS;
                let list = self.links.entry(frn).or_default();
                if !list.contains(&primary) {
                    list.push(primary);
                }
                list.push(link);
                true
            }
        }
    }

    /// Recalcula categorias e totais de uma subárvore a partir do contexto do topo.
    fn recompute_subtree(&mut self, top: u32, top_ctx: DirCtx, classifier: &Classifier) {
        let mut order = Vec::new();
        let mut stack = vec![(top, top_ctx)];
        // Limite de segurança: nunca visita mais nós do que existem (protege contra ciclos).
        let limit = self.nodes.len();
        while let Some((d, ctx)) = stack.pop() {
            if order.len() > limit {
                break;
            }
            order.push(d);
            let kids: Vec<u32> = self.children(d).collect();
            for c in kids {
                if self.nodes[c as usize].is_dir() {
                    let child_ctx = {
                        let has_child = |x: &str| self.children(c).any(|k| self.name(k).eq_ignore_ascii_case(x));
                        let has_sibling = |x: &str| self.children(d).any(|k| self.name(k).eq_ignore_ascii_case(x));
                        classifier.enter_dir(&ctx, self.name(c), &has_child, &has_sibling)
                    };
                    self.nodes[c as usize].category = classifier.dir_category(&child_ctx) as u8;
                    stack.push((c, child_ctx));
                } else {
                    self.nodes[c as usize].category = classifier.file_category(&ctx, self.name(c)) as u8;
                    order.push(c);
                }
            }
        }
        for &i in &order {
            let s = self.nodes[i as usize].slot;
            if s != NONE {
                self.totals[s as usize] = [0; CATEGORY_COUNT];
            }
        }
        for &i in order.iter().rev() {
            let node = &self.nodes[i as usize];
            let (cat, parent) = (node.category as usize, node.parent);
            if node.is_dir() {
                let s = node.slot as usize;
                self.totals[s][cat] += node.size;
                if i != top {
                    let ps = self.nodes[parent as usize].slot as usize;
                    let own = self.totals[s];
                    for (dst, v) in self.totals[ps].iter_mut().zip(own) {
                        *dst += v;
                    }
                }
            } else if i != top {
                let ps = self.nodes[parent as usize].slot as usize;
                self.totals[ps][cat] += node.alloc;
            }
        }
        for &i in &order {
            let s = self.nodes[i as usize].slot;
            if s != NONE {
                self.nodes[i as usize].alloc = self.totals[s as usize].iter().sum();
            }
        }
    }

    // ---------- leitura ----------

    /// O nó `n` está sob o pai de registro `parent` com o nome `name`?
    pub(super) fn is_at(&self, n: u32, parent: u64, name: &str) -> bool {
        let p = self.nodes[n as usize].parent;
        p != NONE && self.nodes[p as usize].frn == parent && self.name(n).eq_ignore_ascii_case(name)
    }

    /// Acha o nó de um nome: o principal, se estiver nesse lugar; senão o link com esse nome no pai.
    pub(super) fn locate(&self, frn: u64, parent: u64, name: &str) -> Option<u32> {
        let primary = self.node_for_frn(frn)?;
        if self.is_at(primary, parent, name) {
            return Some(primary);
        }
        if let Some(p) = self.node_for_frn(parent) {
            if let Some(c) = self.children(p).find(|&c| self.nodes[c as usize].frn == frn && self.name(c).eq_ignore_ascii_case(name)) {
                return Some(c);
            }
        }
        Some(primary)
    }

    /// Contribuição de um nó para os totais dos ancestrais.
    pub(super) fn subtree_vec(&self, n: u32) -> Vec10 {
        let node = &self.nodes[n as usize];
        if node.is_dir() {
            self.totals[node.slot as usize]
        } else {
            let mut v = [0u64; CATEGORY_COUNT];
            v[node.category as usize] = node.alloc;
            v
        }
    }

    /// Contexto de classificação de uma pasta, descendo da raiz.
    pub(super) fn ctx_for(&self, dir: u32, classifier: &Classifier, cache: &mut HashMap<u32, DirCtx>) -> DirCtx {
        if let Some(c) = cache.get(&dir) {
            return c.clone();
        }
        let mut chain = Vec::new();
        let mut cur = dir;
        while cur != self.root && chain.len() <= MAX_DEPTH {
            chain.push(cur);
            cur = self.nodes[cur as usize].parent;
        }
        let mut ctx = classifier.root(self.drive);
        for &d in chain.iter().rev() {
            if let Some(c) = cache.get(&d) {
                ctx = c.clone();
                continue;
            }
            let parent = self.nodes[d as usize].parent;
            let has_child = |n: &str| self.children(d).any(|c| self.name(c).eq_ignore_ascii_case(n));
            let has_sibling = |n: &str| self.children(parent).any(|c| self.name(c).eq_ignore_ascii_case(n));
            ctx = classifier.enter_dir(&ctx, self.name(d), &has_child, &has_sibling);
            cache.insert(d, ctx.clone());
        }
        ctx
    }

    // ---------- escrita ----------

    /// Soma (ou subtrai) `v` em `start` e em todos os ancestrais dele.
    pub(super) fn add_to_ancestors(&mut self, start: u32, v: &Vec10, subtract: bool) {
        let sum: u64 = v.iter().sum();
        let mut cur = start;
        for _ in 0..=MAX_DEPTH {
            let node = &mut self.nodes[cur as usize];
            let slot = node.slot as usize;
            if subtract {
                node.alloc = node.alloc.saturating_sub(sum);
                for (t, x) in self.totals[slot].iter_mut().zip(v) {
                    *t = t.saturating_sub(*x);
                }
            } else {
                node.alloc += sum;
                for (t, x) in self.totals[slot].iter_mut().zip(v) {
                    *t += *x;
                }
            }
            if cur == self.root {
                break;
            }
            cur = self.nodes[cur as usize].parent;
        }
    }

    pub(super) fn link_child(&mut self, parent: u32, n: u32) {
        let next = self.nodes[parent as usize].first_child;
        {
            let node = &mut self.nodes[n as usize];
            node.parent = parent;
            node.prev_sibling = NONE;
            node.next_sibling = next;
        }
        if next != NONE {
            self.nodes[next as usize].prev_sibling = n;
        }
        self.nodes[parent as usize].first_child = n;
    }

    pub(super) fn unlink(&mut self, n: u32) {
        let (parent, prev, next) = {
            let node = &self.nodes[n as usize];
            (node.parent, node.prev_sibling, node.next_sibling)
        };
        if prev != NONE {
            self.nodes[prev as usize].next_sibling = next;
        } else if parent != NONE {
            self.nodes[parent as usize].first_child = next;
        }
        if next != NONE {
            self.nodes[next as usize].prev_sibling = prev;
        }
        let node = &mut self.nodes[n as usize];
        node.prev_sibling = NONE;
        node.next_sibling = NONE;
    }

    pub(super) fn set_name(&mut self, n: u32, name: &str) {
        let name = if name.len() > u16::MAX as usize { &name[..u16::MAX as usize] } else { name };
        let off = self.names.len() as u32;
        self.names.push_str(name);
        let node = &mut self.nodes[n as usize];
        node.name_off = off;
        node.name_len = name.len() as u16;
    }

    fn alloc_slot(&mut self) -> u32 {
        match self.free_slots.pop() {
            Some(s) => {
                self.totals[s as usize] = [0; CATEGORY_COUNT];
                s
            }
            None => {
                self.totals.push([0; CATEGORY_COUNT]);
                (self.totals.len() - 1) as u32
            }
        }
    }

    fn alloc_node(&mut self, node: Node) -> u32 {
        match self.free_nodes.pop() {
            Some(i) => {
                self.nodes[i as usize] = node;
                i
            }
            None => {
                self.nodes.push(node);
                (self.nodes.len() - 1) as u32
            }
        }
    }

    /// Põe um nó novo sob `parent` (sem registrá-lo no mapa de registros nem somar totais).
    #[allow(clippy::too_many_arguments)]
    fn insert_node(&mut self, parent: u32, name: &str, f: u16, category: u8, size: u64, alloc: u64, modified: i64, frn: u64) -> u32 {
        let slot = if f & flags::DIR != 0 { self.alloc_slot() } else { NONE };
        let n = self.alloc_node(Node {
            parent: NONE,
            first_child: NONE,
            next_sibling: NONE,
            prev_sibling: NONE,
            name_off: 0,
            name_len: 0,
            flags: f,
            category,
            size,
            alloc,
            modified,
            frn,
            slot,
        });
        self.set_name(n, name);
        self.link_child(parent, n);
        n
    }

    /// Cria um nó. Devolve `true` se ele ficou órfão (pai desconhecido).
    #[allow(clippy::too_many_arguments)]
    fn create(
        &mut self,
        frn: u64,
        parent: u64,
        name: &str,
        is_dir: bool,
        attributes: u32,
        st: Option<FileStat>,
        classifier: &Classifier,
        cache: &mut HashMap<u32, DirCtx>,
    ) -> bool {
        let (parent_node, orphan) = match self.node_for_frn(parent) {
            Some(p) if self.nodes[p as usize].is_dir() => (p, false),
            _ => (self.root, true),
        };
        let parent_ctx = self.ctx_for(parent_node, classifier, cache);
        let mut f = if is_dir { flags::DIR } else { 0 };
        if orphan {
            f |= flags::ORPHAN;
        }
        if attributes & 0x2 != 0 {
            f |= flags::HIDDEN;
        }
        if attributes & 0x4 != 0 {
            f |= flags::SYSTEM;
        }
        if attributes & 0x400 != 0 {
            f |= flags::REPARSE;
        }
        let (size, alloc, modified) = st.map(|s| (s.size, s.alloc, s.modified)).unwrap_or((0, 0, 0));
        let category = if is_dir {
            let has_sibling = |n: &str| self.children(parent_node).any(|c| self.name(c).eq_ignore_ascii_case(n));
            let ctx = classifier.enter_dir(&parent_ctx, name, &|_| false, &has_sibling);
            classifier.dir_category(&ctx)
        } else {
            classifier.file_category(&parent_ctx, name)
        };
        let (size, alloc) = if is_dir { (0, 0) } else { (size, alloc) };
        let n = self.insert_node(parent_node, name, f, category as u8, size, alloc, modified, frn);
        let i = frn as usize;
        if i >= self.by_record.len() {
            self.by_record.resize(i + 1, NONE);
        }
        self.by_record[i] = n;
        if !is_dir && alloc > 0 {
            let mut v = [0u64; CATEGORY_COUNT];
            v[category as usize] = alloc;
            self.add_to_ancestors(parent_node, &v, false);
        }
        if orphan {
            self.orphans.entry(parent).or_default().push(n);
        }
        if is_dir {
            self.relink_orphans(frn, classifier, cache);
        }
        orphan
    }

    /// Tira um nó (e tudo abaixo dele) da árvore, devolvendo-os às listas livres.
    pub(super) fn remove(&mut self, n: u32) {
        let parent = self.nodes[n as usize].parent;
        let v = self.subtree_vec(n);
        if parent != NONE {
            self.add_to_ancestors(parent, &v, true);
        }
        self.unlink(n);
        let frn = self.nodes[n as usize].frn;
        let had_links = self.nodes[n as usize].flags & flags::HAS_LINKS != 0;
        let was_primary = self.nodes[n as usize].flags & flags::LINK == 0;
        self.kill_subtree(n);
        // Apagar o arquivo de vez apaga também os outros nomes dele.
        if was_primary && had_links {
            let links: Vec<u32> = self.other_links(frn, n);
            for l in links {
                self.unlink(l);
                self.kill_subtree(l);
            }
        }
    }

    pub(super) fn kill_subtree(&mut self, top: u32) {
        let mut stack = vec![top];
        while let Some(i) = stack.pop() {
            stack.extend(self.children(i));
            let node = &mut self.nodes[i as usize];
            node.flags |= flags::DEAD;
            node.first_child = NONE;
            let (frn, slot, linked) = (node.frn, node.slot, node.flags & flags::HAS_LINKS != 0);
            if slot != NONE {
                self.free_slots.push(slot);
            }
            if linked {
                if let Some(list) = self.links.get_mut(&frn) {
                    list.retain(|&x| x != i);
                    if list.is_empty() {
                        self.links.remove(&frn);
                    }
                }
            }
            if self.by_record.get(frn as usize) == Some(&i) {
                self.by_record[frn as usize] = NONE;
            }
            self.free_nodes.push(i);
        }
    }

    /// Atualiza o tamanho de um arquivo. Devolve `true` se algo mudou.
    fn resize(&mut self, n: u32, st: FileStat) -> bool {
        let node = &self.nodes[n as usize];
        if node.is_dir() || node.flags & flags::LINK != 0 {
            return false;
        }
        let (old, cat, parent) = (node.alloc, node.category as usize, node.parent);
        {
            let node = &mut self.nodes[n as usize];
            node.size = st.size;
            node.modified = st.modified;
            node.alloc = st.alloc;
        }
        if st.alloc == old {
            return false;
        }
        let mut v = [0u64; CATEGORY_COUNT];
        if st.alloc > old {
            v[cat] = st.alloc - old;
            self.add_to_ancestors(parent, &v, false);
        } else {
            v[cat] = old - st.alloc;
            self.add_to_ancestors(parent, &v, true);
        }
        true
    }
}
