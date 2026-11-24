//! Atualização ao vivo: aplica o USN Journal na árvore enquanto o app está aberto.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use acervo_classify::Classifier;
use acervo_index::usn::{Change, Coalescer, Journal, JournalInfo, UsnRecord};
use acervo_index::{scan_ntfs, FileStat, ScanError, ScanProgress, Tree};
use acervo_store::{load, plan_start, save, snapshot_path, SnapshotMeta, StartPlan, FORMAT_VERSION};

/// Intervalo mínimo entre dois avisos `index-updated` para a interface.
pub const EMIT_INTERVAL: Duration = Duration::from_secs(1);
/// Intervalo entre gravações periódicas do snapshot (spec 5).
pub const SAVE_INTERVAL: Duration = Duration::from_secs(10 * 60);

/// Erros seguidos da vigia antes de desistir e mostrar a falha.
pub const MAX_CONSECUTIVE_ERRORS: u32 = 5;

/// De onde vêm os registros; abstraído para os testes.
pub trait LiveSource {
    fn read(&mut self, from: i64, wait: bool) -> Result<(i64, Vec<UsnRecord>), ScanError>;
    fn stat(&self, frn_full: u64) -> Option<FileStat>;
    /// Journal que a vigia está lendo.
    fn journal_id(&self) -> u64;
    /// Estado atual do journal no disco.
    fn journal_info(&self) -> Result<JournalInfo, ScanError>;
}

/// O que fazer depois de um erro de leitura da vigia.
pub enum ErrorDecision {
    Retry,
    Reset,
    Fail,
}

/// Confere o journal: trocado ou truncado além de `from` → nova análise; erros seguidos demais → falha.
pub fn decide_on_error(src: &dyn LiveSource, from: i64, consecutive: u32) -> ErrorDecision {
    match src.journal_info() {
        Ok(i) if i.journal_id != src.journal_id() || i.first_usn > from => ErrorDecision::Reset,
        Err(ScanError::JournalReset) => ErrorDecision::Reset,
        _ if consecutive >= MAX_CONSECUTIVE_ERRORS => ErrorDecision::Fail,
        _ => ErrorDecision::Retry,
    }
}

/// Repete `f` enquanto ela devolver `JournalReset` (ex.: journal sendo apagado), até `attempts` vezes.
pub fn with_retry<T>(attempts: u32, delay: Duration, mut f: impl FnMut() -> Result<T, ScanError>) -> Result<T, ScanError> {
    let mut last = Err(ScanError::JournalReset);
    for i in 0..attempts.max(1) {
        last = f();
        match &last {
            Err(ScanError::JournalReset) if i + 1 < attempts => std::thread::sleep(delay),
            _ => return last,
        }
    }
    last
}

pub struct JournalSource {
    pub journal: Journal,
    pub journal_id: u64,
}

impl LiveSource for JournalSource {
    fn read(&mut self, from: i64, wait: bool) -> Result<(i64, Vec<UsnRecord>), ScanError> {
        self.journal.read(self.journal_id, from, wait)
    }
    fn stat(&self, frn_full: u64) -> Option<FileStat> {
        self.journal.stat(frn_full)
    }
    fn journal_id(&self) -> u64 {
        self.journal_id
    }
    fn journal_info(&self) -> Result<JournalInfo, ScanError> {
        self.journal.query_or_create()
    }
}

/// Lê um lote, lê os tamanhos **fora** do lock e aplica na árvore. Devolve (próximo USN, mudou).
pub fn pump_once(
    tree: &RwLock<Tree>,
    src: &mut dyn LiveSource,
    co: &mut Coalescer,
    from: i64,
    wait: bool,
    classifier: &Classifier,
) -> Result<(i64, bool), ScanError> {
    let (next, recs) = src.read(from, wait)?;
    for r in &recs {
        co.push(r);
    }
    let changes = co.drain();
    if changes.is_empty() {
        return Ok((next, false));
    }
    let mut stats: HashMap<u64, Option<FileStat>> = HashMap::new();
    for c in &changes {
        let frn_full = match c {
            Change::Created { frn_full, is_dir: false, .. }
            | Change::Moved { frn_full, is_dir: false, .. }
            | Change::Modified { frn_full, .. }
            | Change::LinkToggled { frn_full, .. } => *frn_full,
            _ => continue,
        };
        stats.entry(frn_full).or_insert_with(|| src.stat(frn_full));
    }
    let report = tree.write().unwrap().apply(&changes, &|f| stats.get(&f).copied().flatten(), classifier);
    Ok((next, report.changed))
}

