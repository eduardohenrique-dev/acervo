//! Operações de arquivo do Acervo: o que pode ser feito, como fazer e o registro.

mod log;
mod policy;
mod shell;

pub use log::{append_log, format_bytes, log_line, log_path};
pub use policy::{check_dest, check_item, ItemFacts, OpKindTag, Protected, Verdict};
pub use shell::{reveal, run_operation, set_owner_window, ItemFailure, OpKind, OpOutcome, OpProgress};
