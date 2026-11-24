use serde::{Deserialize, Serialize};

/// Número de categorias exibidas na barra.
pub const CATEGORY_COUNT: usize = 10;

/// As 10 categorias, com índice `u8` fixo.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Apps = 0,
    Jogos = 1,
    /// Chave "dev" no JSON, a mesma de `key()` e da interface.
    #[serde(rename = "dev")]
    Desenvolvimento = 2,
    Midia = 3,
    Documentos = 4,
    Downloads = 5,
    Cache = 6,
    DadosApps = 7,
    Sistema = 8,
    Outros = 9,
}

impl Category {
    pub const ALL: [Category; CATEGORY_COUNT] = [
        Category::Apps,
        Category::Jogos,
        Category::Desenvolvimento,
        Category::Midia,
        Category::Documentos,
        Category::Downloads,
        Category::Cache,
        Category::DadosApps,
        Category::Sistema,
        Category::Outros,
    ];

    pub fn from_u8(v: u8) -> Option<Category> {
        Self::ALL.get(v as usize).copied()
    }

    /// Chave estável usada no contrato com a interface.
    pub fn key(self) -> &'static str {
        match self {
            Category::Apps => "apps",
            Category::Jogos => "jogos",
            Category::Desenvolvimento => "dev",
            Category::Midia => "midia",
            Category::Documentos => "documentos",
            Category::Downloads => "downloads",
            Category::Cache => "cache",
            Category::DadosApps => "dados_apps",
            Category::Sistema => "sistema",
            Category::Outros => "outros",
        }
    }

    /// Rótulo exibido ao usuário.
    pub fn label(self) -> &'static str {
        match self {
            Category::Apps => "Apps",
            Category::Jogos => "Jogos",
            Category::Desenvolvimento => "Desenvolvimento",
            Category::Midia => "Mídia",
            Category::Documentos => "Documentos",
            Category::Downloads => "Downloads",
            Category::Cache => "Cache e temporários",
            Category::DadosApps => "Dados de apps",
            Category::Sistema => "Sistema",
            Category::Outros => "Outros",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_uses_the_same_keys() {
        for c in Category::ALL {
            assert_eq!(serde_json::to_string(&c).unwrap(), format!("\"{}\"", c.key()));
            assert_eq!(serde_json::from_str::<Category>(&format!("\"{}\"", c.key())).unwrap(), c);
        }
    }

    #[test]
    fn keys_and_labels() {
        assert_eq!(Category::Cache.key(), "cache");
        assert_eq!(Category::Cache.label(), "Cache e temporários");
        assert_eq!(Category::DadosApps.label(), "Dados de apps");
        assert_eq!(Category::from_u8(8), Some(Category::Sistema));
        assert_eq!(Category::from_u8(10), None);
        assert_eq!(Category::ALL.len(), CATEGORY_COUNT);
    }
}
