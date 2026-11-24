#![allow(dead_code)]
//! Gerador de registros FILE sintéticos (1024 bytes, setores de 512).

const RECORD: usize = 1024;
const SECTOR: usize = 512;
const USA_OFFSET: usize = 48;
const FIRST_ATTR: usize = 56;
const USN: u16 = 0x0007;

pub struct RecordBuilder {
    record_no: u32,
    flags: u16,
    base: u64,
    attrs: Vec<Vec<u8>>,
    size: usize,
}

fn align8(n: usize) -> usize {
    (n + 7) & !7
}

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn resident(kind: u32, name: &[u16], value: &[u8]) -> Vec<u8> {
    let name_off = 24;
    let value_off = align8(name_off + name.len() * 2);
    let len = align8(value_off + value.len());
    let mut a = vec![0u8; len];
    a[0..4].copy_from_slice(&kind.to_le_bytes());
    a[4..8].copy_from_slice(&(len as u32).to_le_bytes());
    a[8] = 0;
    a[9] = name.len() as u8;
    a[10..12].copy_from_slice(&(name_off as u16).to_le_bytes());
    a[16..20].copy_from_slice(&(value.len() as u32).to_le_bytes());
    a[20..22].copy_from_slice(&(value_off as u16).to_le_bytes());
    for (i, c) in name.iter().enumerate() {
        a[name_off + i * 2..name_off + i * 2 + 2].copy_from_slice(&c.to_le_bytes());
    }
    a[value_off..value_off + value.len()].copy_from_slice(value);
    a
}

fn nonresident(kind: u32, name: &[u16], size: u64, alloc: u64) -> Vec<u8> {
    let name_off = 64;
    let runs_off = align8(name_off + name.len() * 2);
    let len = align8(runs_off + 1);
    let mut a = vec![0u8; len];
    a[0..4].copy_from_slice(&kind.to_le_bytes());
    a[4..8].copy_from_slice(&(len as u32).to_le_bytes());
    a[8] = 1;
    a[9] = name.len() as u8;
    a[10..12].copy_from_slice(&(name_off as u16).to_le_bytes());
    // start_vcn @16 = 0, last_vcn @24
    a[24..32].copy_from_slice(&(alloc / 4096).saturating_sub(1).to_le_bytes());
    a[32..34].copy_from_slice(&(runs_off as u16).to_le_bytes());
    a[40..48].copy_from_slice(&alloc.to_le_bytes());
    a[48..56].copy_from_slice(&size.to_le_bytes());
    a[56..64].copy_from_slice(&size.to_le_bytes());
    for (i, c) in name.iter().enumerate() {
        a[name_off + i * 2..name_off + i * 2 + 2].copy_from_slice(&c.to_le_bytes());
    }
    a
}

/// Atributo não residente esparso, sem compressão (`compression_unit = 0`), com o
/// tamanho realmente alocado em `@64`, como o Windows grava.
fn sparse(kind: u32, size: u64, logical_alloc: u64, real_alloc: u64) -> Vec<u8> {
    let name_off = 72;
    let runs_off = 72;
    let len = align8(runs_off + 1);
    let mut a = vec![0u8; len];
    a[0..4].copy_from_slice(&kind.to_le_bytes());
    a[4..8].copy_from_slice(&(len as u32).to_le_bytes());
    a[8] = 1;
    a[10..12].copy_from_slice(&(name_off as u16).to_le_bytes());
    a[12..14].copy_from_slice(&0x8000u16.to_le_bytes());
    a[32..34].copy_from_slice(&(runs_off as u16).to_le_bytes());
    a[40..48].copy_from_slice(&logical_alloc.to_le_bytes());
    a[48..56].copy_from_slice(&size.to_le_bytes());
    a[56..64].copy_from_slice(&size.to_le_bytes());
    a[64..72].copy_from_slice(&real_alloc.to_le_bytes());
    a
}

