//! Fixup e interpretação de um registro FILE da MFT.

use super::runs::{decode_runs, Run};

const ATTR_STANDARD_INFORMATION: u32 = 0x10;
const ATTR_FILE_NAME: u32 = 0x30;
const ATTR_DATA: u32 = 0x80;
const ATTR_INDEX_ALLOCATION: u32 = 0xA0;
const ATTR_END: u32 = 0xFFFF_FFFF;

const REF_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;
const NS_DOS: u8 = 2;

/// Um nome (link) do arquivo.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileName {
    /// Número do registro do pai (48 bits baixos da referência).
    pub parent: u64,
    pub name: String,
    pub namespace: u8,
}

/// O que interessa de um registro FILE.
#[derive(Clone, Debug, Default)]
pub struct ParsedRecord {
    pub record_no: u64,
    /// 0 se for o registro base; senão, o base ao qual este pertence.
    pub base_record: u64,
    pub in_use: bool,
    pub is_dir: bool,
    pub names: Vec<FileName>,
    pub size: u64,
    pub alloc: u64,
    pub ads_alloc: u64,
    pub index_alloc: u64,
    pub modified: i64,
    pub attributes: u32,
}

#[derive(Debug)]
pub enum RecordError {
    BadSignature,
    BadFixup,
    Malformed(&'static str),
}

impl std::fmt::Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecordError::BadSignature => write!(f, "assinatura inválida"),
            RecordError::BadFixup => write!(f, "fixup inválido"),
            RecordError::Malformed(m) => write!(f, "registro malformado: {m}"),
        }
    }
}

fn u16_at(b: &[u8], o: usize) -> Result<u16, RecordError> {
    b.get(o..o + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or(RecordError::Malformed("fora do registro"))
}

fn u32_at(b: &[u8], o: usize) -> Result<u32, RecordError> {
    b.get(o..o + 4)
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
        .ok_or(RecordError::Malformed("fora do registro"))
}

fn u64_at(b: &[u8], o: usize) -> Result<u64, RecordError> {
    b.get(o..o + 8)
        .map(|s| u64::from_le_bytes(s.try_into().unwrap()))
        .ok_or(RecordError::Malformed("fora do registro"))
}

/// Bloco protegido pelo update sequence: sempre 512 bytes no NTFS, qualquer que seja o setor físico.
const FIXUP_STRIDE: usize = 512;

/// Restaura os últimos 2 bytes de cada bloco de 512 a partir do update sequence array.
///
/// `_bytes_per_sector` é ignorado: em discos 4Kn o setor tem 4096 bytes, mas o NTFS
/// continua protegendo blocos de 512 (o parâmetro fica para documentar a chamada).
pub fn apply_fixup(buf: &mut [u8], _bytes_per_sector: usize) -> Result<(), RecordError> {
    if buf.len() < 48 || &buf[0..4] != b"FILE" {
        return Err(RecordError::BadSignature);
    }
    let usa_off = u16_at(buf, 4)? as usize;
    let usa_count = u16_at(buf, 6)? as usize;
    if usa_count < 2 {
        return Err(RecordError::BadFixup);
    }
    let sectors = usa_count - 1;
    let bytes_per_sector = FIXUP_STRIDE;
    if sectors * bytes_per_sector > buf.len() || usa_off + usa_count * 2 > buf.len() {
        return Err(RecordError::BadFixup);
    }
    let usn = [buf[usa_off], buf[usa_off + 1]];
    for i in 0..sectors {
        let end = (i + 1) * bytes_per_sector - 2;
        if buf[end..end + 2] != usn {
            return Err(RecordError::BadFixup);
        }
        let slot = usa_off + 2 + i * 2;
        buf[end] = buf[slot];
        buf[end + 1] = buf[slot + 1];
    }
    Ok(())
}

/// Atributo bruto dentro de um registro.
struct Attr<'a> {
    kind: u32,
    bytes: &'a [u8],
    non_resident: bool,
    name_len: usize,
}

impl<'a> Attr<'a> {
    fn flags(&self) -> u16 {
        u16::from_le_bytes([self.bytes[12], self.bytes[13]])
    }
    fn value(&self) -> Result<&'a [u8], RecordError> {
        let len = u32_at(self.bytes, 16)? as usize;
        let off = u16_at(self.bytes, 20)? as usize;
        self.bytes
            .get(off..off + len)
            .ok_or(RecordError::Malformed("valor fora do atributo"))
    }
    fn start_vcn(&self) -> Result<u64, RecordError> {
        u64_at(self.bytes, 16)
    }
    /// (tamanho lógico, tamanho alocado) de um atributo não residente.
    fn nonres_sizes(&self) -> Result<(u64, u64), RecordError> {
        let alloc = u64_at(self.bytes, 40)?;
        let size = u64_at(self.bytes, 48)?;
        // Comprimido ou esparso: o que de fato ocupa o disco fica em @64 (mesmo com
        // compression_unit = 0, como o Windows grava nos esparsos sem compressão).
        let compressed_or_sparse = self.flags() & (0x0001 | 0x8000) != 0;
        if compressed_or_sparse && self.bytes.len() >= 72 {
            return Ok((size, u64_at(self.bytes, 64)?));
        }
        Ok((size, alloc))
    }
}

