//! Analisa um disco (ou carrega o snapshot), reaplica o journal e acompanha as
//! mudanças ao vivo, imprimindo cada total que mudou.
//!
//! Uso (terminal de administrador):
//! `cargo run -p acervo-store --release --example watch -- C <pasta-do-snapshot> [--for 60]`

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use acervo_classify::{discover, Category, Classifier, CATEGORY_COUNT};
use acervo_index::usn::{Coalescer, Journal};
use acervo_index::{scan_ntfs, Tree};
use acervo_store::{load, plan_start, save, snapshot_path, SnapshotMeta, StartPlan, FORMAT_VERSION};

fn gib(n: u64) -> f64 {
    n as f64 / (1u64 << 30) as f64
}

fn epoch_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
}

fn pump(j: &Journal, id: u64, tree: &mut Tree, co: &mut Coalescer, c: &Classifier, from: i64, wait: bool) -> (i64, bool, usize) {
    let (next, recs) = j.read(id, from, wait).expect("leitura do journal");
    for r in &recs {
        co.push(r);
    }
    let changes = co.drain();
    let rep = tree.apply(&changes, &|f| j.stat(f), c);
    (next, rep.changed, recs.len())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let drive = args.get(1).and_then(|s| s.chars().next()).unwrap_or('C').to_ascii_uppercase();
    let dir = PathBuf::from(args.get(2).cloned().unwrap_or_else(|| ".".into()));
    let secs: u64 = args.iter().position(|a| a == "--for").and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(60);

    let known = discover();
    let rules = known.fingerprint();
    let classifier = Classifier::standard(&known);
    let t0 = Instant::now();
    let j = Journal::open(drive).expect("abrir o volume (precisa de administrador)");
    let info = j.query_or_create().expect("journal");
    let serial = j.volume_serial();
    let path = snapshot_path(&dir, serial);
    let loaded = load(&path).ok();
    let plan = plan_start(loaded.as_ref().map(|(m, _)| m), &info, serial, rules);
    let (mut tree, mut from, origem) = match (plan, loaded) {
        (StartPlan::Replay { from, reclassify }, Some((_, mut t))) => {
            if reclassify {
                t.reclassify_all(&classifier);
            }
            (t, from, "snapshot")
        }
        _ => (scan_ntfs(drive, &classifier, &|_| {}).expect("análise"), info.next_usn, "análise completa"),
    };

    let mut co = Coalescer::default();
    loop {
        let (next, _, n) = pump(&j, info.journal_id, &mut tree, &mut co, &classifier, from, false);
        from = next;
        if n == 0 {
            break;
        }
    }
    println!("pronto em {:.2} s ({origem}), {} nós", t0.elapsed().as_secs_f64(), tree.live_len());
    let mut last: [u64; CATEGORY_COUNT] = *tree.totals(tree.root);
    for c in Category::ALL {
        println!("  {:<22} {:>9.2} GB", c.label(), gib(last[c as usize]));
    }

    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        let (next, changed, _) = pump(&j, info.journal_id, &mut tree, &mut co, &classifier, from, true);
        from = next;
        if changed {
            let now = *tree.totals(tree.root);
            for c in Category::ALL {
                let (a, b) = (last[c as usize], now[c as usize]);
                if a != b {
                    println!("{} {:<22} {:>9.3} GB ({:+.3} GB)", epoch_ms(), c.label(), gib(b), (b as f64 - a as f64) / (1u64 << 30) as f64);
                }
            }
            last = now;
        }
    }

    let meta = SnapshotMeta {
        version: FORMAT_VERSION,
        volume_serial: serial,
        journal_id: info.journal_id,
        next_usn: from,
        rules_hash: rules,
        saved_at: (epoch_ms() / 1000) as i64,
    };
    let ts = Instant::now();
    save(&path, &meta, &tree).expect("salvar snapshot");
    println!("snapshot salvo em {:.2} s: {}", ts.elapsed().as_secs_f64(), path.display());
}
