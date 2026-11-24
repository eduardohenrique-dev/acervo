mod support;

use acervo_classify::{Category, Classifier, PathRule, Strength};
use acervo_index::{children_in, query_files, FileFilter, QueryCache, SortKey, Tree};
use support::*;

fn t() -> Tree {
    build_with(
        vec![
            root(5),
            dir(30, 5, "Jogos"),
            dir(31, 30, "Hades"),
            file(32, 31, "a.pak", 8192),
            file(33, 31, "b.pak", 4096),
            dir(40, 5, "Windows"),
            file(41, 40, "k.dll", 65536),
            file(50, 5, "nota.txt", 100),
            file(51, 5, "Foto.JPG", 20000),
        ],
        &Classifier::new(vec![
            PathRule { path: r"C:\Jogos".into(), category: Category::Jogos, strength: Strength::Strong },
            PathRule { path: r"C:\Windows".into(), category: Category::Sistema, strength: Strength::Strong },
        ]),
    )
}

#[test]
fn children_in_category_sorted_by_bytes() {
    let tree = t();
    let rows = children_in(&tree, tree.root, Some(Category::Jogos)).unwrap();
    assert_eq!(rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), vec!["Jogos"]);
    assert_eq!(rows[0].bytes, 12288);
    let inside = children_in(&tree, rows[0].node, Some(Category::Jogos)).unwrap();
    assert_eq!(inside[0].name, "Hades");
    let files = children_in(&tree, inside[0].node, Some(Category::Jogos)).unwrap();
    assert_eq!(files.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), vec!["a.pak", "b.pak"]);
    assert_eq!(files[0].path, r"C:\Jogos\Hades\a.pak");
}

#[test]
fn system_never_listed() {
    let tree = t();
    let all = children_in(&tree, tree.root, None).unwrap();
    assert!(all.iter().all(|r| r.name != "Windows"));
    let page = query_files(&tree, &mut QueryCache::default(), &FileFilter::default(), SortKey::SizeDesc, 0, 100);
    assert!(page.rows.iter().all(|r| r.name != "k.dll"));
}

#[test]
fn files_sorted_smallest_first_and_paged() {
    let tree = t();
    let mut c = QueryCache::default();
    let p = query_files(&tree, &mut c, &FileFilter::default(), SortKey::SizeAsc, 0, 2);
    assert_eq!(p.total, 4);
    assert_eq!(p.rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), vec!["nota.txt", "b.pak"]);
    let p2 = query_files(&tree, &mut c, &FileFilter::default(), SortKey::SizeAsc, 2, 2);
    assert_eq!(p2.rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), vec!["a.pak", "Foto.JPG"]);
}

#[test]
fn filters_combine() {
    let tree = t();
    let mut c = QueryCache::default();
    let f = FileFilter { search: Some("PAK".into()), min_size: Some(5000), ..Default::default() };
    assert_eq!(query_files(&tree, &mut c, &f, SortKey::SizeDesc, 0, 10).rows[0].name, "a.pak");
    let f = FileFilter { extension: Some("jpg".into()), ..Default::default() };
    assert_eq!(query_files(&tree, &mut c, &f, SortKey::SizeDesc, 0, 10).total, 1);
    let jogos = tree.find(r"C:\Jogos").unwrap();
    let f = FileFilter { under: Some(jogos), ..Default::default() };
    assert_eq!(query_files(&tree, &mut c, &f, SortKey::NameAsc, 0, 10).total, 2);
    // `file()` grava modified = 0; nada é mais novo que 1
    let f = FileFilter { modified_after: Some(1), ..Default::default() };
    assert_eq!(query_files(&tree, &mut c, &f, SortKey::SizeDesc, 0, 10).total, 0);
}

#[test]
fn page_beyond_end_is_empty() {
    let tree = t();
    let p = query_files(&tree, &mut QueryCache::default(), &FileFilter::default(), SortKey::SizeDesc, 50, 10);
    assert!(p.rows.is_empty());
    assert_eq!(p.total, 4);
}

#[test]
fn cache_follows_tree_version() {
    let mut tree = t();
    let mut c = QueryCache::default();
    assert_eq!(query_files(&tree, &mut c, &FileFilter::default(), SortKey::SizeDesc, 0, 10).total, 4);
    tree.apply(&[deleted(50, 5, "nota.txt")], &|_| None, &classifier());
    assert_eq!(query_files(&tree, &mut c, &FileFilter::default(), SortKey::SizeDesc, 0, 10).total, 3);
}

#[test]
fn children_of_dead_node_is_none() {
    let mut tree = t();
    let jogos = tree.find(r"C:\Jogos").unwrap();
    tree.apply(&[deleted(30, 5, "Jogos")], &|_| None, &classifier());
    assert!(children_in(&tree, jogos, None).is_none());
    assert!(children_in(&tree, 999_999, None).is_none());
}

#[test]
#[ignore = "desempenho: rodar em release"]
fn query_two_million_is_fast() {
    let mut recs = vec![root(5), dir(30, 5, "a")];
    recs.extend((0..2_000_000u64).map(|i| file(100 + i, 30, &format!("f{i}"), (i % 10_000) * 4096)));
    let tree = build(recs);
    let mut c = QueryCache::default();
    let t0 = std::time::Instant::now();
    let p = query_files(&tree, &mut c, &FileFilter::default(), SortKey::SizeAsc, 0, 200);
    assert!(t0.elapsed() < std::time::Duration::from_millis(1500), "primeira: {:?}", t0.elapsed());
    assert_eq!(p.total, 2_000_000);
    let t1 = std::time::Instant::now();
    query_files(&tree, &mut c, &FileFilter::default(), SortKey::SizeDesc, 10_000, 200);
    assert!(t1.elapsed() < std::time::Duration::from_millis(50), "seguinte: {:?}", t1.elapsed());
}