fn attributes(buf: &[u8]) -> impl Iterator<Item = Result<Attr<'_>, RecordError>> {
    let mut off = u16_at(buf, 20).map(|v| v as usize).unwrap_or(usize::MAX);
    let mut done = false;
    std::iter::from_fn(move || {
        if done {
            return None;
        }
        let item = (|| {
            let kind = u32_at(buf, off)?;
            if kind == ATTR_END {
                return Ok(None);
            }
            let len = u32_at(buf, off + 4)? as usize;
            if len < 16 || off + len > buf.len() {
                return Err(RecordError::Malformed("tamanho de atributo inválido"));
            }
            let bytes = &buf[off..off + len];
            off += len;
            Ok(Some(Attr {
                kind,
                non_resident: bytes[8] != 0,
                name_len: bytes[9] as usize,
                bytes,
            }))
        })();
        match item {
            Ok(Some(a)) => Some(Ok(a)),
            Ok(None) => {
                done = true;
                None
            }
            Err(e) => {
                done = true;
                Some(Err(e))
            }
        }
    })
}

fn parse_file_name(v: &[u8]) -> Result<FileName, RecordError> {
    let parent = u64_at(v, 0)? & REF_MASK;
    let len = *v.get(64).ok_or(RecordError::Malformed("$FILE_NAME curto"))? as usize;
    let namespace = *v.get(65).ok_or(RecordError::Malformed("$FILE_NAME curto"))?;
    let raw = v
        .get(66..66 + len * 2)
        .ok_or(RecordError::Malformed("nome fora do atributo"))?;
    let units: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    Ok(FileName {
        parent,
        name: String::from_utf16_lossy(&units),
        namespace,
    })
}

/// Mantém um nome por pai, preferindo o namespace Win32, e descarta nomes DOS.
fn dedupe_names(all: Vec<FileName>) -> Vec<FileName> {
    let mut out: Vec<FileName> = Vec::new();
    for n in all.into_iter().filter(|n| n.namespace != NS_DOS) {
        match out.iter_mut().find(|o| o.parent == n.parent) {
            Some(existing) => {
                let is_win32 = |ns: u8| ns == 1 || ns == 3;
                if !is_win32(existing.namespace) && is_win32(n.namespace) {
                    *existing = n;
                }
            }
            None => out.push(n),
        }
    }
    out
}

/// Interpreta um registro (já com o fixup aplicado).
pub fn parse_record(buf: &[u8], record_no: u64) -> Result<ParsedRecord, RecordError> {
    if buf.len() < 48 || &buf[0..4] != b"FILE" {
        return Err(RecordError::BadSignature);
    }
    let flags = u16_at(buf, 22)?;
    let mut r = ParsedRecord {
        record_no,
        base_record: u64_at(buf, 32)? & REF_MASK,
        in_use: flags & 0x1 != 0,
        is_dir: flags & 0x2 != 0,
        ..Default::default()
    };
    if !r.in_use {
        return Ok(r);
    }
    let mut names = Vec::new();
    for attr in attributes(buf) {
        let a = attr?;
        match a.kind {
            ATTR_STANDARD_INFORMATION if !a.non_resident => {
                let v = a.value()?;
                r.modified = u64_at(v, 8)? as i64;
                r.attributes = u32_at(v, 32)?;
            }
            ATTR_FILE_NAME if !a.non_resident => names.push(parse_file_name(a.value()?)?),
            ATTR_DATA => {
                let (size, alloc) = if a.non_resident {
                    if a.start_vcn()? != 0 {
                        continue;
                    }
                    a.nonres_sizes()?
                } else {
                    (a.value()?.len() as u64, 0)
                };
                if a.name_len == 0 {
                    r.size = size;
                    r.alloc = alloc;
                } else {
                    r.ads_alloc += alloc;
                }
            }
            ATTR_INDEX_ALLOCATION if a.non_resident && a.start_vcn()? == 0 => {
                r.index_alloc += a.nonres_sizes()?.1;
            }
            _ => {}
        }
    }
    r.names = dedupe_names(names);
    Ok(r)
}

/// Data runs do `$DATA` sem nome (usado para localizar a própria `$MFT`).
pub fn data_runs_of_record(buf: &[u8]) -> Result<Vec<Run>, RecordError> {
    for attr in attributes(buf) {
        let a = attr?;
        if a.kind == ATTR_DATA && a.non_resident && a.name_len == 0 && a.start_vcn()? == 0 {
            let runs_off = u16_at(a.bytes, 32)? as usize;
            let runs = a
                .bytes
                .get(runs_off..)
                .ok_or(RecordError::Malformed("runs fora do atributo"))?;
            return decode_runs(runs);
        }
    }
    Err(RecordError::Malformed("registro sem $DATA não residente"))
}
