mod support;

use acervo_index::{apply_fixup, parse_record, RecordError};
use support::RecordBuilder;

#[test]
fn resident_file() {
    let mut b = RecordBuilder::new(40)
        .name(5, "a.txt", 3)
        .std_info(133_000_000_000_000_000, 0x20)
        .resident_data(120)
        .build();
    apply_fixup(&mut b, 512).unwrap();
    let r = parse_record(&b, 40).unwrap();
    assert!(r.in_use && !r.is_dir);
    assert_eq!((r.size, r.alloc), (120, 0));
    assert_eq!(r.names[0].name, "a.txt");
    assert_eq!(r.names[0].parent, 5);
    assert_eq!(r.modified, 133_000_000_000_000_000);
    assert_eq!(r.attributes, 0x20);
}

#[test]
fn nonresident_ads_and_dir() {
    let mut b = RecordBuilder::new(41)
        .name(5, "v.mkv", 1)
        .nonresident_data(10_000, 12_288)
        .named_data("Zone.Identifier", 4096)
        .build();
    apply_fixup(&mut b, 512).unwrap();
    let r = parse_record(&b, 41).unwrap();
    assert_eq!((r.size, r.alloc, r.ads_alloc), (10_000, 12_288, 4096));

    let mut d = RecordBuilder::new(42).dir().name(5, "pasta", 1).index_alloc(8192).build();
    apply_fixup(&mut d, 512).unwrap();
    let r = parse_record(&d, 42).unwrap();
    assert!(r.is_dir);
    assert_eq!(r.index_alloc, 8192);
}

#[test]
fn dos_name_dropped_hardlinks_kept() {
    let mut b = RecordBuilder::new(43)
        .name(5, "LONGFI~1.TXT", 2)
        .name(5, "longfilename.txt", 1)
        .name(77, "outro.txt", 1)
        .resident_data(1)
        .build();
    apply_fixup(&mut b, 512).unwrap();
    let names: Vec<(u64, String)> = parse_record(&b, 43)
        .unwrap()
        .names
        .into_iter()
        .map(|n| (n.parent, n.name))
        .collect();
    assert_eq!(names, vec![(5, "longfilename.txt".into()), (77, "outro.txt".into())]);
}

#[test]
fn invalid_utf16_name_is_lossy() {
    let mut b = RecordBuilder::new(44).name_utf16(5, &[0x0061, 0xD800, 0x0062], 1).build();
    apply_fixup(&mut b, 512).unwrap();
    assert_eq!(parse_record(&b, 44).unwrap().names[0].name, "a\u{FFFD}b");
}

#[test]
fn bad_fixup_and_signature() {
    let mut b = RecordBuilder::new(45).name(5, "x", 1).build();
    b[510] ^= 0xFF;
    assert!(matches!(apply_fixup(&mut b, 512), Err(RecordError::BadFixup)));
    let mut z = vec![0u8; 1024];
    assert!(matches!(apply_fixup(&mut z, 512), Err(RecordError::BadSignature)));
}

#[test]
fn zero_length_attribute_is_error() {
    let mut b = RecordBuilder::new(46).name(5, "x", 1).raw_attribute(0x80, &[]).build();
    apply_fixup(&mut b, 512).unwrap();
    assert!(matches!(parse_record(&b, 46), Err(RecordError::Malformed(_))));
}

#[test]
fn extension_and_unused() {
    let mut b = RecordBuilder::new(47).base(41).nonresident_data(1, 4096).build();
    apply_fixup(&mut b, 512).unwrap();
    assert_eq!(parse_record(&b, 47).unwrap().base_record, 41);
    let mut u = RecordBuilder::new(48).in_use(false).build();
    apply_fixup(&mut u, 512).unwrap();
    assert!(!parse_record(&u, 48).unwrap().in_use);
}

// ---- Revisão final de código ----

#[test]
fn truncated_file_name_is_error_not_panic() {
    // valor de $FILE_NAME com exatamente 65 bytes: o byte do namespace (@65) não existe
    let mut b = RecordBuilder::new(60).truncated_file_name(65).build();
    apply_fixup(&mut b, 512).unwrap();
    assert!(matches!(parse_record(&b, 60), Err(RecordError::Malformed(_))));
}

#[test]
fn sparse_uncompressed_uses_real_allocation() {
    // 10 GB lógicos, nada alocado de verdade (compression_unit = 0, flag esparso)
    let mut b = RecordBuilder::new(61).name(5, "disco.vhdx", 1).sparse_data(10 << 30, 10 << 30, 0).build();
    apply_fixup(&mut b, 512).unwrap();
    let r = parse_record(&b, 61).unwrap();
    assert_eq!((r.size, r.alloc), (10 << 30, 0));
}

#[test]
fn fixup_on_4kn_disk_uses_512_byte_stride() {
    // disco 4Kn: registro de 4096 bytes, setor físico de 4096, fixup a cada 512
    let mut b = RecordBuilder::new(62).record_size(4096).name(5, "a.txt", 1).resident_data(10).build();
    apply_fixup(&mut b, 4096).unwrap();
    assert_eq!(parse_record(&b, 62).unwrap().names[0].name, "a.txt");
}
