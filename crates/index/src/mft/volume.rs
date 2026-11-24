//! Leitura da `$MFT` direto do volume (exige administrador).

use std::fs::{File, OpenOptions};
use std::os::windows::fs::{FileExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;

use acervo_classify::Classifier;
use rayon::prelude::*;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::{GetVolumeInformationW, FILE_SHARE_READ, FILE_SHARE_WRITE};
use windows::Win32::System::Ioctl::{FSCTL_GET_NTFS_VOLUME_DATA, NTFS_VOLUME_DATA_BUFFER};
use windows::Win32::System::IO::DeviceIoControl;

use super::{apply_fixup, data_runs_of_record, parse_record, ParsedRecord};
use crate::{ScanError, ScanMode, ScanProgress, Tree, TreeBuilder};

const CHUNK: u64 = 4 << 20;
const ROOT_RECORD: u64 = 5;

/// Erro se mais de 0,1% dos registros forem ilegíveis.
pub(crate) fn check_bad_ratio(skipped: u64, total: u64) -> Result<(), ScanError> {
    if skipped * 1000 > total {
        return Err(ScanError::TooManyBadRecords { skipped, total });
    }
    Ok(())
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

/// Nome do sistema de arquivos do disco (ex.: "NTFS").
pub fn filesystem_name(drive: char) -> Result<String, ScanError> {
    let root = wide(&format!("{drive}:\\"));
    let mut fs = [0u16; 32];
    unsafe { GetVolumeInformationW(PCWSTR(root.as_ptr()), None, None, None, None, Some(&mut fs)) }
        .map_err(|e| ScanError::Io(std::io::Error::from_raw_os_error(e.code().0 & 0xFFFF)))?;
    let n = fs.iter().position(|&c| c == 0).unwrap_or(fs.len());
    Ok(String::from_utf16_lossy(&fs[..n]))
}

pub(crate) fn open_volume(drive: char) -> Result<File, ScanError> {
    Ok(OpenOptions::new()
        .read(true)
        .share_mode((FILE_SHARE_READ | FILE_SHARE_WRITE).0)
        .open(format!(r"\\.\{drive}:"))?)
}

pub(crate) fn volume_data(file: &File) -> Result<NTFS_VOLUME_DATA_BUFFER, ScanError> {
    let mut out = NTFS_VOLUME_DATA_BUFFER::default();
    let mut returned = 0u32;
    unsafe {
        DeviceIoControl(
            HANDLE(file.as_raw_handle()),
            FSCTL_GET_NTFS_VOLUME_DATA,
            None,
            0,
            Some(&mut out as *mut _ as *mut _),
            std::mem::size_of::<NTFS_VOLUME_DATA_BUFFER>() as u32,
            Some(&mut returned),
            None,
        )
    }
    .map_err(|e| ScanError::from(std::io::Error::from_raw_os_error(e.code().0 & 0xFFFF)))?;
    Ok(out)
}

fn read_exact_at(file: &File, buf: &mut [u8], mut offset: u64) -> Result<(), ScanError> {
    let mut done = 0;
    while done < buf.len() {
        let n = file.seek_read(&mut buf[done..], offset)?;
        if n == 0 {
            return Err(ScanError::Io(std::io::ErrorKind::UnexpectedEof.into()));
        }
        done += n;
        offset += n as u64;
    }
    Ok(())
}

fn align_up(n: u64, to: u64) -> u64 {
    n.div_ceil(to) * to
}

/// Resultado do processamento de um registro.
enum Parsed {
    Record(ParsedRecord),
    Empty,
    Bad,
}

fn parse_one(buf: &mut [u8], record_no: u64, bps: usize) -> Parsed {
    if buf[0..4] == [0, 0, 0, 0] {
        return Parsed::Empty;
    }
    if apply_fixup(buf, bps).is_err() {
        return Parsed::Bad;
    }
    match parse_record(buf, record_no) {
        Ok(r) => Parsed::Record(r),
        Err(_) => Parsed::Bad,
    }
}

/// Lê a MFT inteira do disco e monta a árvore.
pub fn scan_ntfs(drive: char, classifier: &Classifier, progress: &dyn Fn(ScanProgress)) -> Result<Tree, ScanError> {
    let file = open_volume(drive)?;
    if !filesystem_name(drive)?.eq_ignore_ascii_case("NTFS") {
        return Err(ScanError::NotNtfs);
    }
    let vd = volume_data(&file)?;
    let bps = vd.BytesPerSector as u64;
    let cluster = vd.BytesPerCluster as u64;
    let rec_size = vd.BytesPerFileRecordSegment as u64;
    let valid = vd.MftValidDataLength as u64;
    if bps == 0 || cluster == 0 || rec_size == 0 {
        return Err(ScanError::NotNtfs);
    }

    // Registro 0 = a própria $MFT: dele saem os trechos onde ela está.
    let mut first = vec![0u8; align_up(rec_size, bps) as usize];
    read_exact_at(&file, &mut first, vd.MftStartLcn as u64 * cluster)?;
    let first = &mut first[..rec_size as usize];
    apply_fixup(first, bps as usize).map_err(|_| ScanError::Fragmented)?;
    let runs = data_runs_of_record(first).map_err(|_| ScanError::Fragmented)?;
    let covered: u64 = runs.iter().map(|r| r.clusters * cluster).sum();
    if covered < valid {
        return Err(ScanError::Fragmented);
    }

    let total_records = valid / rec_size;
    let mut builder = TreeBuilder::new(drive, ROOT_RECORD, total_records as usize);
    let mut buf = vec![0u8; CHUNK as usize];
    let mut read = 0u64;
    let mut record_no = 0u64;
    let mut last_pct = u32::MAX;
    let mut skipped = 0u64;

    'runs: for run in &runs {
        let mut offset = run.lcn * cluster;
        let mut remaining = run.clusters * cluster;
        while remaining > 0 {
            if read >= valid {
                break 'runs;
            }
            let want = remaining.min(CHUNK).min(valid - read);
            let len = align_up(want, bps).min(remaining);
            let chunk = &mut buf[..len as usize];
            read_exact_at(&file, chunk, offset)?;
            let usable = (want / rec_size * rec_size) as usize;
            let base = record_no;
            let parsed: Vec<Parsed> = chunk[..usable]
                .par_chunks_mut(rec_size as usize)
                .enumerate()
                .map(|(k, rec)| parse_one(rec, base + k as u64, bps as usize))
                .collect();
            for p in parsed {
                match p {
                    Parsed::Record(r) => builder.push(r),
                    Parsed::Empty => {}
                    Parsed::Bad => {
                        builder.skip();
                        skipped += 1;
                    }
                }
            }
            record_no += (usable as u64) / rec_size;
            read += want;
            offset += len;
            remaining -= len;
            let pct = (read * 100 / valid.max(1)) as u32;
            if pct != last_pct {
                last_pct = pct;
                progress(ScanProgress { drive, fraction: read as f32 / valid as f32, mode: ScanMode::Mft });
            }
        }
    }

    check_bad_ratio(skipped, total_records)?;
    Ok(builder.finish(classifier))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScanError;

    #[test]
    fn too_many_bad_records_is_error() {
        assert!(check_bad_ratio(1, 1000).is_ok());
        assert!(matches!(
            check_bad_ratio(2, 1000),
            Err(ScanError::TooManyBadRecords { skipped: 2, total: 1000 })
        ));
    }
}
