//! Orquestra a varredura de cada disco.

use std::cell::Cell;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use acervo_classify::Classifier;
use acervo_index::{scan_ntfs, scan_walk, ScanError, ScanMode, ScanProgress, Tree};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::live::{open_live, run_watcher, LiveHandle, LiveStart, WatchEvent};
use crate::state::{AppState, DriveStatus};

const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

/// Forma de ler um disco; abstraída para os testes.
pub trait Scanner {
    fn ntfs(&self, drive: char, progress: &dyn Fn(ScanProgress)) -> Result<Tree, ScanError>;
    fn walk(&self, drive: char, progress: &dyn Fn(ScanProgress)) -> Result<Tree, ScanError>;
}

pub struct RealScanner {
    pub classifier: Arc<Classifier>,
}

impl Scanner for RealScanner {
    fn ntfs(&self, drive: char, progress: &dyn Fn(ScanProgress)) -> Result<Tree, ScanError> {
        scan_ntfs(drive, &self.classifier, progress)
    }
    fn walk(&self, drive: char, progress: &dyn Fn(ScanProgress)) -> Result<Tree, ScanError> {
        scan_walk(Path::new(&format!("{drive}:\\")), drive, &self.classifier, progress)
    }
}

/// Lê o disco pela MFT quando possível; senão, pela varredura alternativa.
pub fn run_drive(
    drive: char,
    elevated: bool,
    scanner: &dyn Scanner,
    progress: &dyn Fn(ScanProgress),
) -> Result<(Tree, ScanMode), String> {
    let fail = |e: ScanError| format!("Não foi possível analisar o disco {drive}: — {e}");
    if elevated {
        match scanner.ntfs(drive, progress) {
            Ok(t) => return Ok((t, ScanMode::Mft)),
            Err(e) if falls_back(&e) => {}
            Err(e) => return Err(fail(e)),
        }
    }
    scanner.walk(drive, progress).map(|t| (t, ScanMode::Walk)).map_err(fail)
}

/// Erros da leitura da MFT que ainda permitem a varredura alternativa.
fn falls_back(e: &ScanError) -> bool {
    match e {
        ScanError::NotNtfs | ScanError::Fragmented | ScanError::TooManyBadRecords { .. } => true,
        // Volume bloqueado mesmo como administrador, ou em uso (chkdsk, desfragmentação).
        ScanError::AccessDenied => true,
        ScanError::Io(io) => matches!(io.raw_os_error(), Some(32 | 33)),
        // Só a vigia ao vivo produz este erro; a análise inteira nunca o devolve.
        ScanError::JournalReset => true,
    }
}