impl RecordBuilder {
    pub fn new(record_no: u32) -> Self {
        RecordBuilder { record_no, flags: 0x1, base: 0, attrs: Vec::new(), size: RECORD }
    }
    /// Tamanho do registro (1024 por padrão; 4096 em discos 4Kn).
    pub fn record_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }
    pub fn sparse_data(mut self, size: u64, logical_alloc: u64, real_alloc: u64) -> Self {
        self.attrs.push(sparse(0x80, size, logical_alloc, real_alloc));
        self
    }
    /// `$FILE_NAME` residente com valor de `len` bytes (para testar valores truncados).
    pub fn truncated_file_name(mut self, len: usize) -> Self {
        let mut v = vec![0u8; len];
        v[0..8].copy_from_slice(&(5u64 | (1 << 48)).to_le_bytes());
        if len > 64 {
            v[64] = 0;
        }
        self.attrs.push(resident(0x30, &[], &v));
        self
    }
    pub fn dir(mut self) -> Self {
        self.flags |= 0x2;
        self
    }
    pub fn in_use(mut self, yes: bool) -> Self {
        if yes { self.flags |= 0x1 } else { self.flags &= !0x1 }
        self
    }
    pub fn base(mut self, base: u64) -> Self {
        // número de sequência nos 16 bits altos, como no disco
        self.base = base | (3 << 48);
        self
    }
    pub fn name(self, parent: u64, name: &str, ns: u8) -> Self {
        self.name_utf16(parent, &utf16(name), ns)
    }
    pub fn name_utf16(mut self, parent: u64, name: &[u16], ns: u8) -> Self {
        let mut v = vec![0u8; 66 + name.len() * 2];
        v[0..8].copy_from_slice(&(parent | (1 << 48)).to_le_bytes());
        v[64] = name.len() as u8;
        v[65] = ns;
        for (i, c) in name.iter().enumerate() {
            v[66 + i * 2..68 + i * 2].copy_from_slice(&c.to_le_bytes());
        }
        self.attrs.push(resident(0x30, &[], &v));
        self
    }
    pub fn std_info(mut self, modified: i64, attrs: u32) -> Self {
        let mut v = vec![0u8; 48];
        v[8..16].copy_from_slice(&modified.to_le_bytes());
        v[32..36].copy_from_slice(&attrs.to_le_bytes());
        self.attrs.push(resident(0x10, &[], &v));
        self
    }
    pub fn resident_data(mut self, len: usize) -> Self {
        self.attrs.push(resident(0x80, &[], &vec![0xAB; len]));
        self
    }
    pub fn nonresident_data(mut self, size: u64, alloc: u64) -> Self {
        self.attrs.push(nonresident(0x80, &[], size, alloc));
        self
    }
    pub fn named_data(mut self, name: &str, alloc: u64) -> Self {
        self.attrs.push(nonresident(0x80, &utf16(name), alloc, alloc));
        self
    }
    pub fn index_alloc(mut self, alloc: u64) -> Self {
        self.attrs.push(nonresident(0xA0, &utf16("$I30"), alloc, alloc));
        self
    }
    /// Atributo cru. Com `bytes` vazio grava um atributo de tamanho 0.
    pub fn raw_attribute(mut self, kind: u32, bytes: &[u8]) -> Self {
        let mut a = vec![0u8; 8];
        a[0..4].copy_from_slice(&kind.to_le_bytes());
        a[4..8].copy_from_slice(&(if bytes.is_empty() { 0 } else { 8 + bytes.len() as u32 }).to_le_bytes());
        a.extend_from_slice(bytes);
        self.attrs.push(a);
        self
    }

    /// Registro como fica no disco (fixup "desaplicado").
    pub fn build(self) -> Vec<u8> {
        let size = self.size;
        let usa_count = size / SECTOR + 1;
        let first_attr = align8(USA_OFFSET + usa_count * 2).max(FIRST_ATTR);
        let mut b = vec![0u8; size];
        b[0..4].copy_from_slice(b"FILE");
        b[4..6].copy_from_slice(&(USA_OFFSET as u16).to_le_bytes());
        b[6..8].copy_from_slice(&(usa_count as u16).to_le_bytes());
        b[20..22].copy_from_slice(&(first_attr as u16).to_le_bytes());
        b[22..24].copy_from_slice(&self.flags.to_le_bytes());
        b[28..32].copy_from_slice(&(size as u32).to_le_bytes());
        b[32..40].copy_from_slice(&self.base.to_le_bytes());
        b[44..48].copy_from_slice(&self.record_no.to_le_bytes());
        let mut off = first_attr;
        for a in &self.attrs {
            b[off..off + a.len()].copy_from_slice(a);
            off += a.len();
        }
        b[off..off + 4].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        b[24..28].copy_from_slice(&((off + 8) as u32).to_le_bytes());
        // fixup: guarda os 2 últimos bytes de cada setor e grava o USN no lugar
        b[USA_OFFSET..USA_OFFSET + 2].copy_from_slice(&USN.to_le_bytes());
        for i in 0..size / SECTOR {
            let end = (i + 1) * SECTOR - 2;
            let slot = USA_OFFSET + 2 + i * 2;
            b[slot] = b[end];
            b[slot + 1] = b[end + 1];
            b[end..end + 2].copy_from_slice(&USN.to_le_bytes());
        }
        b
    }
}

