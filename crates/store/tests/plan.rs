use acervo_index::usn::JournalInfo;
use acervo_store::{plan_start, SnapshotMeta, StartPlan, FORMAT_VERSION};

fn j(id: u64, first: i64, next: i64) -> JournalInfo {
    JournalInfo { journal_id: id, first_usn: first, next_usn: next }
}

fn meta_at(journal_id: u64, next_usn: i64, rules_hash: u64) -> SnapshotMeta {
    SnapshotMeta { version: FORMAT_VERSION, volume_serial: 0xABCD, journal_id, next_usn, rules_hash, saved_at: 1 }
}

#[test]
fn replays_when_journal_matches() {
    assert_eq!(
        plan_start(Some(&meta_at(7, 9000, 42)), &j(7, 100, 20_000), 0xABCD, 42),
        StartPlan::Replay { from: 9000, reclassify: false }
    );
}

#[test]
fn changed_rules_reclassify() {
    assert_eq!(
        plan_start(Some(&meta_at(7, 9000, 42)), &j(7, 100, 20_000), 0xABCD, 43),
        StartPlan::Replay { from: 9000, reclassify: true }
    );
}

#[test]
fn truncated_journal_forces_full_scan() {
    assert_eq!(plan_start(Some(&meta_at(7, 9000, 42)), &j(7, 9500, 20_000), 0xABCD, 42), StartPlan::FullScan);
}

#[test]
fn new_journal_id_forces_full_scan() {
    assert_eq!(plan_start(Some(&meta_at(7, 9000, 42)), &j(8, 0, 500), 0xABCD, 42), StartPlan::FullScan);
    assert_eq!(plan_start(None, &j(7, 0, 500), 0xABCD, 42), StartPlan::FullScan);
}

#[test]
fn other_volume_forces_full_scan() {
    assert_eq!(plan_start(Some(&meta_at(7, 9000, 42)), &j(7, 100, 20_000), 0x1234, 42), StartPlan::FullScan);
}
