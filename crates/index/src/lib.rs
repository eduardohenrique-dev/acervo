//! Índice do Acervo: leitura da MFT, árvore em memória e consultas.

pub mod mft;
mod query;
mod scan;
mod browse;
mod tree;
pub mod usn;
mod walk;

pub use mft::volume::{filesystem_name, scan_ntfs};
pub use mft::{apply_fixup, data_runs_of_record, decode_runs, parse_record, FileName, ParsedRecord, RecordError, Run};
pub use query::{summary, system_breakdown, BreakdownItem, DiskSpace, DriveSummary};
pub use scan::{ScanError, ScanMode, ScanProgress};
pub use tree::{flags, ApplyReport, FileStat, Node, Tree, TreeBuilder, NONE};
pub use walk::{scan_walk, scan_walk_with_cluster};
pub use browse::{children_in, query_files, FileFilter, ItemRow, Page, QueryCache, SortKey, MAX_LIMIT};
