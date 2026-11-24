use std::collections::HashMap;

use crate::{category_for_extension, Category};

/// Força de uma regra de caminho.
///
/// `Strong` vale para tudo o que está dentro; `Weak` cede para a extensão
/// de mídia (ex.: um vídeo dentro de Documentos conta como Mídia).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strength {
    Strong,
    Weak,
}

/// Regra que atribui uma categoria a um caminho absoluto e tudo abaixo dele.
#[derive(Clone, Debug)]
pub struct PathRule {
    pub path: String,
    pub category: Category,
    pub strength: Strength,
}

#[derive(Default)]
struct TrieNode {
    children: HashMap<String, usize>,
    rule: Option<(Category, Strength)>,
}

/// Contexto de uma pasta durante a descida pela árvore.
#[derive(Clone, Debug)]
pub struct DirCtx {
    inherited: Option<(Category, Strength)>,
    trie: Option<usize>,
    depth: u16,
}

/// Classificador: trie de caminhos + regras por nome + extensões.
pub struct Classifier {
    nodes: Vec<TrieNode>,
}

const DEV_NAMES: &[&str] = &[
    "node_modules",
    ".venv",
    "venv",
    ".gradle",
    ".m2",
    ".cargo",
    ".rustup",
    ".npm",
    "pnpm-store",
    ".nuget",
];

const CACHE_NAMES: &[&str] = &[
    "cache",
    "code cache",
    "gpucache",
    "shadercache",
    "cache2",
    "dxcache",
    "glcache",
];

fn components(path: &str) -> impl Iterator<Item = String> + '_ {
    path.split(['\\', '/'])
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
}

fn extension(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) if i > 0 => &name[i + 1..],
        _ => "",
    }
}

impl Classifier {
    pub fn new(rules: Vec<PathRule>) -> Classifier {
        let mut c = Classifier {
            nodes: vec![TrieNode::default()],
        };
        for rule in rules {
            let mut cur = 0;
            for comp in components(&rule.path) {
                cur = match c.nodes[cur].children.get(&comp) {
                    Some(&n) => n,
                    None => {
                        let n = c.nodes.len();
                        c.nodes.push(TrieNode::default());
                        c.nodes[cur].children.insert(comp, n);
                        n
                    }
                };
            }
            c.nodes[cur].rule = Some((rule.category, rule.strength));
        }
        c
    }

    fn child(&self, trie: Option<usize>, lname: &str) -> Option<usize> {
        trie.and_then(|t| self.nodes[t].children.get(lname).copied())
    }

    /// Contexto da raiz de um disco (ex.: `C:\`).
    pub fn root(&self, drive: char) -> DirCtx {
        let key = format!("{}:", drive.to_ascii_lowercase());
        let trie = self.child(Some(0), &key);
        DirCtx {
            inherited: trie.and_then(|t| self.nodes[t].rule),
            trie,
            depth: 0,
        }
    }

    /// Contexto de uma subpasta. `has_child` e `has_sibling` respondem se a
    /// pasta (ou a pasta-mãe) tem um item com aquele nome, sem diferenciar
    /// maiúsculas.
    pub fn enter_dir(
        &self,
        parent: &DirCtx,
        name: &str,
        has_child: &dyn Fn(&str) -> bool,
        has_sibling: &dyn Fn(&str) -> bool,
    ) -> DirCtx {
        let lname = name.to_lowercase();
        let trie = self.child(parent.trie, &lname);
        let path_rule = trie.and_then(|t| self.nodes[t].rule);
        let inherited = match path_rule {
            Some(rule) => Some(rule),
            None => self.name_rule(parent, &lname, has_child, has_sibling),
        };
        DirCtx {
            inherited,
            trie,
            depth: parent.depth.saturating_add(1),
        }
    }

    fn name_rule(
        &self,
        parent: &DirCtx,
        lname: &str,
        has_child: &dyn Fn(&str) -> bool,
        has_sibling: &dyn Fn(&str) -> bool,
    ) -> Option<(Category, Strength)> {
        let inherited_cat = parent.inherited.map(|(c, _)| c);
        if matches!(inherited_cat, Some(Category::Sistema) | Some(Category::Cache)) {
            return parent.inherited;
        }
        let strong = |c| Some((c, Strength::Strong));
        if parent.depth == 0 && lname.starts_with('$') && lname != "$recycle.bin" {
            return strong(Category::Sistema);
        }
        if DEV_NAMES.contains(&lname)
            || (lname == "target" && has_sibling("Cargo.toml"))
            || has_child(".git")
        {
            return strong(Category::Desenvolvimento);
        }
        if inherited_cat == Some(Category::DadosApps) && CACHE_NAMES.contains(&lname) {
            return strong(Category::Cache);
        }
        parent.inherited
    }

    /// Categoria de uma pasta (a herdada, ou Outros).
    pub fn dir_category(&self, ctx: &DirCtx) -> Category {
        ctx.inherited.map(|(c, _)| c).unwrap_or(Category::Outros)
    }

    /// Categoria de um arquivo dentro da pasta `parent`.
    pub fn file_category(&self, parent: &DirCtx, name: &str) -> Category {
        let lname = name.to_lowercase();
        if let Some((c, _)) = self
            .child(parent.trie, &lname)
            .and_then(|t| self.nodes[t].rule)
        {
            return c;
        }
        if parent.depth == 0 && lname.starts_with('$') {
            return Category::Sistema;
        }
        let by_ext = category_for_extension(extension(&lname));
        match parent.inherited {
            Some((c, Strength::Strong)) => c,
            Some((c, Strength::Weak)) => match by_ext {
                Some(Category::Midia) => Category::Midia,
                _ => c,
            },
            None => by_ext.unwrap_or(Category::Outros),
        }
    }
}