/// Reaplica o journal sem esperar até alcançar `target` (ou até não haver mais nada).
pub fn catch_up(
    tree: &RwLock<Tree>,
    src: &mut dyn LiveSource,
    co: &mut Coalescer,
    mut from: i64,
    target: i64,
    classifier: &Classifier,
) -> Result<i64, ScanError> {
    loop {
        let (next, _) = pump_once(tree, src, co, from, false, classifier)?;
        if next >= target || next == from {
            return Ok(next);
        }
        from = next;
    }
}

/// Tudo o que a vigia de um disco precisa para começar.
pub struct LiveStart {
    pub tree: Arc<RwLock<Tree>>,
    pub src: JournalSource,
    pub coalescer: Coalescer,
    pub meta: SnapshotMeta,
    /// `None` quando a pasta dos snapshots não pôde ser protegida.
    pub path: Option<PathBuf>,
}

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Abre um disco NTFS para o modo ao vivo: snapshot + journal, ou análise completa + journal.
pub fn open_live(
    drive: char,
    classifier: &Classifier,
    snapshot_dir: Option<&Path>,
    rules_hash: u64,
    force_full: bool,
    progress: &dyn Fn(ScanProgress),
) -> Result<LiveStart, ScanError> {
    let journal = Journal::open(drive)?;
    // Logo após `fsutil usn deletejournal` o journal fica "sendo apagado" por um tempo.
    let info = with_retry(20, Duration::from_millis(500), || journal.query_or_create())?;
    let serial = journal.volume_serial();
    let path = snapshot_dir.map(|d| snapshot_path(d, serial));
    let loaded = match (&path, force_full) {
        (Some(p), false) => load(p).ok(),
        _ => None,
    };
    let plan = plan_start(loaded.as_ref().map(|(m, _)| m), &info, serial, rules_hash);
    let (tree, from) = match (plan, loaded) {
        (StartPlan::Replay { from, reclassify }, Some((_, mut t))) => {
            if reclassify {
                t.reclassify_all(classifier);
            }
            (t, from)
        }
        // O ponto de partida é lido antes da análise: o que mudar durante ela é reaplicado depois.
        _ => (scan_ntfs(drive, classifier, progress)?, info.next_usn),
    };
    let tree = Arc::new(RwLock::new(tree));
    let mut src = JournalSource { journal, journal_id: info.journal_id };
    let mut coalescer = Coalescer::default();
    let target = src.journal.query_or_create()?.next_usn;
    let next = catch_up(&tree, &mut src, &mut coalescer, from, target, classifier)?;
    let meta = SnapshotMeta {
        version: FORMAT_VERSION,
        volume_serial: serial,
        journal_id: info.journal_id,
        next_usn: next,
        rules_hash,
        saved_at: now_secs(),
    };
    Ok(LiveStart { tree, src, coalescer, meta, path })
}

/// Grava o snapshot com o estado atual (lock de leitura durante a serialização).
pub fn save_snapshot(tree: &RwLock<Tree>, meta: &Mutex<SnapshotMeta>, path: &Path) -> std::io::Result<()> {
    let mut m = meta.lock().unwrap().clone();
    m.saved_at = now_secs();
    let t = tree.read().unwrap();
    save(path, &m, &t)
}

/// Controle da vigia de um disco.
pub struct LiveHandle {
    pub stop: Arc<AtomicBool>,
    pub join: Option<std::thread::JoinHandle<()>>,
    pub meta: Arc<Mutex<SnapshotMeta>>,
    pub path: Option<PathBuf>,
    pub tree: Arc<RwLock<Tree>>,
}

