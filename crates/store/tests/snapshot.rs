use acervo_classify::{Category, Classifier, PathRule, Strength};
use acervo_index::{FileName, ParsedRecord, Tree, TreeBuilder};
use acervo_store::{load, save, snapshot_path, SnapshotMeta, StoreError, FORMAT_VERSION};

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

fn sample_tree() -> Tree {
    let mut b = TreeBuilder::new('C', 5, 64);
    b.push(rec(5, 5, ".", true, 0));
    b.push(rec(30, 5, "Windows", true, 0));
    b.push(rec(31, 30, "k.dll", false, 4096));
    b.push(rec(32, 5, "Fotos", true, 0));
    b.push(rec(33, 32, "a.jpg", false, 8192));
    b.finish(&Classifier::new(vec![PathRule { path: r"C:\Windows".into(), category: Category::Sistema, strength: Strength::Strong }]))
}

fn meta() -> SnapshotMeta {
    SnapshotMeta { version: FORMAT_VERSION, volume_serial: 0xABCD, journal_id: 7, next_usn: 9000, rules_hash: 42, saved_at: 1 }
}

#[test]
fn roundtrip() {
    let d = tempfile::tempdir().unwrap();
    let tree = sample_tree();
    let p = snapshot_path(d.path(), 0xABCD);
    assert!(p.ends_with("index-000000000000abcd.bin"));
    save(&p, &meta(), &tree).unwrap();
    let (m2, t2) = load(&p).unwrap();
    assert_eq!(m2, meta());
    assert_eq!(t2.totals(t2.root), tree.totals(tree.root));
    assert_eq!(t2.path(t2.find(r"C:\Fotos\a.jpg").unwrap()), r"C:\Fotos\a.jpg");
    assert_eq!(t2.node_for_frn(33), tree.node_for_frn(33));
}

#[test]
fn corrupt_snapshot_is_error() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("index-x.bin");
    let mut bad = b"ACVO".to_vec();
    bad.extend(FORMAT_VERSION.to_le_bytes());
    bad.extend(b"lixo");
    std::fs::write(&p, bad).unwrap();
    assert!(matches!(load(&p), Err(StoreError::Corrupt(_))));
    std::fs::write(&p, b"XXXX\x01\x00\x00\x00").unwrap();
    assert!(matches!(load(&p), Err(StoreError::Corrupt(_))));
    assert!(matches!(load(&d.path().join("nao-existe.bin")), Err(StoreError::Missing)));
}

#[test]
fn other_version_is_error() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("v.bin");
    std::fs::write(&p, b"ACVO\x09\x00\x00\x00").unwrap();
    assert!(matches!(load(&p), Err(StoreError::Version(9))));
}

#[test]
fn save_is_atomic() {
    let d = tempfile::tempdir().unwrap();
    let p = snapshot_path(d.path(), 1);
    save(&p, &meta(), &sample_tree()).unwrap();
    let tmp = p.with_extension("bin.tmp");
    assert!(!tmp.exists());
    std::fs::write(&tmp, b"pela metade").unwrap(); // gravação interrompida anterior
    save(&p, &meta(), &sample_tree()).unwrap();
    assert!(load(&p).is_ok());
    assert!(!tmp.exists());
}

#[test]
fn absurd_length_prefix_is_corrupt_not_abort() {
    // Cabeçalho válido, meta válida e um vetor de nós que diz ter 2^48 elementos.
    let mut raw = Vec::new();
    raw.extend(FORMAT_VERSION.to_le_bytes());
    raw.extend(0xABCDu64.to_le_bytes());
    raw.extend(7u64.to_le_bytes());
    raw.extend(9000i64.to_le_bytes());
    raw.extend(42u64.to_le_bytes());
    raw.extend(1i64.to_le_bytes());
    raw.push(b'C'); // drive
    raw.extend(0u32.to_le_bytes()); // root
    raw.extend(0u64.to_le_bytes()); // skipped
    raw.extend((1u64 << 48).to_le_bytes()); // nodes.len()
    let mut file = b"ACVO".to_vec();
    file.extend(FORMAT_VERSION.to_le_bytes());
    file.extend(zstd::encode_all(&raw[..], 3).unwrap());
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("huge.bin");
    std::fs::write(&p, file).unwrap();
    assert!(matches!(load(&p), Err(StoreError::Corrupt(_))));
}
