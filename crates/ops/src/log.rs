//! Registro das operações em `<dados>\log\operacoes-AAAA-MM.log` (spec 5).

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::OpKindTag;

const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

/// Tamanho em base 1024 com vírgula, como na interface ("1,2 GB", "931 GB").
pub fn format_bytes(n: u64) -> String {
    if n < 1024 {
        return format!("{n} B");
    }
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    let s = if v >= 100.0 { format!("{}", v.round() as u64) } else { format!("{:.1}", (v * 10.0).round() / 10.0) };
    let s = s.strip_suffix(".0").map(str::to_string).unwrap_or(s);
    format!("{} {}", s.replace('.', ","), UNITS[i])
}

/// Uma linha do log: quando, o quê, de onde, para onde, tamanho e resultado.
pub fn log_line(at: &str, op: OpKindTag, from: &Path, to: Option<&Path>, bytes: u64, result: &str) -> String {
    let target = to.map(|t| format!(" → {}", t.display())).unwrap_or_default();
    format!("{at} {} {}{target} ({}) → {result}", op.verb(), from.display(), format_bytes(bytes))
}

pub fn log_path(dir: &Path, year: i32, month: u32) -> PathBuf {
    dir.join("log").join(format!("operacoes-{year:04}-{month:02}.log"))
}

/// Acrescenta uma linha ao log do mês, criando a pasta se preciso.
pub fn append_log(dir: &Path, year: i32, month: u32, line: &str) -> std::io::Result<()> {
    let path = log_path(dir, year, month);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{line}")
}
