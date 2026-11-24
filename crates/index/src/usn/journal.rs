//! Acesso ao USN Journal de um volume NTFS (exige administrador).

use std::fs::File;
use std::os::windows::io::AsRawHandle;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::{
    FileBasicInfo, FileStandardInfo, GetFileInformationByHandleEx, OpenFileById, FILE_BASIC_INFO, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_ID_DESCRIPTOR, FILE_ID_DESCRIPTOR_0, FILE_ID_TYPE, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, FILE_STANDARD_INFO,
};
use windows::Win32::System::Ioctl::{
    CREATE_USN_JOURNAL_DATA, FSCTL_CREATE_USN_JOURNAL, FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL, READ_USN_JOURNAL_DATA_V0,
    USN_JOURNAL_DATA_V0,
};
use windows::Win32::System::IO::DeviceIoControl;

use super::record::{parse_read_buffer, UsnRecord};
use super::JournalInfo;
use crate::mft::volume::{open_volume, volume_data};
use crate::{FileStat, ScanError};

const ERROR_JOURNAL_DELETE_IN_PROGRESS: i32 = 1178;
const ERROR_JOURNAL_NOT_ACTIVE: i32 = 1179;
const ERROR_JOURNAL_ENTRY_DELETED: i32 = 1181;
const READ_BUFFER: usize = 64 * 1024;
const MAX_SIZE: u64 = 32 << 20;
const ALLOCATION_DELTA: u64 = 8 << 20;

pub struct Journal {
    volume: File,
    serial: u64,
}

fn win32_code(e: &windows::core::Error) -> i32 {
    e.code().0 & 0xFFFF
}

fn map_err(e: windows::core::Error) -> ScanError {
    match win32_code(&e) {
        ERROR_JOURNAL_DELETE_IN_PROGRESS | ERROR_JOURNAL_NOT_ACTIVE | ERROR_JOURNAL_ENTRY_DELETED => ScanError::JournalReset,
        code => ScanError::from(std::io::Error::from_raw_os_error(code)),
    }
}

impl Journal {
    pub fn open(drive: char) -> Result<Journal, ScanError> {
        let volume = open_volume(drive)?;
        let serial = volume_data(&volume)?.VolumeSerialNumber as u64;
        Ok(Journal { volume, serial })
    }

    fn handle(&self) -> HANDLE {
        HANDLE(self.volume.as_raw_handle())
    }

    pub fn volume_serial(&self) -> u64 {
        self.serial
    }

    fn query(&self) -> Result<JournalInfo, windows::core::Error> {
        let mut data = USN_JOURNAL_DATA_V0::default();
        let mut ret = 0u32;
        unsafe {
            DeviceIoControl(
                self.handle(),
                FSCTL_QUERY_USN_JOURNAL,
                None,
                0,
                Some(&mut data as *mut _ as *mut _),
                std::mem::size_of::<USN_JOURNAL_DATA_V0>() as u32,
                Some(&mut ret),
                None,
            )
        }?;
        Ok(JournalInfo { journal_id: data.UsnJournalID, first_usn: data.FirstUsn, next_usn: data.NextUsn })
    }

    /// Consulta o journal; se o volume não tiver um, cria (32 MiB) e consulta de novo.
    pub fn query_or_create(&self) -> Result<JournalInfo, ScanError> {
        match self.query() {
            Ok(info) => Ok(info),
            Err(e) if win32_code(&e) == ERROR_JOURNAL_NOT_ACTIVE => {
                let create = CREATE_USN_JOURNAL_DATA { MaximumSize: MAX_SIZE, AllocationDelta: ALLOCATION_DELTA };
                let mut ret = 0u32;
                unsafe {
                    DeviceIoControl(
                        self.handle(),
                        FSCTL_CREATE_USN_JOURNAL,
                        Some(&create as *const _ as *const _),
                        std::mem::size_of::<CREATE_USN_JOURNAL_DATA>() as u32,
                        None,
                        0,
                        Some(&mut ret),
                        None,
                    )
                }
                .map_err(map_err)?;
                self.query().map_err(map_err)
            }
            Err(e) => Err(map_err(e)),
        }
    }

    /// Lê registros a partir de `from`. Com `wait`, espera até 1 s por algum registro.
    pub fn read(&self, journal_id: u64, from: i64, wait: bool) -> Result<(i64, Vec<UsnRecord>), ScanError> {
        let req = READ_USN_JOURNAL_DATA_V0 {
            StartUsn: from,
            ReasonMask: 0xFFFF_FFFF,
            ReturnOnlyOnClose: 0,
            Timeout: if wait { 1 } else { 0 },
            BytesToWaitFor: if wait { 1 } else { 0 },
            UsnJournalID: journal_id,
        };
        let mut buf = vec![0u8; READ_BUFFER];
        let mut ret = 0u32;
        unsafe {
            DeviceIoControl(
                self.handle(),
                FSCTL_READ_USN_JOURNAL,
                Some(&req as *const _ as *const _),
                std::mem::size_of::<READ_USN_JOURNAL_DATA_V0>() as u32,
                Some(buf.as_mut_ptr() as *mut _),
                buf.len() as u32,
                Some(&mut ret),
                None,
            )
        }
        .map_err(map_err)?;
        parse_read_buffer(&buf[..ret as usize]).map_err(|_| ScanError::Io(std::io::ErrorKind::InvalidData.into()))
    }

    /// Tamanho atual de um arquivo pela referência completa; `None` se não abrir.
    pub fn stat(&self, frn_full: u64) -> Option<FileStat> {
        let id = FILE_ID_DESCRIPTOR {
            dwSize: std::mem::size_of::<FILE_ID_DESCRIPTOR>() as u32,
            Type: FILE_ID_TYPE(0),
            Anonymous: FILE_ID_DESCRIPTOR_0 { FileId: frn_full as i64 },
        };
        unsafe {
            let h = OpenFileById(
                self.handle(),
                &id,
                FILE_READ_ATTRIBUTES.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                FILE_FLAG_BACKUP_SEMANTICS,
            )
            .ok()?;
            let mut std_info = FILE_STANDARD_INFO::default();
            let mut basic = FILE_BASIC_INFO::default();
            let ok = GetFileInformationByHandleEx(
                h,
                FileStandardInfo,
                &mut std_info as *mut _ as *mut _,
                std::mem::size_of::<FILE_STANDARD_INFO>() as u32,
            )
            .is_ok()
                && GetFileInformationByHandleEx(
                    h,
                    FileBasicInfo,
                    &mut basic as *mut _ as *mut _,
                    std::mem::size_of::<FILE_BASIC_INFO>() as u32,
                )
                .is_ok();
            let _ = CloseHandle(h);
            ok.then(|| FileStat {
                size: std_info.EndOfFile as u64,
                alloc: std_info.AllocationSize as u64,
                modified: basic.LastWriteTime,
                attributes: basic.FileAttributes,
            })
        }
    }
}
