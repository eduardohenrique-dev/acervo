//! Fila de operações de arquivo: uma por vez, numa thread própria, com histórico.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use acervo_ops::{ItemFailure, OpKind, OpKindTag, OpOutcome, OpProgress};
use serde::Serialize;

/// Quem executa de fato (o real é `acervo_ops::run_operation`; nos testes, um falso).
pub type Executor = Arc<dyn Fn(&OpKind, &[PathBuf], &dyn Fn(OpProgress)) -> OpOutcome + Send + Sync>;

/// Confere um item na hora de executar: `Some(motivo)` se ele mudou desde que foi pedido.
pub type Precheck = Arc<dyn Fn(&JobItem) -> Option<String> + Send + Sync>;

const HISTORY: usize = 50;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

/// Um item da operação, com o que é preciso para atualizar a árvore depois.
#[derive(Clone, Debug)]
pub struct JobItem {
    pub path: PathBuf,
    pub drive: char,
    pub frn: u64,
    pub parent_frn: u64,
    pub name: String,
    pub bytes: u64,
}

#[derive(Clone, Debug)]
pub struct Job {
    pub kind: OpKind,
    pub items: Vec<JobItem>,
    pub bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum OpState {
    Queued,
    Running { done: u64, total: u64 },
    Done,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpRecord {
    pub id: u64,
    pub kind: OpKindTag,
    pub count: usize,
    pub bytes: u64,
    pub state: OpState,
    pub outcome: Option<OpOutcome>,
    pub started_at: i64,
}

pub enum QueueEvent {
    Progress(OpRecord),
    Done(OpRecord, Job),
}

type Listener = Arc<dyn Fn(QueueEvent) + Send + Sync>;

struct Inner {
    next_id: u64,
    history: VecDeque<OpRecord>,
    pending: usize,
}

pub struct OpQueue {
    inner: Arc<(Mutex<Inner>, Condvar)>,
    tx: mpsc::Sender<(u64, Job)>,
}

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn update(inner: &(Mutex<Inner>, Condvar), id: u64, f: impl FnOnce(&mut OpRecord)) -> Option<OpRecord> {
    let mut g = inner.0.lock().unwrap();
    let r = g.history.iter_mut().find(|r| r.id == id)?;
    f(r);
    Some(r.clone())
}

impl OpQueue {
    pub fn new(executor: Executor, precheck: Precheck, listener: Listener) -> OpQueue {
        let inner = Arc::new((Mutex::new(Inner { next_id: 1, history: VecDeque::new(), pending: 0 }), Condvar::new()));
        let (tx, rx) = mpsc::channel::<(u64, Job)>();
        let worker_inner = inner.clone();
        std::thread::spawn(move || {
            for (id, job) in rx {
                if let Some(r) = update(&worker_inner, id, |r| r.state = OpState::Running { done: 0, total: 0 }) {
                    listener(QueueEvent::Progress(r));
                }
                // O que mudou enquanto esperava na fila fica de fora e conta como falha.
                let mut stale: Vec<ItemFailure> = Vec::new();
                let mut paths: Vec<PathBuf> = Vec::with_capacity(job.items.len());
                for i in &job.items {
                    match precheck(i) {
                        Some(message) => stale.push(ItemFailure { path: i.path.display().to_string(), message }),
                        None => paths.push(i.path.clone()),
                    }
                }
                let last = std::cell::Cell::new(Instant::now() - PROGRESS_INTERVAL);
                let progress = |p: OpProgress| {
                    if last.get().elapsed() < PROGRESS_INTERVAL {
                        return;
                    }
                    last.set(Instant::now());
                    if let Some(r) = update(&worker_inner, id, |r| r.state = OpState::Running { done: p.done, total: p.total }) {
                        listener(QueueEvent::Progress(r));
                    }
                };
                let outcome = if paths.is_empty() {
                    Ok(OpOutcome { succeeded: vec![], failed: vec![], aborted: false })
                } else {
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| executor(&job.kind, &paths, &progress)))
                }
                .map(|mut o| {
                    o.failed.extend(stale);
                    o
                });
                let record = update(&worker_inner, id, |r| match outcome {
                    Ok(o) => {
                        r.state = OpState::Done;
                        r.outcome = Some(o);
                    }
                    Err(_) => r.state = OpState::Failed,
                });
                {
                    let mut g = worker_inner.0.lock().unwrap();
                    g.pending -= 1;
                    worker_inner.1.notify_all();
                }
                if let Some(r) = record {
                    listener(QueueEvent::Done(r, job));
                }
            }
        });
        OpQueue { inner, tx }
    }

    #[cfg(test)]
    pub fn with_executor(executor: Executor) -> OpQueue {
        OpQueue::new(executor, Arc::new(|_| None), Arc::new(|_| {}))
    }

