//! Execução pelas APIs do Explorer (`IFileOperation`), com progresso e falha por item.
//!
//! Cada operação roda numa thread própria com COM (STA); o progresso e o resultado
//! de cada item chegam por canal ao chamador, que repassa para `progress`.

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use serde::Serialize;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
use windows::Win32::UI::Shell::{
    FileOperation, IFileOperation, IFileOperationProgressSink, IFileOperationProgressSink_Impl, IShellItem, ILCreateFromPathW, ILFree,
    SHCreateItemFromParsingName, SHOpenFolderAndSelectItems, FILEOPERATION_FLAGS, FOFX_RECYCLEONDELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION,
    FOF_NOCONFIRMMKDIR, FOF_NOERRORUI, FOF_RENAMEONCOLLISION, FOF_SILENT, FOF_WANTNUKEWARNING, SIGDN_FILESYSPATH,
};
use windows::Win32::Foundation::HWND;
use std::sync::atomic::{AtomicIsize, Ordering};
use windows_core::{implement, Ref, HRESULT};

use crate::OpKindTag;

#[derive(Clone, Debug)]
pub enum OpKind {
    Recycle,
    DeletePermanent,
    Copy { dest: PathBuf },
    Move { dest: PathBuf },
    Rename { new_name: String },
}

