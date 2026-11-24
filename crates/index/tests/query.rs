mod support;

use std::path::PathBuf;

use acervo_classify::{Category, Classifier, KnownPaths};
use acervo_index::{summary, system_breakdown, DiskSpace, ParsedRecord, ScanMode, Tree};
use support::*;

fn standard() -> Classifier {
    Classifier::standard(&KnownPaths {
        windows_dir: PathBuf::from(r"C:\Windows"),
        program_files: vec![],
        program_data: PathBuf::from(r"C:\ProgramData"),
        profiles: vec![],
        game_libraries: vec![],
        drives: vec!['C'],
    })
}

fn tree(records: Vec<ParsedRecord>) -> Tree {
    build_with(records, &standard())
}

#[test]
fn summary_numbers() {
    let t = tree(vec![root(5), dir(30, 5, "Windows"), file(31, 30, "k.dll", 4096), file(32, 5, "a.jpg", 8192)]);
    let s = summary(&t, DiskSpace { total: 100_000, free: 80_000 }, ScanMode::Mft);
    assert_eq!(s.used_bytes, 20_000);
    assert_eq!(s.cataloged_bytes, 12_288);
    assert_eq!(s.by_category[Category::Midia as usize], 8192);
    assert_eq!(s.drive, 'C');
}

#[test]
fn breakdown_groups() {
    let t = tree(vec![
        root(5),
        dir(30, 5, "Windows"),
        file(31, 30, "k.dll", 4096),
        file(32, 5, "pagefile.sys", 16384),
        file(33, 5, "hiberfil.sys", 8192),
        file(34, 5, "$MFT", 2048),
    ]);
    let b = system_breakdown(&t);
    let keys: Vec<_> = b.iter().map(|i| (i.key, i.bytes)).collect();
    assert_eq!(
        keys,
        vec![("memoria_virtual", 16384), ("hibernacao", 8192), ("windows", 4096), ("ntfs", 2048)]
    );
}

#[test]
fn breakdown_excludes_cache_inside_windows() {
    let t = tree(vec![
        root(5),
        dir(30, 5, "Windows"),
        file(31, 30, "k.dll", 4096),
        dir(32, 30, "Temp"),
        file(33, 32, "lixo.tmp", 65536),
    ]);
    let b = system_breakdown(&t);
    assert_eq!(b.iter().find(|i| i.key == "windows").unwrap().bytes, 4096);
}
