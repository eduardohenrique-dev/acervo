use acervo_classify::discover;
use std::path::PathBuf;

#[test]
fn fingerprint_changes_with_libraries() {
    let mut k = discover();
    let a = k.fingerprint();
    assert_eq!(a, discover().fingerprint());
    k.game_libraries.push(PathBuf::from(r"E:\Jogos"));
    assert_ne!(a, k.fingerprint());
}

#[test]
fn discover_on_this_machine() {
    let k = discover();
    assert!(k.windows_dir.join("System32").is_dir());
    assert!(k.drives.contains(&'C'));
    assert!(!k.profiles.is_empty());
    assert!(k.profiles.iter().any(|p| p.downloads.ends_with("Downloads")));
}