impl OpKind {
    pub fn tag(&self) -> OpKindTag {
        match self {
            OpKind::Recycle => OpKindTag::Recycle,
            OpKind::DeletePermanent => OpKindTag::DeletePermanent,
            OpKind::Copy { .. } => OpKindTag::Copy,
            OpKind::Move { .. } => OpKindTag::Move,
            OpKind::Rename { .. } => OpKindTag::Rename,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpProgress {
    pub done: u64,
    pub total: u64,
    pub current: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFailure {
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpOutcome {
    pub succeeded: Vec<String>,
    pub failed: Vec<ItemFailure>,
    pub aborted: bool,
}

enum Msg {
    Progress(OpProgress),
    Item { path: String, hr: HRESULT },
}

#[implement(IFileOperationProgressSink)]
struct Sink {
    tx: mpsc::Sender<Msg>,
}

fn item_path(item: &Ref<'_, IShellItem>) -> String {
    item.as_ref()
        .and_then(|i| unsafe { i.GetDisplayName(SIGDN_FILESYSPATH) }.ok())
        .map(|p| unsafe {
            let s = p.to_string().unwrap_or_default();
            windows::Win32::System::Com::CoTaskMemFree(Some(p.0 as *const _));
            s
        })
        .unwrap_or_default()
}

impl Sink_Impl {
    fn report(&self, item: &Ref<'_, IShellItem>, hr: HRESULT) {
        let _ = self.tx.send(Msg::Item { path: item_path(item), hr });
    }
}

#[allow(non_snake_case)]
impl IFileOperationProgressSink_Impl for Sink_Impl {
    fn StartOperations(&self) -> windows_core::Result<()> {
        Ok(())
    }
    fn FinishOperations(&self, _hr: HRESULT) -> windows_core::Result<()> {
        Ok(())
    }
    fn PreRenameItem(&self, _: u32, _: Ref<'_, IShellItem>, _: &PCWSTR) -> windows_core::Result<()> {
        Ok(())
    }
    fn PostRenameItem(&self, _: u32, item: Ref<'_, IShellItem>, _: &PCWSTR, hr: HRESULT, _: Ref<'_, IShellItem>) -> windows_core::Result<()> {
        self.report(&item, hr);
        Ok(())
    }
    fn PreMoveItem(&self, _: u32, _: Ref<'_, IShellItem>, _: Ref<'_, IShellItem>, _: &PCWSTR) -> windows_core::Result<()> {
        Ok(())
    }
    fn PostMoveItem(&self, _: u32, item: Ref<'_, IShellItem>, _: Ref<'_, IShellItem>, _: &PCWSTR, hr: HRESULT, _: Ref<'_, IShellItem>) -> windows_core::Result<()> {
        self.report(&item, hr);
        Ok(())
    }
    fn PreCopyItem(&self, _: u32, _: Ref<'_, IShellItem>, _: Ref<'_, IShellItem>, _: &PCWSTR) -> windows_core::Result<()> {
        Ok(())
    }
    fn PostCopyItem(&self, _: u32, item: Ref<'_, IShellItem>, _: Ref<'_, IShellItem>, _: &PCWSTR, hr: HRESULT, _: Ref<'_, IShellItem>) -> windows_core::Result<()> {
        self.report(&item, hr);
        Ok(())
    }
    fn PreDeleteItem(&self, _: u32, _: Ref<'_, IShellItem>) -> windows_core::Result<()> {
        Ok(())
    }
    fn PostDeleteItem(&self, _: u32, item: Ref<'_, IShellItem>, hr: HRESULT, _: Ref<'_, IShellItem>) -> windows_core::Result<()> {
        self.report(&item, hr);
        Ok(())
    }
    fn PreNewItem(&self, _: u32, _: Ref<'_, IShellItem>, _: &PCWSTR) -> windows_core::Result<()> {
        Ok(())
    }
    fn PostNewItem(&self, _: u32, _: Ref<'_, IShellItem>, _: &PCWSTR, _: &PCWSTR, _: u32, _: HRESULT, _: Ref<'_, IShellItem>) -> windows_core::Result<()> {
        Ok(())
    }
    fn UpdateProgress(&self, total: u32, sofar: u32) -> windows_core::Result<()> {
        let _ = self.tx.send(Msg::Progress(OpProgress { done: sofar as u64, total: total as u64, current: None }));
        Ok(())
    }
    fn ResetTimer(&self) -> windows_core::Result<()> {
        Ok(())
    }
    fn PauseTimer(&self) -> windows_core::Result<()> {
        Ok(())
    }
    fn ResumeTimer(&self) -> windows_core::Result<()> {
        Ok(())
    }
}

/// O shell do Windows 11 aceita caminhos longos diretamente; o prefixo `\\?\` ele recusa.
fn shell_path(p: &Path) -> HSTRING {
    HSTRING::from(p.as_os_str())
}

/// `path` é `top` ou está dentro dele (sem diferenciar maiúsculas)?
fn is_within(path: &str, top: &str) -> bool {
    let (p, t) = (path.to_lowercase(), top.trim_end_matches('\\').to_lowercase());
    p == t || p.strip_prefix(&t).is_some_and(|rest| rest.starts_with('\\'))
}

fn shell_item(p: &Path) -> windows_core::Result<IShellItem> {
    unsafe { SHCreateItemFromParsingName(&shell_path(p), None) }
}

fn message(hr: HRESULT) -> String {
    let m = windows_core::Error::from(hr).message();
    let m = m.trim().trim_end_matches('.').to_string();
    if m.is_empty() { format!("erro 0x{:08X}", hr.0 as u32) } else { m }
}

fn flags(kind: &OpKind) -> FILEOPERATION_FLAGS {
    let base = FOF_NOCONFIRMMKDIR.0 | FOF_NOERRORUI.0 | FOF_SILENT.0;
    let extra = match kind {
        // FOF_WANTNUKEWARNING: o que não cabe na Lixeira (grande demais, disco sem Lixeira)
        // pede confirmação do Windows em vez de ser apagado de vez em silêncio.
        OpKind::Recycle => FOF_ALLOWUNDO.0 | FOFX_RECYCLEONDELETE.0 | FOF_NOCONFIRMATION.0 | FOF_WANTNUKEWARNING.0,
        OpKind::DeletePermanent => FOF_NOCONFIRMATION.0,
        OpKind::Copy { .. } | OpKind::Move { .. } => FOF_RENAMEONCOLLISION.0 | FOF_NOCONFIRMATION.0,
        // Sem RENAMEONCOLLISION: o nome que já existe é recusado antes (`run_operation`).
        OpKind::Rename { .. } => 0,
    };
    FILEOPERATION_FLAGS(base | extra)
}

/// Janela do Acervo, dona dos diálogos que o Windows ainda mostra (aviso da Lixeira).
static OWNER: AtomicIsize = AtomicIsize::new(0);

/// Registra a janela principal para os diálogos do Windows aparecerem na frente dela.
pub fn set_owner_window(hwnd: isize) {
    OWNER.store(hwnd, Ordering::Relaxed);
}

/// Monta e executa a operação nesta thread (que precisa ter COM inicializado).
fn perform(kind: &OpKind, items: &[PathBuf], tx: mpsc::Sender<Msg>, failed: &mut Vec<ItemFailure>) -> windows_core::Result<bool> {
    let op: IFileOperation = unsafe { CoCreateInstance(&FileOperation, None, CLSCTX_ALL)? };
    unsafe { op.SetOperationFlags(flags(kind))? };
    let owner = OWNER.load(Ordering::Relaxed);
    if owner != 0 {
        let _ = unsafe { op.SetOwnerWindow(HWND(owner as *mut _)) };
    }
    let sink: IFileOperationProgressSink = Sink { tx }.into();
    let cookie = unsafe { op.Advise(&sink)? };
    let dest = match kind {
        OpKind::Copy { dest } | OpKind::Move { dest } => Some(shell_item(dest)?),
        _ => None,
    };
    let mut queued = 0;
    for p in items {
        let item = match shell_item(p) {
            Ok(i) => i,
            Err(e) => {
                failed.push(ItemFailure { path: p.display().to_string(), message: message(e.code()) });
                continue;
            }
        };
        let r = unsafe {
            match kind {
                OpKind::Recycle | OpKind::DeletePermanent => op.DeleteItem(&item, None),
                OpKind::Copy { .. } => op.CopyItem(&item, dest.as_ref().unwrap(), PCWSTR::null(), None),
                OpKind::Move { .. } => op.MoveItem(&item, dest.as_ref().unwrap(), PCWSTR::null(), None),
                OpKind::Rename { new_name } => op.RenameItem(&item, &HSTRING::from(new_name.as_str()), None),
            }
        };
        match r {
            Ok(()) => queued += 1,
            Err(e) => failed.push(ItemFailure { path: p.display().to_string(), message: message(e.code()) }),
        }
    }
    if queued > 0 {
        // Falhas de itens chegam pelo sink; o erro geral aqui só repete a última.
        let _ = unsafe { op.PerformOperations() };
    }
    let aborted = unsafe { op.GetAnyOperationsAborted() }.map(|b| b.as_bool()).unwrap_or(false);
    let _ = unsafe { op.Unadvise(cookie) };
    Ok(aborted)
}

/// Executa a operação sobre os itens. Itens que falham não impedem os outros.
pub fn run_operation(kind: &OpKind, items: &[PathBuf], progress: &dyn Fn(OpProgress)) -> OpOutcome {
    if let OpKind::Rename { new_name } = kind {
        let refuse = |message: &str| OpOutcome {
            succeeded: vec![],
            failed: items.iter().map(|p| ItemFailure { path: p.display().to_string(), message: message.into() }).collect(),
            aborted: false,
        };
        if items.len() != 1 {
            return refuse("Renomear funciona com um item por vez");
        }
        // Só mudar maiúsculas é o mesmo item; outro nome que já existe é recusado.
        let target = items[0].with_file_name(new_name);
        let same = target.as_os_str().to_string_lossy().to_lowercase() == items[0].as_os_str().to_string_lossy().to_lowercase();
        if !same && target.symlink_metadata().is_ok() {
            return refuse("Já existe um item com esse nome nesta pasta");
        }
    }
    let (tx, rx) = mpsc::channel();
    let (kind2, items2) = (kind.clone(), items.to_vec());
    let worker = std::thread::spawn(move || {
        let mut failed = Vec::new();
        let init = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
        let aborted = match perform(&kind2, &items2, tx, &mut failed) {
            Ok(a) => a,
            Err(e) => {
                for p in &items2 {
                    failed.push(ItemFailure { path: p.display().to_string(), message: message(e.code()) });
                }
                false
            }
        };
        if init.is_ok() {
            unsafe { CoUninitialize() };
        }
        (failed, aborted)
    });

    // O Explorer avisa cada arquivo de dentro de uma pasta; o resultado é por item pedido.
    let mut failed: Vec<ItemFailure> = Vec::new();
    let mut reports: Vec<(String, bool)> = Vec::new();
    for msg in rx {
        match msg {
            Msg::Progress(p) => progress(p),
            Msg::Item { path, hr } => {
                if hr.is_err() {
                    failed.push(ItemFailure { path: path.clone(), message: message(hr) });
                }
                reports.push((path, hr.is_ok()));
            }
        }
    }
    match worker.join() {
        Ok((early, aborted)) => {
            failed.splice(0..0, early);
            summarize(items, &reports, failed, Some(aborted))
        }
        Err(_) => summarize(items, &reports, failed, None),
    }
}

/// Resultado por item pedido. `aborted: None` = a thread da operação entrou em pânico.
///
/// Sem interrupção, um item deu certo se nada dentro dele falhou. Interrompida, só conta
/// como feito o que o Windows confirmou item a item; o resto nem foi tentado.
fn summarize(items: &[PathBuf], reports: &[(String, bool)], mut failed: Vec<ItemFailure>, aborted: Option<bool>) -> OpOutcome {
    let Some(aborted) = aborted else {
        let failed = items
            .iter()
            .map(|p| ItemFailure { path: p.display().to_string(), message: "Falha inesperada ao executar a operação".into() })
            .collect();
        return OpOutcome { succeeded: vec![], failed, aborted: true };
    };
    let mut succeeded = Vec::new();
    for p in items.iter().map(|p| p.display().to_string()) {
        let failed_inside = failed.iter().any(|f| is_within(&f.path, &p));
        let confirmed = reports.iter().any(|(r, ok)| *ok && r.eq_ignore_ascii_case(&p));
        if !failed_inside && (!aborted || confirmed) {
            succeeded.push(p);
        } else if !failed_inside {
            failed.push(ItemFailure { path: p, message: "Não foi feito: a operação foi interrompida".into() });
        }
    }
    OpOutcome { succeeded, failed, aborted }
}

/// Abre a pasta do item no Explorer com ele selecionado.
pub fn reveal(path: &Path) -> std::io::Result<()> {
    let p = path.to_path_buf();
    std::thread::spawn(move || unsafe {
        let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
        let pidl = ILCreateFromPathW(&shell_path(&p));
        let r = if pidl.is_null() {
            Err(std::io::Error::from(std::io::ErrorKind::NotFound))
        } else {
            let r = SHOpenFolderAndSelectItems(pidl, None, 0).map_err(|e| std::io::Error::other(e.message()));
            ILFree(Some(pidl));
            r
        };
        if init.is_ok() {
            CoUninitialize();
        }
        r
    })
    .join()
    .unwrap_or_else(|_| Err(std::io::Error::other("falha ao abrir o Explorer")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recycle_warns_before_destroying_what_does_not_fit() {
        // Sem isto, FOF_NOCONFIRMATION responde "sim" sozinho quando o item não cabe na Lixeira.
        assert_ne!(flags(&OpKind::Recycle).0 & FOF_WANTNUKEWARNING.0, 0);
        assert_eq!(flags(&OpKind::Rename { new_name: "x".into() }).0 & FOF_RENAMEONCOLLISION.0, 0);
    }

    fn items(v: &[&str]) -> Vec<PathBuf> {
        v.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn aborted_counts_only_items_the_shell_confirmed() {
        let reports = vec![(r"C:\x\a".to_string(), true)];
        let o = summarize(&items(&[r"C:\x\a", r"C:\x\b"]), &reports, vec![], Some(true));
        assert_eq!(o.succeeded, vec![r"C:\x\a".to_string()]);
        assert!(o.aborted);
        assert_eq!(o.failed.len(), 1, "o item que nem foi tentado não é sucesso");
        assert_eq!(o.failed[0].path, r"C:\x\b");
    }

    #[test]
    fn worker_panic_marks_everything_failed() {
        let o = summarize(&items(&[r"C:\x\a", r"C:\x\b"]), &[], vec![], None);
        assert!(o.succeeded.is_empty());
        assert_eq!(o.failed.len(), 2);
    }

    #[test]
    fn normal_run_infers_success_from_missing_failures() {
        let reports = vec![(r"C:\x\pasta\f".to_string(), false)];
        let failed = vec![ItemFailure { path: r"C:\x\pasta\f".into(), message: "em uso".into() }];
        let o = summarize(&items(&[r"C:\x\pasta", r"C:\x\b"]), &reports, failed, Some(false));
        assert_eq!(o.succeeded, vec![r"C:\x\b".to_string()]);
        assert!(!o.aborted);
    }
}