// ---- Registros prontos e montagem de árvores (Tasks 4 e 7) ----

use acervo_classify::{Category, Classifier, PathRule, Strength};
use acervo_index::{FileName, ParsedRecord, Tree, TreeBuilder};

pub fn fname(parent: u64, name: &str) -> FileName {
    FileName { parent, name: name.into(), namespace: 1 }
}

pub fn root(no: u64) -> ParsedRecord {
    ParsedRecord { record_no: no, in_use: true, is_dir: true, names: vec![fname(no, ".")], ..Default::default() }
}

pub fn dir(no: u64, parent: u64, name: &str) -> ParsedRecord {
    ParsedRecord { record_no: no, in_use: true, is_dir: true, names: vec![fname(parent, name)], ..Default::default() }
}

pub fn file(no: u64, parent: u64, name: &str, alloc: u64) -> ParsedRecord {
    ParsedRecord { record_no: no, in_use: true, names: vec![fname(parent, name)], size: alloc, alloc, ..Default::default() }
}

pub fn build_with(records: Vec<ParsedRecord>, classifier: &Classifier) -> Tree {
    let mut b = TreeBuilder::new('C', 5, 1024);
    for r in records {
        b.push(r);
    }
    b.finish(classifier)
}

/// Classificador mínimo: só `C:\Windows` → Sistema.
pub fn build(records: Vec<ParsedRecord>) -> Tree {
    let c = Classifier::new(vec![PathRule {
        path: r"C:\Windows".into(),
        category: Category::Sistema,
        strength: Strength::Strong,
    }]);
    build_with(records, &c)
}

// ---- Buffers do USN Journal (Fase 2) ----

/// Monta o buffer devolvido por FSCTL_READ_USN_JOURNAL: próximo USN + registros V2.
pub struct UsnBuilder {
    buf: Vec<u8>,
}

impl UsnBuilder {
    pub fn new(next_usn: i64) -> Self {
        UsnBuilder { buf: next_usn.to_le_bytes().to_vec() }
    }
    pub fn record(mut self, usn: i64, frn_full: u64, parent_full: u64, reason: u32, attrs: u32, name: &str) -> Self {
        let name: Vec<u16> = name.encode_utf16().collect();
        let len = align8(60 + name.len() * 2);
        let mut r = vec![0u8; len];
        r[0..4].copy_from_slice(&(len as u32).to_le_bytes());
        r[4..6].copy_from_slice(&2u16.to_le_bytes());
        r[8..16].copy_from_slice(&frn_full.to_le_bytes());
        r[16..24].copy_from_slice(&parent_full.to_le_bytes());
        r[24..32].copy_from_slice(&usn.to_le_bytes());
        r[40..44].copy_from_slice(&reason.to_le_bytes());
        r[52..56].copy_from_slice(&attrs.to_le_bytes());
        r[56..58].copy_from_slice(&((name.len() * 2) as u16).to_le_bytes());
        r[58..60].copy_from_slice(&60u16.to_le_bytes());
        for (i, c) in name.iter().enumerate() {
            r[60 + i * 2..62 + i * 2].copy_from_slice(&c.to_le_bytes());
        }
        self.buf.extend(r);
        self
    }
    pub fn build(self) -> Vec<u8> {
        self.buf
    }
}

