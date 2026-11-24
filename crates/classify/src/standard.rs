use std::path::Path;

use crate::{Category, Classifier, KnownPaths, PathRule, Strength};

/// Versão das regras padrão. Incrementar sempre que `Classifier::standard` mudar,
/// para que snapshots antigos sejam reclassificados ao abrir.
pub const RULES_VERSION: u32 = 1;

fn rule(path: &Path, category: Category, strength: Strength) -> PathRule {
    PathRule {
        path: path.to_string_lossy().into_owned(),
        category,
        strength,
    }
}

impl Classifier {
    /// Regras padrão da spec (seção 6) para os caminhos desta máquina.
    pub fn standard(known: &KnownPaths) -> Classifier {
        use Category::*;
        use Strength::*;
        let mut rules = Vec::new();
        let mut add = |p: &Path, c: Category, s: Strength| rules.push(rule(p, c, s));

        for &d in &known.drives {
            let root = format!("{d}:\\");
            let root = Path::new(&root);
            for name in ["System Volume Information", "Recovery", "pagefile.sys", "hiberfil.sys", "swapfile.sys"] {
                add(&root.join(name), Sistema, Strong);
            }
            add(&root.join("$Recycle.Bin"), Cache, Strong);
            add(&root.join("XboxGames"), Jogos, Strong);
        }

        let win = &known.windows_dir;
        add(win, Sistema, Strong);
        add(&win.join("Temp"), Cache, Strong);
        add(&win.join(r"SoftwareDistribution\Download"), Cache, Strong);
        for pf in &known.program_files {
            add(pf, Apps, Strong);
        }
        add(&known.program_data, DadosApps, Strong);
        add(&known.program_data.join(r"Microsoft\Windows\WER"), Cache, Strong);
        for lib in &known.game_libraries {
            add(lib, Jogos, Strong);
        }

        for p in &known.profiles {
            add(&p.appdata_roaming, DadosApps, Strong);
            add(&p.appdata_local, DadosApps, Strong);
            add(&p.appdata_locallow, DadosApps, Strong);
            add(&p.appdata_local.join("Programs"), Apps, Strong);
            add(&p.temp, Cache, Strong);
            for sub in [r"Microsoft\Windows\INetCache", "CrashDumps", "D3DSCache"] {
                add(&p.appdata_local.join(sub), Cache, Strong);
            }
            add(&p.downloads, Downloads, Strong);
            add(&p.documents, Documentos, Weak);
        }

        Classifier::new(rules)
    }
}