/// `run_drive`, mas um pânico em qualquer ponto da leitura vira falha visível.
pub fn run_drive_guarded(
    drive: char,
    elevated: bool,
    scanner: &dyn Scanner,
    progress: &dyn Fn(ScanProgress),
) -> Result<(Tree, ScanMode), String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_drive(drive, elevated, scanner, progress)))
        .unwrap_or_else(|_| Err(format!("Não foi possível analisar o disco {drive}: — erro inesperado na leitura")))
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    drive: char,
    fraction: f32,
    mode: ScanMode,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadyEvent {
    drive: char,
    live: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FailedEvent {
    drive: char,
    message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdatedEvent {
    drive: char,
}

fn set_status(app: &AppHandle, drive: char, status: DriveStatus, tree: Option<Arc<RwLock<Tree>>>) {
    let state = app.state::<AppState>();
    let mut drives = state.drives.write().unwrap();
    if let Some(slot) = drives.get_mut(&drive) {
        slot.busy = matches!(status, DriveStatus::Scanning { .. });
        slot.status = status;
        if tree.is_some() {
            slot.tree = tree;
        }
    }
}

fn fail(app: &AppHandle, drive: char, message: String) {
    set_status(app, drive, DriveStatus::Failed { message: message.clone() }, None);
    let _ = app.emit("index-failed", FailedEvent { drive, message });
}

/// Começa a analisar um disco numa thread própria. Ignora se já estiver analisando.
/// `force_full` ignora o snapshot (botão "Tentar de novo" e journal reiniciado).
pub fn start_drive(app: AppHandle, drive: char, force_full: bool) {
    let previous_live = {
        let state = app.state::<AppState>();
        let mut drives = state.drives.write().unwrap();
        let Some(slot) = drives.get_mut(&drive) else { return };
        if slot.busy {
            return;
        }
        slot.busy = true;
        let mode = if state.elevated { ScanMode::Mft } else { ScanMode::Walk };
        slot.status = DriveStatus::Scanning { fraction: 0.0, mode };
        slot.live.take()
    };
    std::thread::spawn(move || {
        if let Some(mut h) = previous_live {
            h.stop_and_wait();
        }
        let state = app.state::<AppState>();
        let last = Cell::new(Instant::now() - PROGRESS_INTERVAL);
        let progress = |p: ScanProgress| {
            if last.get().elapsed() < PROGRESS_INTERVAL {
                return;
            }
            last.set(Instant::now());
            set_status(&app, drive, DriveStatus::Scanning { fraction: p.fraction, mode: p.mode }, None);
            let _ = app.emit("index-progress", ProgressEvent { drive, fraction: p.fraction, mode: p.mode });
        };

        if state.elevated {
            let opened = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                open_live(drive, &state.classifier, state.snapshot_dir.as_deref(), state.rules_hash, force_full, &progress)
            }));
            match opened {
                Ok(Ok(start)) => return go_live(&app, drive, start),
                Ok(Err(_)) => {} // sem ao vivo neste disco: segue pela análise normal
                Err(_) => return fail(&app, drive, format!("Não foi possível analisar o disco {drive}: — erro inesperado na leitura")),
            }
        }

        let scanner = RealScanner { classifier: state.classifier.clone() };
        match run_drive_guarded(drive, state.elevated, &scanner, &progress) {
            Ok((tree, mode)) => {
                set_status(&app, drive, DriveStatus::Ready { mode, live: false }, Some(Arc::new(RwLock::new(tree))));
                let _ = app.emit("index-ready", ReadyEvent { drive, live: false });
            }
            Err(message) => fail(&app, drive, message),
        }
    });
}

/// Marca o disco como pronto e ao vivo, e inicia a vigia dele.
fn go_live(app: &AppHandle, drive: char, start: LiveStart) {
    let stop = Arc::new(AtomicBool::new(false));
    let meta = Arc::new(Mutex::new(start.meta.clone()));
    let tree = start.tree.clone();
    let path = start.path.clone();
    let classifier = app.state::<AppState>().classifier.clone();
    let watcher_app = app.clone();
    let (watch_stop, watch_meta) = (stop.clone(), meta.clone());
    let join = std::thread::spawn(move || {
        let notify = |ev: WatchEvent| match ev {
            WatchEvent::Updated => {
                let _ = watcher_app.emit("index-updated", UpdatedEvent { drive });
            }
            WatchEvent::Reset => {
                clear_live(&watcher_app, drive);
                start_drive(watcher_app.clone(), drive, true);
            }
            WatchEvent::Failed(message) => {
                clear_live(&watcher_app, drive);
                fail(&watcher_app, drive, format!("Não foi possível acompanhar o disco {drive}: — {message}"));
            }
        };
        run_watcher(start, classifier, watch_stop, watch_meta, &notify);
    });
    {
        let state = app.state::<AppState>();
        let mut drives = state.drives.write().unwrap();
        if let Some(slot) = drives.get_mut(&drive) {
            slot.busy = false;
            slot.status = DriveStatus::Ready { mode: ScanMode::Mft, live: true };
            slot.tree = Some(tree.clone());
            slot.live = Some(LiveHandle { stop, join: Some(join), meta, path, tree });
        }
    }
    let _ = app.emit("index-ready", ReadyEvent { drive, live: true });
}

/// A vigia terminou sozinha: tira o controle dela do estado (sem esperar, ela já saiu).
fn clear_live(app: &AppHandle, drive: char) {
    let state = app.state::<AppState>();
    let mut drives = state.drives.write().unwrap();
    if let Some(slot) = drives.get_mut(&drive) {
        slot.live = None;
        if let DriveStatus::Ready { mode, .. } = slot.status {
            slot.status = DriveStatus::Ready { mode, live: false };
        }
    }
}

