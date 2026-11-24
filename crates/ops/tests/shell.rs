use std::path::PathBuf;

use acervo_ops::{run_operation, OpKind};

#[test]
fn copy_move_rename_delete() {
    let d = tempfile::tempdir().unwrap();
    let a = d.path().join("a.txt");
    std::fs::write(&a, b"oi").unwrap();
    let dest = d.path().join("dest");
    std::fs::create_dir(&dest).unwrap();
    let o = run_operation(&OpKind::Copy { dest: dest.clone() }, &[a.clone()], &|_| {});
    assert!(o.failed.is_empty() && dest.join("a.txt").exists() && a.exists(), "copiar");
    let o = run_operation(&OpKind::Rename { new_name: "b.txt".into() }, &[a.clone()], &|_| {});
    assert!(o.failed.is_empty() && d.path().join("b.txt").exists() && !a.exists(), "renomear");
    let o = run_operation(&OpKind::Move { dest: dest.clone() }, &[d.path().join("b.txt")], &|_| {});
    assert!(o.failed.is_empty() && dest.join("b.txt").exists(), "mover");
    let o = run_operation(&OpKind::DeletePermanent, &[dest.clone()], &|_| {});
    assert!(o.failed.is_empty() && !dest.exists(), "apagar");
    assert_eq!(o.succeeded.len(), 1);
}

#[test]
fn locked_file_reports_failure() {
    let d = tempfile::tempdir().unwrap();
    let (a, b) = (d.path().join("preso.bin"), d.path().join("livre.bin"));
    std::fs::write(&a, b"x").unwrap();
    std::fs::write(&b, b"y").unwrap();
    use std::os::windows::fs::OpenOptionsExt;
    let lock = std::fs::OpenOptions::new().read(true).share_mode(0).open(&a).unwrap(); // sem compartilhamento
    let o = run_operation(&OpKind::DeletePermanent, &[a.clone(), b.clone()], &|_| {});
    assert_eq!(o.failed.len(), 1, "falhas: {:?}", o.failed.iter().map(|f| &f.path).collect::<Vec<_>>());
    assert!(o.failed[0].path.ends_with("preso.bin") && !o.failed[0].message.is_empty());
    assert!(!b.exists(), "o item livre deve ser apagado mesmo com o outro falhando");
    drop(lock);
}

#[test]
fn many_items_in_one_operation() {
    let d = tempfile::tempdir().unwrap();
    let items: Vec<PathBuf> = (0..2000)
        .map(|i| {
            let p = d.path().join(format!("f{i}.tmp"));
            std::fs::write(&p, b"z").unwrap();
            p
        })
        .collect();
    let calls = std::cell::Cell::new(0);
    let o = run_operation(&OpKind::DeletePermanent, &items, &|_| calls.set(calls.get() + 1));
    assert_eq!(o.succeeded.len(), 2000);
    assert!(calls.get() > 0);
}

#[test]
fn long_and_unicode_paths() {
    let d = tempfile::tempdir().unwrap();
    let mut deep = d.path().to_path_buf();
    for i in 0..12 {
        deep.push(format!("pasta-com-nome-comprido-{i:02}-ação"));
    }
    std::fs::create_dir_all(&deep).unwrap();
    let f = deep.join("arquivo 🎮 ção.bin");
    std::fs::write(&f, b"1").unwrap();
    assert!(f.as_os_str().len() > 260);
    let o = run_operation(&OpKind::DeletePermanent, &[f.clone()], &|_| {});
    assert!(o.failed.is_empty(), "{:?}", o.failed.iter().map(|x| &x.message).collect::<Vec<_>>());
    assert!(!f.exists());
}

#[test]
fn rename_needs_single_item() {
    let d = tempfile::tempdir().unwrap();
    let (a, b) = (d.path().join("a"), d.path().join("b"));
    std::fs::write(&a, b"1").unwrap();
    std::fs::write(&b, b"2").unwrap();
    let o = run_operation(&OpKind::Rename { new_name: "c".into() }, &[a.clone(), b.clone()], &|_| {});
    assert_eq!(o.failed.len(), 2);
    assert!(a.exists() && b.exists());
}

#[test]
#[ignore = "manda um arquivo de teste para a Lixeira do usuário"]
fn recycle_moves_to_bin() {
    let d = tempfile::tempdir().unwrap();
    let a = d.path().join("acervo-teste-lixeira.txt");
    std::fs::write(&a, b"x").unwrap();
    let o = run_operation(&OpKind::Recycle, &[a.clone()], &|_| {});
    assert!(o.failed.is_empty() && !a.exists());
}

#[test]
fn rename_to_existing_name_fails_and_touches_nothing() {
    let d = tempfile::tempdir().unwrap();
    let (a, b) = (d.path().join("a.txt"), d.path().join("b.txt"));
    std::fs::write(&a, b"A").unwrap();
    std::fs::write(&b, b"B").unwrap();
    let o = run_operation(&OpKind::Rename { new_name: "b.txt".into() }, &[a.clone()], &|_| {});
    assert_eq!(o.failed.len(), 1, "{o:?}");
    assert!(o.succeeded.is_empty());
    assert_eq!(std::fs::read(&a).unwrap(), b"A");
    assert_eq!(std::fs::read(&b).unwrap(), b"B");
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 2, "não pode criar \"b (2).txt\"");
}

#[test]
fn rename_changing_only_case_works() {
    let d = tempfile::tempdir().unwrap();
    let a = d.path().join("nota.txt");
    std::fs::write(&a, b"x").unwrap();
    let o = run_operation(&OpKind::Rename { new_name: "Nota.txt".into() }, &[a.clone()], &|_| {});
    assert!(o.failed.is_empty(), "{o:?}");
    let names: Vec<String> = std::fs::read_dir(d.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names, vec!["Nota.txt"]);
}
