//! Leitura e interpretação da Master File Table do NTFS.

pub mod record;
pub mod runs;
pub mod volume;

pub use record::{apply_fixup, data_runs_of_record, parse_record, FileName, ParsedRecord, RecordError};
pub use runs::{decode_runs, Run};
