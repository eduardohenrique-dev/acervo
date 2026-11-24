mod support;

use acervo_classify::Category;
use acervo_index::usn::Change;
use acervo_index::flags;
use support::*;

// ---- Task 3: criar, apagar e redimensionar ----

#[test]
fn create_file_updates_totals() {
    let mut t = build(vec![root(5), dir(30, 5, "Fotos")]);
    let st = stats(&[(70, 8192)]);
    let r = t.apply(&[created(70, 30, "a.jpg", false)], &|f| st.get(&(f & MASK)).copied(), &classifier());
    assert!(r.changed);
    assert_eq!(t.totals(t.root)[Category::Midia as usize], 8192);
    assert_eq!(t.path(t.find(r"C:\Fotos\a.jpg").unwrap()), r"C:\Fotos\a.jpg");
    assert_eq!(t.node(t.find(r"C:\Fotos").unwrap()).alloc, 8192);
}

#[test]
fn create_dir_then_file_inside() {
    let mut t = build(vec![root(5), dir(30, 5, "Windows")]);
    let st = stats(&[(71, 4096)]);
    t.apply(
        &[created(70, 30, "Logs", true), created(71, 70, "a.log", false)],
        &|f| st.get(&(f & MASK)).copied(),
        &classifier(),
    );
    assert_eq!(t.category(t.find(r"C:\Windows\Logs\a.log").unwrap()), Category::Sistema);
    assert_eq!(t.totals(t.root)[Category::Sistema as usize], 4096);
}

#[test]
fn delete_dir_removes_subtree_totals() {
    let mut t = build(vec![root(5), dir(30, 5, "Fotos"), file(31, 30, "a.jpg", 4096), file(32, 30, "b.jpg", 4096)]);
    t.apply(&[deleted(30, 5, "Fotos")], &|_| None, &classifier());
    assert_eq!(t.totals(t.root).iter().sum::<u64>(), 0);
    assert!(t.find(r"C:\Fotos").is_none());
    assert_eq!(t.live_len(), 1);
}

#[test]
fn modified_applies_delta() {
    let mut t = build(vec![root(5), file(40, 5, "x.bin", 4096)]);
    let st = stats(&[(40, 1 << 20)]);
    t.apply(&[modified(40)], &|f| st.get(&(f & MASK)).copied(), &classifier());
    assert_eq!(t.totals(t.root)[Category::Outros as usize], 1 << 20);
    assert_eq!(t.node(t.root).alloc, 1 << 20);
}

#[test]
fn unknown_frn_is_ignored() {
    let mut t = build(vec![root(5)]);
    assert!(!t.apply(&[deleted(999, 5, "x"), modified(998)], &|_| None, &classifier()).changed);
}

#[test]
fn create_existing_in_same_place_is_noop() {
    let mut t = build(vec![root(5), file(40, 5, "x.bin", 4096)]);
    let st = stats(&[(40, 4096)]);
    assert!(!t.apply(&[created(40, 5, "x.bin", false)], &|f| st.get(&(f & MASK)).copied(), &classifier()).changed);
    assert_eq!(t.totals(t.root).iter().sum::<u64>(), 4096);
}

#[test]
fn create_with_unknown_parent_is_orphan() {
    let mut t = build(vec![root(5)]);
    let st = stats(&[(70, 4096)]);
    let r = t.apply(&[created(70, 12345, "perdido.txt", false)], &|f| st.get(&(f & MASK)).copied(), &classifier());
    assert_eq!(r.orphaned, 1);
    assert!(t.node(t.find(r"C:\perdido.txt").unwrap()).flags & flags::ORPHAN != 0);
    assert_eq!(t.totals(t.root).iter().sum::<u64>(), 4096);
}

#[test]
fn freed_slots_are_reused() {
    let mut t = build(vec![root(5), dir(30, 5, "a")]);
    let before = t.len();
    t.apply(&[deleted(30, 5, "a"), created(31, 5, "b", true)], &|_| None, &classifier());
    assert_eq!(t.len(), before);
    assert!(t.find(r"C:\b").is_some());
}

