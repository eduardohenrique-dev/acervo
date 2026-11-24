//! USN Journal do NTFS: leitura de registros e tradução em mudanças da árvore.

pub mod changes;
pub mod journal;
pub mod record;

pub use changes::{Change, Coalescer};
pub use journal::Journal;
pub use record::{parse_read_buffer, reason, UsnError, UsnRecord, ATTR_DIRECTORY};

/// Estado do journal de um volume (`FSCTL_QUERY_USN_JOURNAL`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JournalInfo {
    pub journal_id: u64,
    /// O registro mais antigo ainda disponível.
    pub first_usn: i64,
    /// Onde o próximo registro será gravado.
    pub next_usn: i64,
}
