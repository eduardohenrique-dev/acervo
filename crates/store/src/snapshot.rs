//! `index-<série>.bin`: `b"ACVO"`, versão (u32 LE) e um stream zstd de `bincode((meta, tree))`.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use acervo_index::Tree;
use serde::{Deserialize, Serialize};

/// 2: a árvore ganhou o índice de links e a lista de órfãos.
pub const FORMAT_VERSION: u32 = 2;
const MAGIC: &[u8; 4] = b"ACVO";
const ZSTD_LEVEL: i32 = 3;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub version: u32,
    pub volume_serial: u64,
    pub journal_id: u64,
    /// Primeiro USN ainda não aplicado à árvore salva.
    pub next_usn: i64,
    pub rules_hash: u64,
    /// Segundos desde 1970.
    pub saved_at: i64,
}

#[derive(Debug)]
pub enum StoreError {
    Missing,
    Corrupt(String),
    Version(u32),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Missing => write!(f, "snapshot inexistente"),
            StoreError::Corrupt(e) => write!(f, "snapshot corrompido: {e}"),
            StoreError::Version(v) => write!(f, "snapshot de outra versão ({v})"),
        }
    }
}

pub fn snapshot_path(dir: &Path, volume_serial: u64) -> PathBuf {
    dir.join(format!("index-{volume_serial:016x}.bin"))
}

/// Grava o snapshot de forma atômica: escreve `<path>.tmp` e renomeia por cima.
pub fn save(path: &Path, meta: &SnapshotMeta, tree: &Tree) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("bin.tmp");
    // Nunca reaproveita um .tmp que já exista (poderia ter sido criado por outro usuário).
    match std::fs::remove_file(&tmp) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    {
        let file = std::fs::OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        let mut w = BufWriter::new(file);
        w.write_all(MAGIC)?;
        w.write_all(&FORMAT_VERSION.to_le_bytes())?;
        let mut enc = zstd::Encoder::new(w, ZSTD_LEVEL)?;
        bincode::serialize_into(&mut enc, &(meta, tree)).map_err(std::io::Error::other)?;
        let w = enc.finish()?;
        let file = w.into_inner().map_err(|e| e.into_error())?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path)
}

pub fn load(path: &Path) -> Result<(SnapshotMeta, Tree), StoreError> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(StoreError::Missing),
        Err(e) => return Err(StoreError::Corrupt(e.to_string())),
    };
    let mut r = BufReader::new(file);
    let mut head = [0u8; 8];
    r.read_exact(&mut head).map_err(|_| StoreError::Corrupt("cabeçalho curto".into()))?;
    if &head[0..4] != MAGIC {
        return Err(StoreError::Corrupt("assinatura inválida".into()));
    }
    let version = u32::from_le_bytes(head[4..8].try_into().unwrap());
    if version != FORMAT_VERSION {
        return Err(StoreError::Version(version));
    }
    let dec = zstd::Decoder::new(r).map_err(|e| StoreError::Corrupt(e.to_string()))?;
    let (meta, tree): (SnapshotMeta, Tree) = bincode::deserialize_from(dec).map_err(|e| StoreError::Corrupt(e.to_string()))?;
    if meta.version != FORMAT_VERSION {
        return Err(StoreError::Version(meta.version));
    }
    if !tree.validate() {
        return Err(StoreError::Corrupt("índices fora do lugar".into()));
    }
    Ok((meta, tree))
}
