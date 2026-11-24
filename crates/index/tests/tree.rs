mod support;

use acervo_classify::Category;
use acervo_index::{flags, FileName, ParsedRecord};
use support::*;

#[test]
fn builds_paths_and_totals() {
    let t = build(vec![
        root(5),
        dir(30, 5, "Windows"),
        file(31, 30, "k.dll", 4096),
        dir(32, 5, "Fotos"),
        file(33, 32, "a.jpg", 8192),
        file(34, 5, "x.bin", 100),
    ]);
    assert_eq!(t.path(t.find(r"C:\Fotos\a.jpg").unwrap()), r"C:\Fotos\a.jpg");
    let tot = t.totals(t.root);
    assert_eq!(tot[Category::Sistema as usize], 4096);
    assert_eq!(tot[Category::Midia as usize], 8192);
    assert_eq!(tot[Category::Outros as usize], 100);
    assert_eq!(t.path(t.root), r"C:\");
}

#[test]
fn hardlink_counted_once() {
    let mut r = file(40, 5, "a.dll", 4096);
    r.names.push(FileName { parent: 30, name: "b.dll".into(), namespace: 1 });
    let t = build(vec![root(5), dir(30, 5, "Windows"), r]);
    let sum: u64 = t.totals(t.root).iter().sum();
    assert_eq!(sum, 4096);
    let link = t.find(r"C:\Windows\b.dll").unwrap();
    assert!(t.node(link).flags & flags::LINK != 0);
    assert_eq!(t.node(link).alloc, 0);
}

#[test]
fn extension_record_merged() {
    let base = ParsedRecord { record_no: 50, in_use: true, names: vec![fname(5, "grande.iso")], ..Default::default() };
    let ext = ParsedRecord {
        record_no: 51,
        base_record: 50,
        in_use: true,
        size: 1 << 30,
        alloc: 1 << 30,
        ..Default::default()
    };
    let t = build(vec![root(5), base, ext]);
    assert_eq!(t.node(t.find(r"C:\grande.iso").unwrap()).alloc, 1 << 30);
}

#[test]
fn orphan_attaches_to_root() {
    let t = build(vec![root(5), file(60, 999, "perdido.txt", 4096)]);
    let n = t.find(r"C:\perdido.txt").unwrap();
    assert!(t.node(n).flags & flags::ORPHAN != 0);
    assert_eq!(t.totals(t.root).iter().sum::<u64>(), 4096);
}

#[test]
fn parent_cycle_terminates() {
    let t = build(vec![root(5), dir(70, 71, "a"), dir(71, 70, "b")]);
    assert!(t.find(r"C:\a").is_some() || t.find(r"C:\b").is_some());
    for i in 0..t.len() as u32 {
        let _ = t.path(i);
    }
}

#[test]
fn dir_own_bytes_counted() {
    let mut d = dir(80, 5, "Windows");
    d.index_alloc = 8192;
    let t = build(vec![root(5), d]);
    assert_eq!(t.totals(t.root)[Category::Sistema as usize], 8192);
}

#[test]
fn dev_folder_detected_by_git_child() {
    let t = build(vec![root(5), dir(90, 5, "site"), dir(91, 90, ".git"), file(92, 90, "index.html", 4096)]);
    assert_eq!(t.category(t.find(r"C:\site\index.html").unwrap()), Category::Desenvolvimento);
}
