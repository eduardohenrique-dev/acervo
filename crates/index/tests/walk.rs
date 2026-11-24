use acervo_classify::{Category, Classifier};
use acervo_index::scan_walk_with_cluster;

#[test]
fn walks_temp_tree() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("Fotos")).unwrap();
    std::fs::write(d.path().join("Fotos/a.jpg"), vec![0u8; 5000]).unwrap();
    std::fs::write(d.path().join("b.bin"), vec![0u8; 10]).unwrap();
    let t = scan_walk_with_cluster(d.path(), 'C', &Classifier::new(vec![]), &|_| {}, 4096).unwrap();
    let tot = t.totals(t.root);
    assert_eq!(tot[Category::Midia as usize], 8192);
    assert_eq!(tot[Category::Outros as usize], 4096);
    assert!(t.find(r"C:\Fotos\a.jpg").is_some());
}

#[test]
fn does_not_follow_junctions() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("real")).unwrap();
    std::fs::write(d.path().join("real/x.bin"), vec![0u8; 4096]).unwrap();
    let ok = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(d.path().join("link"))
        .arg(d.path().join("real"))
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "mklink /J falhou");
    let t = scan_walk_with_cluster(d.path(), 'C', &Classifier::new(vec![]), &|_| {}, 4096).unwrap();
    assert_eq!(t.totals(t.root).iter().sum::<u64>(), 4096);
    assert!(t.find(r"C:\link").is_some());
}
