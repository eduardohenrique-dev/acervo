//! Registros `USN_RECORD_V2` devolvidos por `FSCTL_READ_USN_JOURNAL`.

/// Bits de `UsnRecord::reason`.
pub mod reason {
    pub const DATA_OVERWRITE: u32 = 0x1;
    pub const DATA_EXTEND: u32 = 0x2;
    pub const DATA_TRUNCATION: u32 = 0x4;
    pub const NAMED_DATA_OVERWRITE: u32 = 0x10;
    pub const NAMED_DATA_EXTEND: u32 = 0x20;
    pub const NAMED_DATA_TRUNCATION: u32 = 0x40;
    pub const FILE_CREATE: u32 = 0x100;
    pub const FILE_DELETE: u32 = 0x200;
    pub const RENAME_OLD_NAME: u32 = 0x1000;
    pub const RENAME_NEW_NAME: u32 = 0x2000;
    pub const HARD_LINK_CHANGE: u32 = 0x10000;
    pub const STREAM_CHANGE: u32 = 0x200000;
    pub const CLOSE: u32 = 0x8000_0000;

    /// Qualquer mudança de conteúdo.
    pub const ANY_DATA: u32 = DATA_OVERWRITE
        | DATA_EXTEND
        | DATA_TRUNCATION
        | NAMED_DATA_OVERWRITE
        | NAMED_DATA_EXTEND
        | NAMED_DATA_TRUNCATION
        | STREAM_CHANGE;
}

/// Atributo de pasta em `UsnRecord::attributes`.
pub const ATTR_DIRECTORY: u32 = 0x10;

const REF_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

#[derive(Clone, Debug, PartialEq)]
pub struct UsnRecord {
    pub usn: i64,
    /// Número do registro (48 bits baixos).
    pub frn: u64,
    /// Referência completa, com o número de sequência (para `OpenFileById`).
    pub frn_full: u64,
    /// Número do registro da pasta-mãe.
    pub parent: u64,
    pub reason: u32,
    pub attributes: u32,
    pub name: String,
}

#[derive(Debug, PartialEq)]
pub enum UsnError {
    Truncated,
    BadVersion(u16),
}

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u64_at(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}

/// Interpreta o buffer: 8 bytes com o próximo USN e depois registros V2 encadeados.
pub fn parse_read_buffer(buf: &[u8]) -> Result<(i64, Vec<UsnRecord>), UsnError> {
    if buf.len() < 8 {
        return Err(UsnError::Truncated);
    }
    let next = u64_at(buf, 0) as i64;
    let mut out = Vec::new();
    let mut off = 8;
    while off < buf.len() {
        if buf.len() - off < 60 {
            return Err(UsnError::Truncated);
        }
        let rec = &buf[off..];
        let len = u32_at(rec, 0) as usize;
        if len < 60 || len > rec.len() {
            return Err(UsnError::Truncated);
        }
        let major = u16_at(rec, 4);
        if major != 2 {
            return Err(UsnError::BadVersion(major));
        }
        let name_len = u16_at(rec, 56) as usize;
        let name_off = u16_at(rec, 58) as usize;
        let raw = rec.get(name_off..name_off + name_len).ok_or(UsnError::Truncated)?;
        if name_off + name_len > len {
            return Err(UsnError::Truncated);
        }
        let units: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let frn_full = u64_at(rec, 8);
        out.push(UsnRecord {
            usn: u64_at(rec, 24) as i64,
            frn: frn_full & REF_MASK,
            frn_full,
            parent: u64_at(rec, 16) & REF_MASK,
            reason: u32_at(rec, 40),
            attributes: u32_at(rec, 52),
            name: String::from_utf16_lossy(&units),
        });
        off += len;
    }
    Ok((next, out))
}
