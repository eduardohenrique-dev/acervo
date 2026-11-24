//! Decide, ao abrir, se dá para reaproveitar o snapshot.

use acervo_index::usn::JournalInfo;

use crate::SnapshotMeta;

#[derive(Debug, PartialEq)]
pub enum StartPlan {
    /// Usar o snapshot e reaplicar o journal a partir de `from`.
    Replay { from: i64, reclassify: bool },
    /// Analisar o disco inteiro.
    FullScan,
}

/// Sem snapshot, outro volume, outro journal ou journal truncado além do ponto
/// salvo → análise completa. Regras diferentes → reaproveita e reclassifica.
pub fn plan_start(meta: Option<&SnapshotMeta>, journal: &JournalInfo, volume_serial: u64, rules_hash: u64) -> StartPlan {
    let Some(m) = meta else { return StartPlan::FullScan };
    if m.volume_serial != volume_serial || m.journal_id != journal.journal_id || m.next_usn < journal.first_usn {
        return StartPlan::FullScan;
    }
    StartPlan::Replay { from: m.next_usn, reclassify: m.rules_hash != rules_hash }
}