/// Ao sair do app: para todas as vigias e salva os snapshots.
pub fn shutdown(app: &AppHandle) {
    let state = app.state::<AppState>();
    let handles: Vec<LiveHandle> = state.drives.write().unwrap().values_mut().filter_map(|s| s.live.take()).collect();
    for mut h in handles {
        h.stop_and_wait();
        let _ = h.save();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use acervo_classify::Classifier;
    use acervo_index::{ScanError, ScanMode, ScanProgress, Tree, TreeBuilder};

    use super::*;

    fn empty_tree() -> Tree {
        TreeBuilder::new('C', 5, 0).finish(&Classifier::new(vec![]))
    }

    struct Fake {
        ntfs: RefCell<Option<Result<Tree, ScanError>>>,
        walk: RefCell<Option<Result<Tree, ScanError>>>,
        ntfs_calls: Cell<u32>,
    }

    impl Fake {
        fn new(ntfs: Result<Tree, ScanError>, walk: Result<Tree, ScanError>) -> Fake {
            Fake { ntfs: RefCell::new(Some(ntfs)), walk: RefCell::new(Some(walk)), ntfs_calls: Cell::new(0) }
        }
    }

    impl Scanner for Fake {
        fn ntfs(&self, _: char, _: &dyn Fn(ScanProgress)) -> Result<Tree, ScanError> {
            self.ntfs_calls.set(self.ntfs_calls.get() + 1);
            self.ntfs.borrow_mut().take().unwrap()
        }
        fn walk(&self, _: char, _: &dyn Fn(ScanProgress)) -> Result<Tree, ScanError> {
            self.walk.borrow_mut().take().unwrap()
        }
    }

    #[test]
    fn not_elevated_uses_walk() {
        let s = Fake::new(Err(ScanError::AccessDenied), Ok(empty_tree()));
        let (_, mode) = run_drive('C', false, &s, &|_| {}).unwrap();
        assert_eq!(mode, ScanMode::Walk);
        assert_eq!(s.ntfs_calls.get(), 0);
    }

    #[test]
    fn fragmented_falls_back_to_walk() {
        let s = Fake::new(Err(ScanError::Fragmented), Ok(empty_tree()));
        assert_eq!(run_drive('C', true, &s, &|_| {}).unwrap().1, ScanMode::Walk);
    }

    #[test]
    fn elevated_ntfs_uses_mft() {
        let s = Fake::new(Ok(empty_tree()), Err(ScanError::AccessDenied));
        assert_eq!(run_drive('C', true, &s, &|_| {}).unwrap().1, ScanMode::Mft);
    }

    #[test]
    fn elevated_access_denied_falls_back() {
        let s = Fake::new(Err(ScanError::AccessDenied), Ok(empty_tree()));
        assert_eq!(run_drive('C', true, &s, &|_| {}).unwrap().1, ScanMode::Walk);
    }

    #[test]
    fn volume_in_use_falls_back() {
        let s = Fake::new(Err(ScanError::Io(std::io::Error::from_raw_os_error(32))), Ok(empty_tree()));
        assert_eq!(run_drive('C', true, &s, &|_| {}).unwrap().1, ScanMode::Walk);
    }

    struct Panics;
    impl Scanner for Panics {
        fn ntfs(&self, _: char, _: &dyn Fn(ScanProgress)) -> Result<Tree, ScanError> {
            panic!("registro corrompido")
        }
        fn walk(&self, _: char, _: &dyn Fn(ScanProgress)) -> Result<Tree, ScanError> {
            panic!("registro corrompido")
        }
    }

    #[test]
    fn panic_becomes_failure() {
        let err = run_drive_guarded('D', true, &Panics, &|_| {}).unwrap_err();
        assert!(err.contains("D:"), "{err}");
    }

    #[test]
    fn run_drive_reports_failure() {
        let s = Fake::new(
            Err(ScanError::Io(std::io::Error::other("disco removido"))),
            Err(ScanError::AccessDenied),
        );
        let err = run_drive('D', true, &s, &|_| {}).unwrap_err();
        assert!(err.contains("D:"), "{err}");
    }
}