    /// Põe na fila e devolve o id.
    pub fn push(&self, job: Job) -> u64 {
        let id = {
            let mut g = self.inner.0.lock().unwrap();
            let id = g.next_id;
            g.next_id += 1;
            g.pending += 1;
            g.history.push_back(OpRecord {
                id,
                kind: job.kind.tag(),
                count: job.items.len(),
                bytes: job.bytes,
                state: OpState::Queued,
                outcome: None,
                started_at: now_secs(),
            });
            while g.history.len() > HISTORY {
                g.history.pop_front();
            }
            id
        };
        let _ = self.tx.send((id, job));
        id
    }

    /// Operações desta sessão, da mais antiga para a mais nova.
    pub fn history(&self) -> Vec<OpRecord> {
        self.inner.0.lock().unwrap().history.iter().cloned().collect()
    }

    #[cfg(test)]
    pub fn wait_idle(&self) {
        let mut g = self.inner.0.lock().unwrap();
        while g.pending > 0 {
            g = self.inner.1.wait(g).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use acervo_ops::{ItemFailure, OpKind, OpOutcome};

    use super::*;

    fn fake_exec(results: &[(&'static str, usize)]) -> Executor {
        let results: Vec<(&'static str, usize)> = results.to_vec();
        let calls = Arc::new(std::sync::Mutex::new(0usize));
        Arc::new(move |_kind: &OpKind, items: &[PathBuf], progress: &dyn Fn(acervo_ops::OpProgress)| {
            let mut c = calls.lock().unwrap();
            let (msg, fails) = results[*c % results.len()];
            *c += 1;
            progress(acervo_ops::OpProgress { done: 1, total: 2, current: None });
            let failed: Vec<ItemFailure> =
                items.iter().take(fails).map(|p| ItemFailure { path: p.display().to_string(), message: msg.into() }).collect();
            let succeeded = items.iter().skip(fails).map(|p| p.display().to_string()).collect();
            OpOutcome { succeeded, failed, aborted: false }
        })
    }

    fn job(kind: OpKind, n: usize, bytes: u64) -> Job {
        Job {
            kind,
            items: (0..n)
                .map(|i| JobItem { path: PathBuf::from(format!(r"C:\x\{i}")), drive: 'C', frn: 100 + i as u64, parent_frn: 5, name: format!("{i}"), bytes: 0 })
                .collect(),
            bytes,
        }
    }

    #[test]
    fn queue_runs_in_order_and_records_history() {
        let q = OpQueue::with_executor(fake_exec(&[("ok", 0), ("ok", 0)]));
        let a = q.push(job(OpKind::DeletePermanent, 2, 100));
        let b = q.push(job(OpKind::Copy { dest: r"D:\x".into() }, 1, 50));
        q.wait_idle();
        let h = q.history();
        assert_eq!(h.iter().map(|r| r.id).collect::<Vec<_>>(), vec![a, b]);
        assert!(h.iter().all(|r| matches!(r.state, OpState::Done)));
        assert_eq!(h[0].count, 2);
        assert_eq!(h[0].bytes, 100);
    }

    #[test]
    fn failures_are_kept_in_outcome() {
        let q = OpQueue::with_executor(fake_exec(&[("falhou: acesso negado", 1)]));
        q.push(job(OpKind::Recycle, 2, 10));
        q.wait_idle();
        let r = &q.history()[0];
        assert_eq!(r.outcome.as_ref().unwrap().failed.len(), 1);
        assert!(matches!(r.state, OpState::Done));
    }

    #[test]
    fn items_changed_while_waiting_are_not_executed() {
        // A fila pode esperar minutos por uma cópia longa: o que mudou nesse tempo não pode ser tocado.
        let seen = Arc::new(std::sync::Mutex::new(Vec::<PathBuf>::new()));
        let seen2 = seen.clone();
        let exec: Executor = Arc::new(move |_k: &OpKind, items: &[PathBuf], _p: &dyn Fn(acervo_ops::OpProgress)| {
            seen2.lock().unwrap().extend(items.iter().cloned());
            OpOutcome { succeeded: items.iter().map(|p| p.display().to_string()).collect(), failed: vec![], aborted: false }
        });
        let precheck: Precheck = Arc::new(|i: &JobItem| (i.name == "1").then(|| "mudou desde que foi pedido".to_string()));
        let q = OpQueue::new(exec, precheck, Arc::new(|_| {}));
        q.push(job(OpKind::Recycle, 3, 30));
        q.wait_idle();
        assert_eq!(*seen.lock().unwrap(), vec![PathBuf::from(r"C:\x\0"), PathBuf::from(r"C:\x\2")]);
        let o = q.history()[0].outcome.clone().unwrap();
        assert_eq!(o.succeeded.len(), 2);
        assert_eq!(o.failed.len(), 1);
        assert_eq!((o.failed[0].path.as_str(), o.failed[0].message.as_str()), (r"C:\x\1", "mudou desde que foi pedido"));
    }

    #[test]
    fn history_keeps_last_50() {
        let q = OpQueue::with_executor(fake_exec(&[("ok", 0)]));
        for _ in 0..60 {
            q.push(job(OpKind::DeletePermanent, 1, 1));
        }
        q.wait_idle();
        let h = q.history();
        assert_eq!(h.len(), 50);
        assert_eq!(h.last().unwrap().id, 60);
    }
}
