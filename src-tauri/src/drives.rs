//! Discos fixos da máquina e espaço livre.

use acervo_index::DiskSpace;
use serde::Serialize;
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW};

const DRIVE_FIXED: u32 = 3;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    pub letter: char,
    pub label: String,
    pub filesystem: String,
    pub total: u64,
    pub free: u64,
}

fn root(letter: char) -> Vec<u16> {
    format!("{letter}:\\").encode_utf16().chain([0]).collect()
}

fn utf16_until_nul(buf: &[u16]) -> String {
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n])
}

pub fn disk_space(letter: char) -> DiskSpace {
    let r = root(letter);
    let (mut free, mut total) = (0u64, 0u64);
    let _ = unsafe { GetDiskFreeSpaceExW(PCWSTR(r.as_ptr()), None, Some(&mut total), Some(&mut free)) };
    DiskSpace { total, free }
}

pub fn fixed_drives() -> Vec<DriveInfo> {
    let mask = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| (b'A' + i) as char)
        .filter_map(|letter| {
            let r = root(letter);
            if unsafe { GetDriveTypeW(PCWSTR(r.as_ptr())) } != DRIVE_FIXED {
                return None;
            }
            let (mut label, mut fs) = ([0u16; 261], [0u16; 32]);
            let _ = unsafe {
                GetVolumeInformationW(PCWSTR(r.as_ptr()), Some(&mut label), None, None, None, Some(&mut fs))
            };
            let space = disk_space(letter);
            Some(DriveInfo {
                letter,
                label: utf16_until_nul(&label),
                filesystem: utf16_until_nul(&fs),
                total: space.total,
                free: space.free,
            })
        })
        .collect()
}
