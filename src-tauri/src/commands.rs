//! Comandos chamados pela interface.

use acervo_classify::Category;
use acervo_index::{children_in, summary, system_breakdown as breakdown, BreakdownItem, DriveSummary, FileFilter, ItemRow, Page, SortKey};
use acervo_ops::OpKindTag;

use crate::items::ItemRef;
use crate::operations::{self, CheckResult, OpRequest};
use crate::queue::OpRecord;
use tauri::{AppHandle, State};

use crate::drives::{disk_space, fixed_drives};
use crate::state::{AppState, DriveStatus, DriveView};

#[tauri::command]
pub fn list_drives(state: State<'_, AppState>) -> Vec<DriveView> {
    // Atualiza o espaço livre a cada consulta (lido antes de pegar o lock).
    let fresh = fixed_drives();
    let mut drives = state.drives.write().unwrap();
    for info in fresh {
        if let Some(slot) = drives.get_mut(&info.letter) {
            slot.info = info;
        }
    }
    drives
        .values()
        .map(|s| DriveView { info: s.info.clone(), status: s.status.clone() })
        .collect()
}

#[tauri::command]
pub fn drive_summary(state: State<'_, AppState>, drive: char) -> Option<DriveSummary> {
    let (tree, mode) = {
        let drives = state.drives.read().unwrap();
        let slot = drives.get(&drive)?;
        let mode = match slot.status {
            DriveStatus::Ready { mode, .. } | DriveStatus::Scanning { mode, .. } => mode,
            DriveStatus::Failed { .. } => return None,
        };
        (slot.tree.clone()?, mode)
    };
    let space = disk_space(drive);
    let t = tree.read().unwrap();
    Some(summary(&t, space, mode))
}

#[tauri::command]
pub fn system_breakdown(state: State<'_, AppState>, drive: char) -> Option<Vec<BreakdownItem>> {
    let tree = state.drives.read().unwrap().get(&drive)?.tree.clone()?;
    let t = tree.read().unwrap();
    Some(breakdown(&t))
}

/// "Tentar de novo": analisa o disco inteiro de novo, sem usar o snapshot.
#[tauri::command]
pub fn rescan_drive(app: AppHandle, drive: char) {
    crate::scan::start_drive(app, drive, true);
}

#[tauri::command]
pub fn app_is_elevated(state: State<'_, AppState>) -> bool {
    state.elevated
}

fn tree_of(state: &AppState, drive: char) -> Option<std::sync::Arc<std::sync::RwLock<acervo_index::Tree>>> {
    state.drives.read().unwrap().get(&drive)?.tree.clone()
}

/// Filhos de uma pasta (`node` ausente = raiz do disco) que ocupam algo na categoria.
#[tauri::command]
pub fn category_children(
    state: State<'_, AppState>,
    drive: char,
    node: Option<u32>,
    path: Option<String>,
    category: Option<Category>,
) -> Option<Vec<ItemRow>> {
    let tree = tree_of(&state, drive)?;
    let t = tree.read().unwrap();
    // `None` faz a tela voltar para a pasta acima mais próxima que ainda existe.
    if let (Some(n), Some(p)) = (node, path.as_deref()) {
        if !crate::items::folder_matches(&t, n, p) {
            return None;
        }
    }
    children_in(&t, node.unwrap_or(t.root), category)
}

/// Uma página da lista de arquivos do disco.
#[tauri::command]
pub fn query_files(
    state: State<'_, AppState>,
    drive: char,
    filter: FileFilter,
    sort: SortKey,
    offset: usize,
    limit: usize,
) -> Option<Page> {
    let tree = tree_of(&state, drive)?;
    let t = tree.read().unwrap();
    let mut caches = state.caches.lock().unwrap();
    let cache = caches.entry(drive).or_default();
    Some(acervo_index::query_files(&t, cache, &filter, sort, offset, limit))
}

/// O que pode ser feito com cada item (sem fazer nada).
#[tauri::command]
pub fn check_items(state: State<'_, AppState>, items: Vec<ItemRef>, op: OpKindTag) -> Vec<CheckResult> {
    operations::check(&state, &items, op)
}

/// Põe a operação na fila; devolve o id ou o motivo da recusa.
#[tauri::command]
pub fn start_operation(state: State<'_, AppState>, items: Vec<ItemRef>, op: OpRequest) -> Result<u64, String> {
    operations::start(&state, &items, op)
}

#[tauri::command]
pub fn reveal_item(state: State<'_, AppState>, item: ItemRef) -> Result<(), String> {
    let tree = tree_of(&state, item.drive).ok_or(operations::STALE)?;
    let path = {
        let t = tree.read().unwrap();
        crate::items::resolve(&t, &item).ok_or(operations::STALE)?.facts.path
    };
    acervo_ops::reveal(&path).map_err(|e| format!("Não foi possível abrir o Explorer: {e}"))
}

#[tauri::command]
pub fn list_operations(state: State<'_, AppState>) -> Vec<OpRecord> {
    state.queue.history()
}
