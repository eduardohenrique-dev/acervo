//! Tipos comuns às varreduras.

use serde::Serialize;

/// Como o disco foi lido.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanMode {
    /// Leitura direta da MFT (rápida, exige administrador).
    Mft,
    /// Varredura pasta por pasta (alternativa).
    Walk,
}

/// Progresso de uma varredura.
#[derive(Clone, Copy, Debug)]
pub struct ScanProgress {
    pub drive: char,
    pub fraction: f32,
    pub mode: ScanMode,
}

#[derive(Debug)]
pub enum ScanError {
    AccessDenied,
    NotNtfs,
    Io(std::io::Error),
    TooManyBadRecords { skipped: u64, total: u64 },
    Fragmented,
    /// O USN Journal foi apagado, recriado ou truncado além do ponto lido.
    JournalReset,
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScanError::AccessDenied => write!(f, "acesso negado ao disco (é preciso rodar como administrador)"),
            ScanError::NotNtfs => write!(f, "o disco não é NTFS"),
            ScanError::Io(e) => write!(f, "erro de leitura: {e}"),
            ScanError::TooManyBadRecords { skipped, total } => {
                write!(f, "{skipped} de {total} registros da MFT estão ilegíveis")
            }
            ScanError::Fragmented => write!(f, "a MFT está fragmentada demais para leitura direta"),
            ScanError::JournalReset => write!(f, "o diário de mudanças do disco foi reiniciado"),
        }
    }
}

impl std::error::Error for ScanError {}

impl From<std::io::Error> for ScanError {
    fn from(e: std::io::Error) -> Self {
        if e.raw_os_error() == Some(5) {
            ScanError::AccessDenied
        } else {
            ScanError::Io(e)
        }
    }
}
