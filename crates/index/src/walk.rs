//! Varredura alternativa, pasta por pasta (sem administrador ou fora do NTFS).

use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};

use acervo_classify::Classifier;
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDiskFreeSpaceW};

use crate::mft::{FileName, ParsedRecord};
use crate::{ScanError, ScanMode, ScanProgress, Tree, TreeBuilder};

const ROOT_RECORD: u64 = 5;
const FIRST_RECORD: u64 = 16;
const ATTR_DIRECTORY: u32 = 0x10;
const ATTR_REPARSE: u32 = 0x400;
const DEFAULT_CLUSTER: u64 = 4096;

fn wide(p: &Path) -> Vec<u16> {
    let mut s = p.to_string_lossy().into_owned();
    if !s.ends_with('\\') {
        s.push('\\');
    }
    s.encode_utf16().chain([0]).collect()
}

fn cluster_size(root: &Path) -> u64 {
    let w = wide(root);
    let (mut spc, mut bps, mut free, mut total) = (0u32, 0u32, 0u32, 0u32);
    let ok = unsafe {
        GetDiskFreeSpaceW(PCWSTR(w.as_ptr()), Some(&mut spc), Some(&mut bps), Some(&mut free), Some(&mut total))
    };
    match ok {
        Ok(()) if spc > 0 && bps > 0 => spc as u64 * bps as u64,
        _ => DEFAULT_CLUSTER,
    }
}

fn used_bytes(root: &Path) -> u64 {
    let w = wide(root);
    let (mut free, mut total) = (0u64, 0u64);
    match unsafe { GetDiskFreeSpaceExW(PCWSTR(w.as_ptr()), None, Some(&mut total), Some(&mut free)) } {
        Ok(()) => total.saturating_sub(free),
        Err(_) => 0,
    }
}

/// Varre `root` como se fosse a raiz do disco `drive`.
pub fn scan_walk(root: &Path, drive: char, classifier: &Classifier, progress: &dyn Fn(ScanProgress)) -> Result<Tree, ScanError> {
    scan_walk_with_cluster(root, drive, classifier, progress, cluster_size(root))
}

/// Igual a `scan_walk`, com o tamanho do cluster fixado (usado nos testes).
#[doc(hidden)]
pub fn scan_walk_with_cluster(
    root: &Path,
    drive: char,
    classifier: &Classifier,
    progress: &dyn Fn(ScanProgress),
    cluster: u64,
) -> Result<Tree, ScanError> {
    std::fs::read_dir(root)?; // falha cedo se a raiz não puder ser lida
    let used = used_bytes(root).max(1);
    let mut builder = TreeBuilder::new(drive, ROOT_RECORD, 1 << 16);
    builder.push(ParsedRecord {
        record_no: ROOT_RECORD,
        in_use: true,
        is_dir: true,
        ..Default::default()
    });

    let mut stack: Vec<(PathBuf, u64)> = vec![(root.to_path_buf(), ROOT_RECORD)];
    let mut next = FIRST_RECORD;
    let mut seen = 0u64;
    let mut last_pct = u32::MAX;
    while let Some((dir, parent)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            let attrs = meta.file_attributes();
            let is_dir = attrs & ATTR_DIRECTORY != 0;
            let size = if is_dir { 0 } else { meta.len() };
            let alloc = size.div_ceil(cluster) * cluster;
            let no = next;
            next += 1;
            builder.push(ParsedRecord {
                record_no: no,
                in_use: true,
                is_dir,
                names: vec![FileName {
                    parent,
                    name: entry.file_name().to_string_lossy().into_owned(),
                    namespace: 1,
                }],
                size,
                alloc,
                modified: meta.last_write_time() as i64,
                attributes: attrs,
                ..Default::default()
            });
            seen += alloc;
            if is_dir && attrs & ATTR_REPARSE == 0 {
                stack.push((entry.path(), no));
            }
        }
        let fraction = (seen as f64 / used as f64).min(0.99) as f32;
        let pct = (fraction * 100.0) as u32;
        if pct != last_pct {
            last_pct = pct;
            progress(ScanProgress { drive, fraction, mode: ScanMode::Walk });
        }
    }
    Ok(builder.finish(classifier))
}
