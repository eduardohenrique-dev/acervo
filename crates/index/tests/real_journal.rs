use acervo_index::usn::{reason, Journal};

#[test]
#[ignore = "exige terminal de administrador"]
fn journal_sees_new_file() {
    let j = Journal::open('C').unwrap();
    let info = j.query_or_create().unwrap();
    let path = std::env::temp_dir().join(format!("acervo-teste-{}.bin", std::process::id()));
    std::fs::write(&path, vec![0u8; 1 << 20]).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let mut from = info.next_usn;
    let mut found = None;
    for _ in 0..50 {
        let (next, recs) = j.read(info.journal_id, from, false).unwrap();
        if let Some(r) = recs.into_iter().find(|r| r.name == name && r.reason & reason::CLOSE != 0) {
            found = Some(r);
            break;
        }
        if next == from {
            break;
        }
        from = next;
    }
    let r = found.expect("registro do arquivo");
    let st = j.stat(r.frn_full).expect("stat");
    assert!(st.alloc >= 1 << 20, "alloc = {}", st.alloc);
    std::fs::remove_file(path).unwrap();
}
