//! Liga a interface à fila: confere os itens, enfileira, registra no log e,
//! nos discos sem "Ao vivo", atualiza a árvore depois.

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use acervo_index::usn::Change;
use acervo_index::Tree;
use acervo_ops::{append_log, check_dest, check_item, log_line, OpKind, OpKindTag, Verdict};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::System::SystemInformation::GetLocalTime;

use crate::items::{resolve, ItemRef, Resolved};
use crate::queue::{Job, JobItem, OpRecord, QueueEvent};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum OpRequest {
    Recycle,
    DeletePermanent,
    Copy { dest: String },
    Move { dest: String },
    Rename { new_name: String },
}

impl OpRequest {
    fn into_kind(self) -> OpKind {
        match self {
            OpRequest::Recycle => OpKind::Recycle,
            OpRequest::DeletePermanent => OpKind::DeletePermanent,
            OpRequest::Copy { dest } => OpKind::Copy { dest: dest.into() },
            OpRequest::Move { dest } => OpKind::Move { dest: dest.into() },
            OpRequest::Rename { new_name } => OpKind::Rename { new_name },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    pub item: ItemRef,
    pub verdict: Verdict,
    pub bytes: u64,
    /// `None` = o item mudou desde que a lista foi mostrada.
    pub path: Option<String>,
}

pub const STALE: &str = "Algum item mudou desde que a lista foi mostrada. Atualize e tente de novo.";

fn tree_of(state: &AppState, drive: char) -> Option<(Arc<RwLock<Tree>>, bool)> {
    let drives = state.drives.read().unwrap();
    let slot = drives.get(&drive)?;
    Some((slot.tree.clone()?, slot.live.is_some()))
}

fn resolve_one(state: &AppState, r: &ItemRef) -> Option<Resolved> {
    let (tree, _) = tree_of(state, r.drive)?;
    let t = tree.read().unwrap();
    resolve(&t, r)
}

/// Categoria da pasta de destino, pela árvore do disco dela (se o Acervo a conhece).
fn dest_category(state: &AppState, dest: &Path) -> Option<acervo_classify::Category> {
    let drive = dest.to_string_lossy().chars().next()?.to_ascii_uppercase();
    let (tree, _) = tree_of(state, drive)?;
    let t = tree.read().unwrap();
    t.find(&dest.display().to_string()).map(|n| t.category(n))
}

/// Confere o que pode ser feito com cada item, sem fazer nada.
pub fn check(state: &AppState, items: &[ItemRef], op: OpKindTag) -> Vec<CheckResult> {
    items
        .iter()
        .map(|r| match resolve_one(state, r) {
            Some(res) => CheckResult {
                item: r.clone(),
                verdict: check_item(&res.facts, op, &state.protected),
                bytes: res.bytes,
                path: Some(res.facts.path.display().to_string()),
            },
            None => CheckResult { item: r.clone(), verdict: Verdict::Allowed, bytes: 0, path: None },
        })
        .collect()
}

/// Confere tudo de novo e põe na fila. Nada é feito se algum item mudou ou está bloqueado.
pub fn start(state: &AppState, items: &[ItemRef], req: OpRequest) -> Result<u64, String> {
    if items.is_empty() {
        return Err("Nenhum item selecionado.".into());
    }
    let kind = req.into_kind();
    if let OpKind::Copy { dest } | OpKind::Move { dest } = &kind {
        if let Verdict::Blocked { message } = check_dest(dest, dest_category(state, dest), &state.protected) {
            return Err(format!("{} — {}", dest.display(), message));
        }
    }
    let mut job_items = Vec::with_capacity(items.len());
    let mut bytes = 0;
    for r in items {
        let res = resolve_one(state, r).ok_or(STALE)?;
        if let Verdict::Blocked { message } = check_item(&res.facts, kind.tag(), &state.protected) {
            return Err(format!("{} — {}", res.facts.path.display(), message));
        }
        bytes += res.bytes;
        job_items.push(JobItem {
            path: res.facts.path,
            drive: r.drive,
            frn: r.frn,
            parent_frn: res.parent_frn,
            name: res.name,
            bytes: res.bytes,
        });
    }
    Ok(state.queue.push(Job { kind, items: job_items, bytes }))
}

pub const CHANGED_WHILE_QUEUED: &str = "Mudou desde que foi pedido; nada foi feito com ele.";

/// Na hora de executar: o caminho ainda leva ao mesmo registro da MFT?
pub fn precheck(state: &AppState, item: &JobItem) -> Option<String> {
    let Some((tree, _)) = tree_of(state, item.drive) else {
        return Some("O disco está sendo analisado de novo; tente depois.".into());
    };
    let t = tree.read().unwrap();
    match t.find(&item.path.display().to_string()) {
        Some(n) if t.node(n).frn == item.frn => None,
        _ => Some(CHANGED_WHILE_QUEUED.into()),
    }
}

fn local_stamp() -> (String, i32, u32) {
    let t = unsafe { GetLocalTime() };
    (
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond),
        t.wYear as i32,
        t.wMonth as u32,
    )
}

fn same_path(a: &str, b: &Path) -> bool {
    a.eq_ignore_ascii_case(&b.display().to_string())
}

fn write_log(dir: &Path, record: &OpRecord, job: &Job) {
    let (at, year, month) = local_stamp();
    let dest: Option<PathBuf> = match &job.kind {
        OpKind::Copy { dest } | OpKind::Move { dest } => Some(dest.clone()),
        OpKind::Rename { new_name } => job.items.first().and_then(|i| i.path.parent()).map(|p| p.join(new_name)),
        _ => None,
    };
    let outcome = record.outcome.as_ref();
    for item in &job.items {
        let failure = outcome.and_then(|o| o.failed.iter().find(|f| f.path.to_lowercase().starts_with(&item.path.display().to_string().to_lowercase())));
        let result = match (outcome, failure) {
            (None, _) => "erro: falha inesperada".to_string(),
            (_, Some(f)) => format!("erro: {}", f.message),
            _ => "ok".to_string(),
        };
        let _ = append_log(dir, year, month, &log_line(&at, record.kind, &item.path, dest.as_deref(), item.bytes, &result));
    }
}

/// Disco sem "Ao vivo": aplica na árvore o que a operação mudou (ou reanalisa).
fn refresh_trees(app: &AppHandle, record: &OpRecord, job: &Job) {
    let state = app.state::<AppState>();
    let Some(outcome) = record.outcome.as_ref() else { return };
    let ok = |i: &JobItem| outcome.succeeded.iter().any(|s| same_path(s, &i.path));
    let dest_drive = match &job.kind {
        OpKind::Copy { dest } | OpKind::Move { dest } => dest.to_string_lossy().chars().next().map(|c| c.to_ascii_uppercase()),
        _ => None,
    };
    let mut rescan: Vec<char> = Vec::new();
    if let Some(d) = dest_drive {
        if tree_of(&state, d).is_some_and(|(_, live)| !live) {
            rescan.push(d);
        }
    }
    let mut drives: Vec<char> = job.items.iter().map(|i| i.drive).collect();
    drives.sort();
    drives.dedup();
    for drive in drives {
        let Some((tree, live)) = tree_of(&state, drive) else { continue };
        if live || rescan.contains(&drive) {
            continue;
        }
        let changes: Vec<Change> = job
            .items
            .iter()
            .filter(|i| i.drive == drive && ok(i))
            .filter_map(|i| match &job.kind {
                OpKind::Recycle | OpKind::DeletePermanent | OpKind::Move { .. } => {
                    Some(Change::Deleted { frn: i.frn, parent: i.parent_frn, name: i.name.clone() })
                }
                OpKind::Rename { new_name } => Some(Change::Moved {
                    frn: i.frn,
                    frn_full: i.frn,
                    old_parent: i.parent_frn,
                    old_name: i.name.clone(),
                    new_parent: i.parent_frn,
                    new_name: new_name.clone(),
                    is_dir: false,
                    attributes: 0,
                }),
                OpKind::Copy { .. } => None,
            })
            .collect();
        if !changes.is_empty() {
            tree.write().unwrap().apply(&changes, &|_| None, &state.classifier);
            let _ = app.emit("index-updated", serde_json::json!({ "drive": drive }));
        }
    }
    for d in rescan {
        crate::scan::start_drive(app.clone(), d, true);
    }
}

/// Recebe os eventos da fila.
pub fn on_event(app: &AppHandle, ev: QueueEvent) {
    match ev {
        QueueEvent::Progress(r) => {
            let _ = app.emit("op-progress", r);
        }
        QueueEvent::Done(r, job) => {
            let state = app.state::<AppState>();
            if let Some(dir) = &state.snapshot_dir {
                write_log(dir, &r, &job);
            }
            refresh_trees(app, &r, &job);
            let _ = app.emit("op-done", r);
        }
    }
}
