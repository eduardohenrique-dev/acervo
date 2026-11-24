//! Persistência do Acervo: snapshot do índice e decisão de como abrir cada disco.

mod plan;
mod snapshot;

pub use plan::{plan_start, StartPlan};
pub use snapshot::{load, save, snapshot_path, SnapshotMeta, StoreError, FORMAT_VERSION};