#[test]
fn delete_many_siblings_is_linear() {
    let mut recs = vec![root(5), dir(30, 5, "node_modules")];
    recs.extend((0..50_000u64).map(|i| file(100 + i, 30, &format!("f{i}.js"), 4096)));
    let mut t = build(recs);
    let dels: Vec<Change> = (0..50_000u64).map(|i| deleted(100 + i, 30, &format!("f{i}.js"))).collect();
    let t0 = std::time::Instant::now();
    t.apply(&dels, &|_| None, &classifier());
    assert!(t0.elapsed() < std::time::Duration::from_secs(2), "{:?}", t0.elapsed());
    assert_eq!(t.totals(t.root).iter().sum::<u64>(), 0);
}

// ---- Task 4: mover, links e reclassificar ----

use acervo_classify::{Classifier, PathRule, Strength};
use acervo_index::FileName;

#[test]
fn move_between_categories_moves_bytes() {
    let mut t = build_with(user_tree(), &standard());
    assert_eq!(t.totals(t.root)[Category::Downloads as usize], 8192);
    t.apply(&[moved(70, 30, "v.mp4", 31, "v.mp4")], &|_| None, &standard());
    let tot = t.totals(t.root);
    assert_eq!(tot[Category::Downloads as usize], 0);
    assert_eq!(tot[Category::Midia as usize], 8192);
    assert_eq!(t.node(t.find(r"C:\Users\Ana\Downloads").unwrap()).alloc, 0);
}

#[test]
fn move_dir_reclassifies_subtree() {
    let mut t = build(vec![root(5), dir(30, 5, "proj"), file(31, 30, "a.js", 4096), dir(40, 5, "x"), dir(41, 40, "node_modules")]);
    t.apply(&[moved(30, 5, "proj", 41, "proj")], &|_| None, &classifier());
    assert_eq!(t.category(t.find(r"C:\x\node_modules\proj\a.js").unwrap()), Category::Desenvolvimento);
    assert_eq!(t.totals(t.root)[Category::Desenvolvimento as usize], 4096);
    assert_eq!(t.totals(t.root)[Category::Outros as usize], 0);
}

#[test]
fn rename_in_place_keeps_totals() {
    let mut t = build(vec![root(5), file(70, 5, "a.bin", 4096)]);
    t.apply(&[moved(70, 5, "a.bin", 5, "b.bin")], &|_| None, &classifier());
    assert!(t.find(r"C:\b.bin").is_some() && t.find(r"C:\a.bin").is_none());
    assert_eq!(t.totals(t.root).iter().sum::<u64>(), 4096);
}

#[test]
fn created_for_existing_frn_elsewhere_moves_it() {
    let mut t = build(vec![root(5), dir(30, 5, "Fotos"), file(70, 5, "a.jpg", 4096)]);
    t.apply(&[created(70, 30, "a.jpg", false)], &|_| None, &classifier());
    assert!(t.find(r"C:\Fotos\a.jpg").is_some() && t.find(r"C:\a.jpg").is_none());
    assert_eq!(t.totals(t.root).iter().sum::<u64>(), 4096);
}

#[test]
fn moved_orphan_is_relinked() {
    let mut t = build(vec![root(5), dir(30, 5, "Fotos")]);
    let st = stats(&[(70, 4096)]);
    t.apply(&[created(70, 999, "a.jpg", false)], &|f| st.get(&(f & MASK)).copied(), &classifier());
    t.apply(&[moved(70, 999, "a.jpg", 30, "a.jpg")], &|_| None, &classifier());
    let n = t.find(r"C:\Fotos\a.jpg").unwrap();
    assert_eq!(t.node(n).flags & flags::ORPHAN, 0);
    assert_eq!(t.node(t.find(r"C:\Fotos").unwrap()).alloc, 4096);
}

#[test]
fn deleting_primary_link_promotes_other() {
    let mut r = file(40, 5, "a.dll", 4096);
    r.names.push(FileName { parent: 30, name: "b.dll".into(), namespace: 1 });
    let mut t = build(vec![root(5), dir(30, 5, "Windows"), r]);
    t.apply(&[link_toggled(40, 5, "a.dll")], &|_| None, &classifier());
    assert!(t.find(r"C:\a.dll").is_none());
    let b = t.find(r"C:\Windows\b.dll").unwrap();
    assert_eq!(t.node(b).alloc, 4096);
    assert_eq!(t.node(b).flags & flags::LINK, 0);
    assert_eq!(t.totals(t.root).iter().sum::<u64>(), 4096);
    assert_eq!(t.node_for_frn(40), Some(b));
}

