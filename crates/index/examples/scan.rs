//! Lê um disco e imprime o total por categoria.
//!
//! Uso (terminal de administrador): `cargo run -p acervo-index --release --example scan -- C`

use acervo_classify::{discover, Category, Classifier};
use acervo_index::{scan_ntfs, scan_walk};

fn gib(n: u64) -> f64 {
    n as f64 / (1u64 << 30) as f64
}

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| "C".into());
    let drive = arg.chars().next().unwrap().to_ascii_uppercase();
    let walk = std::env::args().any(|a| a == "--walk");
    let classifier = Classifier::standard(&discover());
    let t0 = std::time::Instant::now();
    let result = if walk {
        scan_walk(std::path::Path::new(&format!("{drive}:\\")), drive, &classifier, &|_| {})
    } else {
        scan_ntfs(drive, &classifier, &|_| {})
    };
    let tree = match result {
        Ok(t) => t,
        Err(e) => {
            eprintln!("falhou: {e}");
            std::process::exit(1);
        }
    };
    println!("{drive}: {} nós em {:.1}s, {} registros descartados", tree.len(), t0.elapsed().as_secs_f64(), tree.skipped_records);
    let totals = tree.totals(tree.root);
    for c in Category::ALL {
        println!("  {:<22} {:>9.2} GB", c.label(), gib(totals[c as usize]));
    }
    println!("  {:<22} {:>9.2} GB", "TOTAL", gib(totals.iter().sum()));

    // As 15 maiores pastas de primeiro nível, para conferir com o Explorer.
    let mut top: Vec<(u64, String)> = tree.children(tree.root).map(|c| (tree.node(c).alloc, tree.path(c))).collect();
    top.sort_by(|a, b| b.0.cmp(&a.0));
    for (bytes, path) in top.into_iter().take(15) {
        println!("    {:>9.2} GB  {path}", gib(bytes));
    }
}
