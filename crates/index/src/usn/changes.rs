//! Traduz registros do USN em mudanças que a árvore sabe aplicar.
//!
//! Os motivos do USN se acumulam até o arquivo ser fechado, então só se age em
//! registros com `CLOSE`. A exceção é `RENAME_OLD_NAME`, que chega sozinho e é
//! guardado até o `RENAME_NEW_NAME` correspondente, mesmo entre lotes.

use std::collections::HashMap;

use super::record::{reason, UsnRecord, ATTR_DIRECTORY};

#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Created { frn: u64, frn_full: u64, parent: u64, name: String, is_dir: bool, attributes: u32 },
    Deleted { frn: u64, parent: u64, name: String },
    /// `frn_full`, `is_dir` e `attributes` permitem criar o nó se ele ainda não for conhecido.
    Moved {
        frn: u64,
        frn_full: u64,
        old_parent: u64,
        old_name: String,
        new_parent: u64,
        new_name: String,
        is_dir: bool,
        attributes: u32,
    },
    /// Um link foi criado ou removido; a árvore decide qual dos dois.
    LinkToggled { frn: u64, frn_full: u64, parent: u64, name: String },
    Modified { frn: u64, frn_full: u64 },
}

#[derive(Default)]
pub struct Coalescer {
    pending_renames: HashMap<u64, (u64, String)>,
    structural: Vec<Change>,
    modified: Vec<(u64, u64)>,
}

impl Coalescer {
    pub fn push(&mut self, rec: &UsnRecord) {
        let r = rec.reason;
        if r & reason::RENAME_OLD_NAME != 0 {
            self.pending_renames.insert(rec.frn, (rec.parent, rec.name.clone()));
            return;
        }
        if r & reason::CLOSE == 0 {
            return;
        }
        let is_dir = rec.attributes & ATTR_DIRECTORY != 0;
        if r & reason::FILE_DELETE != 0 {
            self.pending_renames.remove(&rec.frn);
            self.modified.retain(|&(f, _)| f != rec.frn);
            self.structural.push(Change::Deleted { frn: rec.frn, parent: rec.parent, name: rec.name.clone() });
            return;
        }
        if r & reason::RENAME_NEW_NAME != 0 && r & reason::FILE_CREATE != 0 {
            // Criado e renomeado antes de fechar: para quem lê, é só uma criação com o nome final.
            self.pending_renames.remove(&rec.frn);
            self.structural.push(self.created(rec, is_dir));
        } else if r & reason::RENAME_NEW_NAME != 0 {
            match self.pending_renames.remove(&rec.frn) {
                Some((old_parent, old_name)) => self.structural.push(Change::Moved {
                    frn: rec.frn,
                    frn_full: rec.frn_full,
                    old_parent,
                    old_name,
                    new_parent: rec.parent,
                    new_name: rec.name.clone(),
                    is_dir,
                    attributes: rec.attributes,
                }),
                None => self.structural.push(self.created(rec, is_dir)),
            }
        } else if r & reason::HARD_LINK_CHANGE != 0 {
            self.structural.push(Change::LinkToggled {
                frn: rec.frn,
                frn_full: rec.frn_full,
                parent: rec.parent,
                name: rec.name.clone(),
            });
        } else if r & reason::FILE_CREATE != 0 {
            self.structural.push(self.created(rec, is_dir));
        }
        if !is_dir && r & reason::ANY_DATA != 0 && !self.modified.iter().any(|&(f, _)| f == rec.frn) {
            self.modified.push((rec.frn, rec.frn_full));
        }
    }

    fn created(&self, rec: &UsnRecord, is_dir: bool) -> Change {
        Change::Created {
            frn: rec.frn,
            frn_full: rec.frn_full,
            parent: rec.parent,
            name: rec.name.clone(),
            is_dir,
            attributes: rec.attributes,
        }
    }

    /// Mudanças estruturais na ordem de chegada e, depois, um `Modified` por arquivo.
    pub fn drain(&mut self) -> Vec<Change> {
        let mut out = std::mem::take(&mut self.structural);
        out.extend(self.modified.drain(..).map(|(frn, frn_full)| Change::Modified { frn, frn_full }));
        out
    }
}