#[test]
fn new_link_is_zero_sized() {
    let mut t = build(vec![root(5), dir(30, 5, "Windows"), file(40, 5, "a.dll", 4096)]);
    t.apply(&[link_toggled(40, 30, "b.dll")], &|_| None, &classifier());
    let b = t.find(r"C:\Windows\b.dll").unwrap();
    assert_ne!(t.node(b).flags & flags::LINK, 0);
    assert_eq!(t.totals(t.root).iter().sum::<u64>(), 4096);
}

#[test]
fn reclassify_all_applies_new_rules() {
    let mut t = build(vec![root(5), dir(30, 5, "Jogos"), file(31, 30, "x.pak", 4096)]);
    assert_eq!(t.totals(t.root)[Category::Outros as usize], 4096);
    let c = Classifier::new(vec![PathRule { path: r"C:\Jogos".into(), category: Category::Jogos, strength: Strength::Strong }]);
    t.reclassify_all(&c);
    assert_eq!(t.totals(t.root)[Category::Jogos as usize], 4096);
    assert_eq!(t.totals(t.root)[Category::Outros as usize], 0);
}

// ---- Revisão final de código ----

#[test]
fn moved_unknown_frn_is_created() {
    let mut t = build(vec![root(5), dir(30, 5, "Fotos")]);
    let st = stats(&[(70, 4096)]);
    let r = t.apply(&[moved(70, 999, "tmp", 30, "a.jpg")], &|f| st.get(&(f & MASK)).copied(), &classifier());
    assert!(r.changed);
    assert!(t.find(r"C:\Fotos\a.jpg").is_some());
    assert_eq!(t.totals(t.root)[Category::Midia as usize], 4096);
}

#[test]
fn orphan_relinked_when_parent_appears() {
    let mut t = build(vec![root(5), dir(30, 5, "Windows")]);
    let st = stats(&[(70, 4096)]);
    t.apply(&[created(70, 999, "k.dll", false)], &|f| st.get(&(f & MASK)).copied(), &classifier());
    assert_eq!(t.totals(t.root)[Category::Outros as usize], 4096);
    t.apply(&[created(999, 30, "Logs", true)], &|_| None, &classifier());
    let n = t.find(r"C:\Windows\Logs\k.dll").expect("religado ao pai");
    assert_eq!(t.node(n).flags & flags::ORPHAN, 0);
    assert_eq!(t.totals(t.root)[Category::Sistema as usize], 4096);
    assert_eq!(t.totals(t.root)[Category::Outros as usize], 0);
}

#[test]
fn move_into_descendant_terminates() {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut t = build(vec![root(5), dir(30, 5, "a"), dir(31, 30, "b"), file(32, 31, "x.bin", 4096)]);
        t.apply(&[moved_dir(30, 5, "a", 31, "a")], &|_| None, &classifier());
        let _ = tx.send(t.totals(t.root).iter().sum::<u64>());
    });
    let total = rx.recv_timeout(std::time::Duration::from_secs(5)).expect("apply travou ao mover pasta para dentro de si");
    assert_eq!(total, 4096);
}

#[test]
fn removing_many_primary_links_is_fast() {
    let mut recs = vec![root(5), dir(30, 5, "store"), dir(31, 5, "proj")];
    recs.extend((0..200_000u64).map(|i| file(1_000 + i, 30, &format!("f{i}"), 4096)));
    for i in 0..2_000u64 {
        let mut r = file(500_000 + i, 30, &format!("l{i}.js"), 4096);
        r.names.push(FileName { parent: 31, name: format!("l{i}.js"), namespace: 1 });
        recs.push(r);
    }
    let mut t = build(recs);
    let before = t.totals(t.root).iter().sum::<u64>();
    let ch: Vec<Change> = (0..2_000u64).map(|i| link_toggled(500_000 + i, 30, &format!("l{i}.js"))).collect();
    let t0 = std::time::Instant::now();
    t.apply(&ch, &|_| None, &classifier());
    assert!(t0.elapsed() < std::time::Duration::from_secs(1), "{:?}", t0.elapsed());
    assert_eq!(t.totals(t.root).iter().sum::<u64>(), before);
    assert!(t.find(r"C:\proj\l0.js").is_some());
}