// ---- Mudanças para Tree::apply (Fase 2) ----

use acervo_index::usn::Change;
use acervo_index::FileStat;
use std::collections::HashMap;

pub const MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

pub fn created(frn: u64, parent: u64, name: &str, is_dir: bool) -> Change {
    Change::Created { frn, frn_full: frn | (1 << 48), parent, name: name.into(), is_dir, attributes: if is_dir { 0x10 } else { 0x20 } }
}
pub fn deleted(frn: u64, parent: u64, name: &str) -> Change {
    Change::Deleted { frn, parent, name: name.into() }
}
pub fn modified(frn: u64) -> Change {
    Change::Modified { frn, frn_full: frn | (1 << 48) }
}
pub fn moved(frn: u64, old_parent: u64, old_name: &str, new_parent: u64, new_name: &str) -> Change {
    Change::Moved { frn, frn_full: frn | (1 << 48), old_parent, old_name: old_name.into(), new_parent, new_name: new_name.into(), is_dir: false, attributes: 0x20 }
}
pub fn link_toggled(frn: u64, parent: u64, name: &str) -> Change {
    Change::LinkToggled { frn, frn_full: frn | (1 << 48), parent, name: name.into() }
}
/// Tamanhos por frn (48 bits): (frn, alloc).
pub fn stats(list: &[(u64, u64)]) -> HashMap<u64, FileStat> {
    list.iter().map(|&(f, a)| (f, FileStat { size: a, alloc: a, modified: 1, attributes: 0x20 })).collect()
}
/// O classificador mínimo de `build` (só `C:\Windows` → Sistema).
pub fn classifier() -> Classifier {
    Classifier::new(vec![PathRule { path: r"C:\Windows".into(), category: Category::Sistema, strength: Strength::Strong }])
}

/// `Classifier::standard` de uma máquina fixa com o perfil C:\Users\Ana.
pub fn standard() -> Classifier {
    use acervo_classify::{KnownPaths, UserProfile};
    use std::path::PathBuf;
    let root = PathBuf::from(r"C:\Users\Ana");
    Classifier::standard(&KnownPaths {
        windows_dir: PathBuf::from(r"C:\Windows"),
        program_files: vec![PathBuf::from(r"C:\Program Files")],
        program_data: PathBuf::from(r"C:\ProgramData"),
        profiles: vec![UserProfile {
            downloads: root.join("Downloads"),
            documents: root.join("Documents"),
            appdata_roaming: root.join(r"AppData\Roaming"),
            appdata_local: root.join(r"AppData\Local"),
            appdata_locallow: root.join(r"AppData\LocalLow"),
            temp: root.join(r"AppData\Local\Temp"),
            root,
        }],
        game_libraries: vec![],
        drives: vec!['C'],
    })
}

/// C:\Users\Ana\Downloads (30) e Documents (31), com v.mp4 (70, 8192 bytes) em Downloads.
pub fn user_tree() -> Vec<ParsedRecord> {
    vec![
        root(5),
        dir(20, 5, "Users"),
        dir(21, 20, "Ana"),
        dir(30, 21, "Downloads"),
        dir(31, 21, "Documents"),
        file(70, 30, "v.mp4", 8192),
    ]
}
/// Igual a `moved`, para pastas.
pub fn moved_dir(frn: u64, old_parent: u64, old_name: &str, new_parent: u64, new_name: &str) -> Change {
    Change::Moved { frn, frn_full: frn | (1 << 48), old_parent, old_name: old_name.into(), new_parent, new_name: new_name.into(), is_dir: true, attributes: 0x10 }
}