impl LiveHandle {
    /// Pede para a vigia parar e espera no máximo 2 s.
    pub fn stop_and_wait(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let deadline = Instant::now() + Duration::from_secs(2);
        while self.join.as_ref().is_some_and(|j| !j.is_finished()) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        if self.join.as_ref().is_some_and(|j| j.is_finished()) {
            let _ = self.join.take().map(|j| j.join());
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        match &self.path {
            Some(p) => save_snapshot(&self.tree, &self.meta, p),
            None => Ok(()),
        }
    }
}

/// O que a vigia avisa a quem a criou.
pub enum WatchEvent {
    Updated,
    Reset,
    Failed(String),
}

/// Laço da vigia: lê com espera, aplica, avisa no máximo 1×/s e salva a cada 10 min.
pub fn run_watcher(start: LiveStart, classifier: Arc<Classifier>, stop: Arc<AtomicBool>, meta: Arc<Mutex<SnapshotMeta>>, notify: &dyn Fn(WatchEvent)) {
    let LiveStart { tree, mut src, mut coalescer, path, .. } = start;
    let mut from = meta.lock().unwrap().next_usn;
    let mut last_emit = Instant::now() - EMIT_INTERVAL;
    let mut pending = false;
    let mut last_save = Instant::now();
    let mut dirty = false;
    let mut errors = 0u32;
    while !stop.load(Ordering::SeqCst) {
        let step = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pump_once(&tree, &mut src, &mut coalescer, from, true, &classifier)
        }));
        match step {
            Ok(Ok((next, changed))) => {
                errors = 0;
                from = next;
                meta.lock().unwrap().next_usn = next;
                if changed {
                    pending = true;
                    dirty = true;
                }
            }
            Ok(Err(ScanError::JournalReset)) => return notify(WatchEvent::Reset),
            Ok(Err(_)) => {
                errors += 1;
                match decide_on_error(&src, from, errors) {
                    ErrorDecision::Reset => return notify(WatchEvent::Reset),
                    ErrorDecision::Fail => {
                        return notify(WatchEvent::Failed("o disco parou de responder ao acompanhamento".into()))
                    }
                    ErrorDecision::Retry => std::thread::sleep(Duration::from_secs(1)),
                }
            }
            Err(_) => return notify(WatchEvent::Failed("erro inesperado ao acompanhar as mudanças".into())),
        }
        if pending && last_emit.elapsed() >= EMIT_INTERVAL {
            notify(WatchEvent::Updated);
            pending = false;
            last_emit = Instant::now();
        }
        if dirty && last_save.elapsed() >= SAVE_INTERVAL {
            if path.as_ref().is_none_or(|p| save_snapshot(&tree, &meta, p).is_ok()) {
                dirty = false;
            }
            last_save = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::collections::{HashMap, VecDeque};
    use std::sync::{Arc, RwLock};

    use acervo_classify::{Category, Classifier};
    use acervo_index::usn::{reason, Coalescer, JournalInfo, UsnRecord};
    use acervo_index::{FileName, FileStat, ParsedRecord, ScanError, Tree, TreeBuilder};

    use super::*;

    const MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

    fn rec(no: u64, parent: u64, name: &str, dir: bool) -> ParsedRecord {
        ParsedRecord {
            record_no: no,
            in_use: true,
            is_dir: dir,
            names: vec![FileName { parent, name: name.into(), namespace: 1 }],
            ..Default::default()
        }
    }

    /// build(vec![root(5), dir(30, 5, "Fotos")])
    fn tree_with_dir(no: u64, name: &str) -> Tree {
        let mut b = TreeBuilder::new('C', 5, 64);
        b.push(rec(5, 5, ".", true));
        b.push(rec(no, 5, name, true));
        b.finish(&classifier())
    }

    fn classifier() -> Classifier {
        Classifier::new(vec![])
    }

    fn usn(frn: u64, parent: u64, reason: u32, name: &str) -> UsnRecord {
        UsnRecord { usn: 0, frn, frn_full: frn | (1 << 48), parent, reason, attributes: 0x20, name: name.into() }
    }

    struct FakeSource {
        batches: VecDeque<Result<(i64, Vec<UsnRecord>), ScanError>>,
        stats: HashMap<u64, FileStat>,
        stat_calls: Cell<u32>,
        probe: Option<Arc<RwLock<Tree>>>,
        lock_was_free: Cell<bool>,
        info: Option<Result<JournalInfo, ()>>,
    }

    impl FakeSource {
        fn with_info(mut self, info: Result<JournalInfo, ()>) -> FakeSource {
            self.info = Some(info);
            self
        }
        fn new(batches: Vec<Result<(i64, Vec<UsnRecord>), ScanError>>, stats: &[(u64, u64)]) -> FakeSource {
            FakeSource {
                batches: batches.into(),
                stats: stats
                    .iter()
                    .map(|&(f, a)| (f, FileStat { size: a, alloc: a, modified: 1, attributes: 0x20 }))
                    .collect(),
                stat_calls: Cell::new(0),
                probe: None,
                lock_was_free: Cell::new(false),
                info: None,
            }
        }
        fn probing(mut self, tree: Arc<RwLock<Tree>>) -> FakeSource {
            self.probe = Some(tree);
            self
        }
    }

    impl LiveSource for FakeSource {
        fn read(&mut self, _from: i64, _wait: bool) -> Result<(i64, Vec<UsnRecord>), ScanError> {
            self.batches.pop_front().unwrap_or(Ok((0, vec![])))
        }
        fn stat(&self, frn_full: u64) -> Option<FileStat> {
            self.stat_calls.set(self.stat_calls.get() + 1);
            if let Some(t) = &self.probe {
                self.lock_was_free.set(t.try_write().is_ok());
            }
            self.stats.get(&(frn_full & MASK)).copied()
        }
        fn journal_id(&self) -> u64 {
            7
        }
        fn journal_info(&self) -> Result<JournalInfo, ScanError> {
            match &self.info {
                Some(Ok(i)) => Ok(*i),
                _ => Err(ScanError::Io(std::io::ErrorKind::Other.into())),
            }
        }
    }

    #[test]
    fn pump_applies_batch() {
        let tree = RwLock::new(tree_with_dir(30, "Fotos"));
        let mut src = FakeSource::new(vec![Ok((200, vec![usn(70, 30, reason::FILE_CREATE | reason::CLOSE, "a.jpg")]))], &[(70, 8192)]);
        let (next, changed) = pump_once(&tree, &mut src, &mut Coalescer::default(), 100, false, &classifier()).unwrap();
        assert_eq!((next, changed), (200, true));
        assert_eq!(tree.read().unwrap().totals(0)[Category::Midia as usize], 8192);
    }

    #[test]
    fn pump_without_records_reports_unchanged() {
        let tree = RwLock::new(tree_with_dir(30, "Fotos"));
        let mut src = FakeSource::new(vec![Ok((100, vec![]))], &[]);
        assert_eq!(pump_once(&tree, &mut src, &mut Coalescer::default(), 100, true, &classifier()).unwrap(), (100, false));
    }

    #[test]
    fn stat_happens_outside_write_lock() {
        let tree = Arc::new(RwLock::new(tree_with_dir(30, "Fotos")));
        let mut src = FakeSource::new(vec![Ok((200, vec![usn(70, 30, reason::FILE_CREATE | reason::CLOSE, "a.jpg")]))], &[(70, 8192)])
            .probing(tree.clone());
        pump_once(&tree, &mut src, &mut Coalescer::default(), 100, false, &classifier()).unwrap();
        assert_eq!(src.stat_calls.get(), 1);
        assert!(src.lock_was_free.get(), "stat foi chamado com o lock de escrita preso");
    }

    #[test]
    fn journal_reset_triggers_rescan() {
        let tree = RwLock::new(tree_with_dir(30, "Fotos"));
        let mut src = FakeSource::new(vec![Err(ScanError::JournalReset)], &[]);
        assert!(matches!(
            pump_once(&tree, &mut src, &mut Coalescer::default(), 100, true, &classifier()),
            Err(ScanError::JournalReset)
        ));
    }

    #[test]
    fn error_with_new_journal_id_resets() {
        let src = FakeSource::new(vec![], &[]).with_info(Ok(JournalInfo { journal_id: 99, first_usn: 0, next_usn: 500 }));
        assert!(matches!(decide_on_error(&src, 100, 1), ErrorDecision::Reset));
    }

    #[test]
    fn error_after_truncation_resets() {
        let src = FakeSource::new(vec![], &[]).with_info(Ok(JournalInfo { journal_id: 7, first_usn: 150, next_usn: 500 }));
        assert!(matches!(decide_on_error(&src, 100, 1), ErrorDecision::Reset));
    }

    #[test]
    fn repeated_errors_fail_instead_of_going_silent() {
        let src = FakeSource::new(vec![], &[]).with_info(Ok(JournalInfo { journal_id: 7, first_usn: 0, next_usn: 500 }));
        assert!(matches!(decide_on_error(&src, 100, 1), ErrorDecision::Retry));
        assert!(matches!(decide_on_error(&src, 100, MAX_CONSECUTIVE_ERRORS), ErrorDecision::Fail));
    }

    #[test]
    fn retry_waits_out_journal_deletion() {
        let mut calls = 0;
        let r = with_retry(5, std::time::Duration::ZERO, || {
            calls += 1;
            if calls < 3 { Err(ScanError::JournalReset) } else { Ok(42) }
        });
        assert_eq!(r.unwrap(), 42);
        assert_eq!(calls, 3);
        let r: Result<(), ScanError> = with_retry(2, std::time::Duration::ZERO, || Err(ScanError::JournalReset));
        assert!(matches!(r, Err(ScanError::JournalReset)));
    }

    #[test]
    fn catch_up_reads_until_target() {
        let tree = RwLock::new(tree_with_dir(30, "Fotos"));
        let mut src = FakeSource::new(vec![Ok((150, vec![])), Ok((300, vec![])), Ok((300, vec![]))], &[]);
        assert_eq!(catch_up(&tree, &mut src, &mut Coalescer::default(), 100, 300, &classifier()).unwrap(), 300);
    }
}
