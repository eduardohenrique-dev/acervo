mod support;

use acervo_index::usn::{parse_read_buffer, reason, UsnError, ATTR_DIRECTORY};
use support::UsnBuilder;

#[test]
fn parses_buffer() {
    let buf = UsnBuilder::new(5000)
        .record(4000, (2 << 48) | 70, (1 << 48) | 5, reason::FILE_CREATE | reason::CLOSE, 0x20, "a.txt")
        .record(4100, (3 << 48) | 71, (1 << 48) | 5, reason::FILE_DELETE | reason::CLOSE, 0x10, "pasta")
        .build();
    let (next, recs) = parse_read_buffer(&buf).unwrap();
    assert_eq!(next, 5000);
    assert_eq!(recs.len(), 2);
    assert_eq!((recs[0].frn, recs[0].frn_full, recs[0].parent), (70, (2 << 48) | 70, 5));
    assert_eq!(recs[0].name, "a.txt");
    assert_eq!(recs[0].usn, 4000);
    assert_eq!(recs[1].attributes & ATTR_DIRECTORY, ATTR_DIRECTORY);
}

#[test]
fn empty_buffer_has_only_next_usn() {
    assert_eq!(parse_read_buffer(&7000i64.to_le_bytes()).unwrap(), (7000, vec![]));
}

#[test]
fn truncated_record_is_error() {
    let mut buf = UsnBuilder::new(1).record(1, 70, 5, reason::CLOSE, 0, "x").build();
    buf.truncate(buf.len() - 4);
    assert!(matches!(parse_read_buffer(&buf), Err(UsnError::Truncated)));
}

// ---- Coalescedor (Task 2) ----

use acervo_index::usn::{Change, Coalescer, UsnRecord};

fn rec(frn: u64, parent: u64, reason: u32, name: &str) -> UsnRecord {
    UsnRecord { usn: 0, frn, frn_full: frn | (1 << 48), parent, reason, attributes: 0x20, name: name.into() }
}

#[test]
fn create_then_write_then_close_is_one_create_and_one_modify() {
    let mut c = Coalescer::default();
    c.push(&rec(70, 5, reason::FILE_CREATE, "a.txt"));
    c.push(&rec(70, 5, reason::FILE_CREATE | reason::DATA_EXTEND, "a.txt"));
    c.push(&rec(70, 5, reason::FILE_CREATE | reason::DATA_EXTEND | reason::CLOSE, "a.txt"));
    let ch = c.drain();
    assert!(matches!(&ch[0], Change::Created { frn: 70, name, .. } if name == "a.txt"));
    assert!(matches!(ch[1], Change::Modified { frn: 70, .. }));
    assert_eq!(ch.len(), 2);
}

#[test]
fn rename_pairs_old_and_new_across_drains() {
    let mut c = Coalescer::default();
    c.push(&rec(70, 5, reason::RENAME_OLD_NAME, "velho.txt"));
    assert!(c.drain().is_empty());
    c.push(&rec(70, 80, reason::RENAME_NEW_NAME | reason::CLOSE, "novo.txt"));
    assert_eq!(
        c.drain(),
        vec![Change::Moved { frn: 70, frn_full: 70 | (1 << 48), old_parent: 5, old_name: "velho.txt".into(), new_parent: 80, new_name: "novo.txt".into(), is_dir: false, attributes: 0x20 }]
    );
}

#[test]
fn delete_and_modified_dedup() {
    let mut c = Coalescer::default();
    c.push(&rec(71, 5, reason::DATA_OVERWRITE | reason::CLOSE, "b"));
    c.push(&rec(71, 5, reason::DATA_EXTEND | reason::CLOSE, "b"));
    c.push(&rec(72, 5, reason::FILE_DELETE | reason::CLOSE, "c"));
    let ch = c.drain();
    assert_eq!(ch.len(), 2);
    assert!(matches!(ch[0], Change::Deleted { frn: 72, .. }));
    assert!(matches!(ch[1], Change::Modified { frn: 71, .. }));
}

#[test]
fn hard_link_change_is_toggle() {
    let mut c = Coalescer::default();
    c.push(&rec(73, 9, reason::HARD_LINK_CHANGE | reason::CLOSE, "link.dll"));
    assert!(matches!(&c.drain()[0], Change::LinkToggled { frn: 73, parent: 9, .. }));
}

#[test]
fn rename_new_without_old_is_create() {
    let mut c = Coalescer::default();
    c.push(&rec(74, 5, reason::RENAME_NEW_NAME | reason::CLOSE, "x"));
    assert!(matches!(&c.drain()[0], Change::Created { frn: 74, .. }));
}

#[test]
fn created_then_renamed_before_close_is_create() {
    let mut c = Coalescer::default();
    c.push(&rec(75, 5, reason::FILE_CREATE, "tmp123"));
    c.push(&rec(75, 5, reason::RENAME_OLD_NAME | reason::FILE_CREATE, "tmp123"));
    c.push(&rec(75, 30, reason::RENAME_NEW_NAME | reason::FILE_CREATE | reason::DATA_EXTEND | reason::CLOSE, "final.bin"));
    let ch = c.drain();
    assert!(matches!(&ch[0], Change::Created { frn: 75, parent: 30, name, .. } if name == "final.bin"), "{ch:?}");
}
