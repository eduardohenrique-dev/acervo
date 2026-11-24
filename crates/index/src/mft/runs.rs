//! Decodificação de data runs (mapa de clusters de um atributo não residente).

use super::record::RecordError;

/// Trecho contíguo no disco.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub lcn: u64,
    pub clusters: u64,
}

fn read_le(bytes: &[u8], signed: bool) -> i64 {
    let mut v: i64 = 0;
    for (i, b) in bytes.iter().enumerate() {
        v |= (*b as i64) << (8 * i);
    }
    if signed && !bytes.is_empty() && bytes.len() < 8 && bytes[bytes.len() - 1] & 0x80 != 0 {
        v -= 1i64 << (8 * bytes.len());
    }
    v
}

/// Decodifica uma lista de data runs. Runs esparsos (sem offset) são ignorados.
pub fn decode_runs(bytes: &[u8]) -> Result<Vec<Run>, RecordError> {
    let mut runs = Vec::new();
    let mut i = 0;
    let mut lcn: i64 = 0;
    while i < bytes.len() {
        let header = bytes[i];
        if header == 0 {
            break;
        }
        let len_size = (header & 0x0F) as usize;
        let off_size = (header >> 4) as usize;
        if len_size == 0 || len_size > 8 || off_size > 8 {
            return Err(RecordError::Malformed("cabeçalho de run inválido"));
        }
        let end = i + 1 + len_size + off_size;
        if end > bytes.len() {
            return Err(RecordError::Malformed("run truncado"));
        }
        let clusters = read_le(&bytes[i + 1..i + 1 + len_size], false) as u64;
        if off_size > 0 {
            lcn += read_le(&bytes[i + 1 + len_size..end], true);
            if lcn < 0 {
                return Err(RecordError::Malformed("LCN negativo"));
            }
            runs.push(Run { lcn: lcn as u64, clusters });
        }
        i = end;
    }
    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode() {
        let runs = decode_runs(&[0x21, 0x18, 0x34, 0x56, 0x11, 0x10, 0xF0, 0x00]).unwrap();
        assert_eq!(
            runs,
            vec![Run { lcn: 0x5634, clusters: 0x18 }, Run { lcn: 0x5624, clusters: 0x10 }]
        );
        assert!(decode_runs(&[0x21, 0x18]).is_err());
    }
}
