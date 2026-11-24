//! Estado compartilhado: discos, árvores e status das varreduras.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use acervo_classify::Classifier;
use acervo_index::{QueryCache, ScanMode, Tree};
use acervo_ops::Protected;
use serde::Serialize;

use crate::drives::DriveInfo;
use crate::live::LiveHandle;
use crate::queue::OpQueue;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum DriveStatus {
    Scanning { fraction: f32, mode: ScanMode },
    Ready { mode: ScanMode, live: bool },
    Failed { message: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveView {
    pub info: DriveInfo,
    pub status: DriveStatus,
}

pub struct DriveSlot {
    pub info: DriveInfo,
    pub status: DriveStatus,
    pub tree: Option<Arc<RwLock<Tree>>>,
    /// Há uma varredura em andamento para este disco.
    pub busy: bool,
    /// Vigia ao vivo (só no modo MFT).
    pub live: Option<LiveHandle>,
}

pub struct AppState {
    pub drives: RwLock<BTreeMap<char, DriveSlot>>,
    pub classifier: Arc<Classifier>,
    pub elevated: bool,
    /// `%ProgramData%\Acervo`, onde ficam os snapshots.
    pub snapshot_dir: Option<PathBuf>,
    /// Impressão digital das regras de classificação desta máquina.
    pub rules_hash: u64,
    /// Fila de operações de arquivo.
    pub queue: OpQueue,
    /// Cache da lista de arquivos, por disco.
    pub caches: Mutex<HashMap<char, QueryCache>>,
    /// Pastas que nenhuma operação pode tocar.
    pub protected: Protected,
}
