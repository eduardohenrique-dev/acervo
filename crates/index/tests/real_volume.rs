use acervo_classify::{discover, Classifier};
use acervo_index::scan_ntfs;

fn used_bytes(drive: char) -> u64 {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let root: Vec<u16> = format!("{drive}:\\").encode_utf16().chain([0]).collect();
    let (mut free, mut total) = (0u64, 0u64);
    unsafe { GetDiskFreeSpaceExW(PCWSTR(root.as_ptr()), None, Some(&mut total), Some(&mut free)) }
        .expect("GetDiskFreeSpaceExW");
    total - free
}

#[test]
#[ignore = "exige terminal de administrador"]
fn scan_c_matches_used_space() {
    let classifier = Classifier::standard(&discover());
    let t0 = std::time::Instant::now();
    let tree = scan_ntfs('C', &classifier, &|_| {}).expect("scan");
    let secs = t0.elapsed().as_secs_f64();
    let cataloged: u64 = tree.totals(tree.root).iter().sum();
    let used = used_bytes('C');
    let diff = (cataloged as f64 - used as f64).abs() / used as f64;
    println!(
        "catalogado={cataloged} usado={used} diff={:.3}% tempo={secs:.1}s descartados={}",
        diff * 100.0,
        tree.skipped_records
    );
    assert!(diff <= 0.01, "diferença acima de 1%");
    assert!(secs <= 30.0, "primeira análise acima de 30 s");
}
