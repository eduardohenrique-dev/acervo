//! Referências a itens vindas da interface, conferidas contra a árvore atual.

use std::path::PathBuf;

use acervo_index::{flags, Tree};
use acervo_ops::ItemFacts;
use serde::{Deserialize, Serialize};

/// Um item como a interface o conhece: o nó, o registro da MFT e o caminho que ela mostrou.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemRef {
    pub drive: char,
    pub node: u32,
    pub frn: u64,
    /// O caminho mostrado na tela. Nó e frn podem ser reaproveitados por outro arquivo;
    /// o caminho garante que a ação cai no item que a pessoa viu.
    pub path: String,
}

/// O item conferido: fatos para a política e dados para atualizar a árvore depois.
#[derive(Clone, Debug)]
pub struct Resolved {
    pub facts: ItemFacts,
    pub parent_frn: u64,
    pub name: String,
    pub bytes: u64,
}

fn same_path(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b) || a.to_lowercase() == b.to_lowercase()
}

/// A pasta aberta na tela ainda é a mesma (o nó pode ter sido reaproveitado por outra pasta)?
pub fn folder_matches(tree: &Tree, node: u32, path: &str) -> bool {
    (node as usize) < tree.len()
        && tree.node(node).flags & flags::DEAD == 0
        && tree.node(node).is_dir()
        && same_path(&tree.path(node), path)
}

/// `None` se o nó não existe mais, foi apagado ou agora é outro arquivo.
pub fn resolve(tree: &Tree, r: &ItemRef) -> Option<Resolved> {
    if r.node as usize >= tree.len() {
        return None;
    }
    let n = tree.node(r.node);
    if n.flags & flags::DEAD != 0 || n.frn != r.frn || r.node == tree.root {
        return None;
    }
    let path = tree.path(r.node);
    if !same_path(&path, &r.path) {
        return None;
    }
    let parent = n.parent;
    Some(Resolved {
        facts: ItemFacts {
            path: PathBuf::from(path),
            category: tree.category(r.node),
            is_dir: n.is_dir(),
            is_reparse: n.flags & flags::REPARSE != 0,
            is_system: n.flags & flags::SYSTEM != 0,
        },
        parent_frn: tree.node(parent).frn,
        name: tree.name(r.node).to_string(),
        bytes: n.alloc,
    })
}

#[cfg(test)]
mod tests {
    use acervo_classify::Classifier;
    use acervo_index::usn::Change;
    use acervo_index::{FileName, ParsedRecord, Tree, TreeBuilder};

    use super::*;

    fn rec(no: u64, parent: u64, name: &str, dir: bool, alloc: u64) -> ParsedRecord {
        ParsedRecord {
            record_no: no,
            in_use: true,
            is_dir: dir,
            names: vec![FileName { parent, name: name.into(), namespace: 1 }],
            size: alloc,
            alloc,
            ..Default::default()
        }
    }

    fn tree_with_file(frn: u64, name: &str) -> Tree {
        let mut b = TreeBuilder::new('C', 5, 64);
        b.push(rec(5, 5, ".", true, 0));
        b.push(rec(frn, 5, name, false, 4096));
        b.finish(&Classifier::new(vec![]))
    }

    #[test]
    fn stale_item_ref_is_rejected() {
        let mut tree = tree_with_file(70, "a.bin");
        let n = tree.find(r"C:\a.bin").unwrap();
        let r = ItemRef { drive: 'C', node: n, frn: 70, path: r"C:\a.bin".into() };
        let ok = resolve(&tree, &r).unwrap();
        assert_eq!(ok.facts.path.to_string_lossy(), r"C:\a.bin");
        assert_eq!((ok.parent_frn, ok.name.as_str(), ok.bytes), (5, "a.bin", 4096));
        let created = Change::Created { frn: 71, frn_full: 71, parent: 5, name: "outro.bin".into(), is_dir: false, attributes: 0x20 };
        tree.apply(&[Change::Deleted { frn: 70, parent: 5, name: "a.bin".into() }, created], &|_| None, &Classifier::new(vec![]));
        assert_eq!(tree.find(r"C:\outro.bin"), Some(n), "o nó foi reaproveitado");
        assert!(resolve(&tree, &r).is_none(), "mesmo nó, outro arquivo: deve recusar");
        assert!(resolve(&tree, &ItemRef { drive: 'C', node: 9999, frn: 70, path: r"C:\a.bin".into() }).is_none());
    }

    #[test]
    fn open_folder_must_still_be_the_same_path() {
        let mut b = TreeBuilder::new('C', 5, 64);
        b.push(rec(5, 5, ".", true, 0));
        b.push(rec(80, 5, "Fotos", true, 0));
        let mut tree = b.finish(&Classifier::new(vec![]));
        let n = tree.find(r"C:\Fotos").unwrap();
        assert!(folder_matches(&tree, n, r"C:\fotos"));
        let created = Change::Created { frn: 80, frn_full: 80 | (3 << 48), parent: 5, name: "Jogos".into(), is_dir: true, attributes: 0x10 };
        tree.apply(&[Change::Deleted { frn: 80, parent: 5, name: "Fotos".into() }, created], &|_| None, &Classifier::new(vec![]));
        assert_eq!(tree.find(r"C:\Jogos"), Some(n), "o nó foi reaproveitado por outra pasta");
        assert!(!folder_matches(&tree, n, r"C:\Fotos"));
        assert!(!folder_matches(&tree, 9999, r"C:\Fotos"));
    }

    #[test]
    fn same_record_reused_for_another_path_is_rejected() {
        // O NTFS reusa o registro (mesmo frn) e a árvore reusa o nó: só o caminho mostrado denuncia a troca.
        let mut tree = tree_with_file(70, "foto.jpg");
        let n = tree.find(r"C:\foto.jpg").unwrap();
        let shown = ItemRef { drive: 'C', node: n, frn: 70, path: r"C:\foto.jpg".into() };
        let created = Change::Created { frn: 70, frn_full: 70 | (2 << 48), parent: 5, name: "outro.doc".into(), is_dir: false, attributes: 0x20 };
        tree.apply(&[Change::Deleted { frn: 70, parent: 5, name: "foto.jpg".into() }, created], &|_| None, &Classifier::new(vec![]));
        assert_eq!(tree.find(r"C:\outro.doc"), Some(n));
        assert!(resolve(&tree, &shown).is_none(), "mesmo nó e frn, outro caminho: deve recusar");
        // Maiúsculas diferentes continuam sendo o mesmo item.
        let upper = ItemRef { path: r"C:\OUTRO.DOC".into(), ..shown.clone() };
        assert!(resolve(&tree, &upper).is_some());
    }
}
