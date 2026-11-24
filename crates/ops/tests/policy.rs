use std::path::Path;

use acervo_classify::Category;
use acervo_ops::{check_item, log_line, log_path, ItemFacts, OpKindTag, Protected, Verdict};

fn item(p: &str, cat: Category) -> ItemFacts {
    ItemFacts { path: p.into(), category: cat, is_dir: false, is_reparse: false, is_system: false }
}

fn prot() -> Protected {
    Protected { app_dirs: vec![r"C:\Users\Ana\AppData\Local\Acervo".into(), r"C:\ProgramData\Acervo".into()] }
}

#[test]
fn blocks() {
    assert!(matches!(check_item(&item(r"C:\Windows\a.dll", Category::Sistema), OpKindTag::Recycle, &prot()), Verdict::Blocked { .. }));
    assert!(matches!(check_item(&item(r"D:\", Category::Outros), OpKindTag::Move, &prot()), Verdict::Blocked { .. }));
    assert!(matches!(
        check_item(&item(r"C:\ProgramData\Acervo\index-1.bin", Category::DadosApps), OpKindTag::DeletePermanent, &prot()),
        Verdict::Blocked { .. }
    ));
    let mut j = item(r"C:\Documents and Settings", Category::Outros);
    j.is_dir = true;
    j.is_reparse = true;
    j.is_system = true;
    assert!(matches!(check_item(&j, OpKindTag::Recycle, &prot()), Verdict::Blocked { .. }));
}

#[test]
fn warns_for_program_files() {
    assert!(matches!(check_item(&item(r"C:\Program Files\X\x.exe", Category::Apps), OpKindTag::Recycle, &prot()), Verdict::Warn { .. }));
    assert_eq!(check_item(&item(r"C:\Program Files\X\x.exe", Category::Apps), OpKindTag::Copy, &prot()), Verdict::Allowed);
}

#[test]
fn allows_and_reveal() {
    assert_eq!(
        check_item(&item(r"C:\Users\Ana\Downloads\a.zip", Category::Downloads), OpKindTag::DeletePermanent, &prot()),
        Verdict::Allowed
    );
    assert_eq!(check_item(&item(r"C:\Windows\a.dll", Category::Sistema), OpKindTag::Reveal, &prot()), Verdict::Allowed);
}

#[test]
fn case_insensitive_protection() {
    assert!(matches!(check_item(&item(r"c:\programdata\ACERVO\x", Category::DadosApps), OpKindTag::Recycle, &prot()), Verdict::Blocked { .. }));
    // "AcervoX" não é a pasta do Acervo
    assert!(!matches!(check_item(&item(r"C:\ProgramData\AcervoX\x", Category::DadosApps), OpKindTag::Copy, &prot()), Verdict::Blocked { .. }));
}

#[test]
fn log_format() {
    let l = log_line("2026-11-05T21:03:04", OpKindTag::Recycle, Path::new(r"C:\x\a.bin"), None, 1288490188, "ok");
    assert_eq!(l, r"2026-11-05T21:03:04 lixeira C:\x\a.bin (1,2 GB) → ok");
    let l = log_line("2026-11-05T21:03:04", OpKindTag::Move, Path::new(r"C:\a"), Some(Path::new(r"D:\b")), 0, "erro: acesso negado");
    assert_eq!(l, r"2026-11-05T21:03:04 mover C:\a → D:\b (0 B) → erro: acesso negado");
    assert!(log_path(Path::new(r"C:\ProgramData\Acervo"), 2026, 11).ends_with(r"log\operacoes-2026-11.log"));
}

#[test]
fn append_creates_log_dir() {
    let d = tempfile::tempdir().unwrap();
    acervo_ops::append_log(d.path(), 2026, 11, "linha 1").unwrap();
    acervo_ops::append_log(d.path(), 2026, 11, "linha 2").unwrap();
    let text = std::fs::read_to_string(log_path(d.path(), 2026, 11)).unwrap();
    assert_eq!(text, "linha 1\nlinha 2\n");
}

#[test]
fn blocks_folders_that_contain_the_app_dirs() {
    // Mandar C:\ProgramData para a Lixeira levaria junto os dados do Acervo.
    let mut pd = item(r"C:\ProgramData", Category::DadosApps);
    pd.is_dir = true;
    assert!(matches!(check_item(&pd, OpKindTag::Recycle, &prot()), Verdict::Blocked { .. }));
    let mut other = item(r"C:\ProgramData\Outro", Category::DadosApps);
    other.is_dir = true;
    assert!(matches!(check_item(&other, OpKindTag::Recycle, &prot()), Verdict::Warn { .. }));
}

#[test]
fn destination_of_copy_and_move_is_checked() {
    use acervo_ops::check_dest;
    assert!(matches!(check_dest(Path::new(r"C:\Windows\System32"), Some(Category::Sistema), &prot()), Verdict::Blocked { .. }));
    assert!(matches!(check_dest(Path::new(r"c:\programdata\acervo\x"), Some(Category::DadosApps), &prot()), Verdict::Blocked { .. }));
    assert!(matches!(check_dest(Path::new(r"D:\Backup"), Some(Category::Outros), &prot()), Verdict::Allowed));
    assert!(matches!(check_dest(Path::new(r"E:\Novo"), None, &prot()), Verdict::Allowed));
}
