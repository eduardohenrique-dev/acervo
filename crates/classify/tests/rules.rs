use acervo_classify::{Category, Classifier, KnownPaths, UserProfile};
use std::path::PathBuf;

fn known() -> KnownPaths {
    let root = PathBuf::from(r"C:\Users\Ana");
    KnownPaths {
        windows_dir: PathBuf::from(r"C:\Windows"),
        program_files: vec![PathBuf::from(r"C:\Program Files"), PathBuf::from(r"C:\Program Files (x86)")],
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
        game_libraries: vec![PathBuf::from(r"D:\SteamLibrary\steamapps\common")],
        drives: vec!['C', 'D'],
    }
}

/// Desce componente por componente, como a árvore fará.
fn classify_path(c: &Classifier, path: &str, is_dir: bool, children: &[&str], siblings: &[&str]) -> Category {
    let parts: Vec<&str> = path.split('\\').filter(|s| !s.is_empty()).collect();
    let drive = parts[0].chars().next().unwrap();
    let mut ctx = c.root(drive);
    let last = parts.len() - 1;
    for (i, part) in parts.iter().enumerate().skip(1) {
        if i < last {
            ctx = c.enter_dir(&ctx, part, &|_| false, &|_| false);
        } else if is_dir {
            let has_child = |n: &str| children.iter().any(|c| c.eq_ignore_ascii_case(n));
            let has_sibling = |n: &str| siblings.iter().any(|c| c.eq_ignore_ascii_case(n));
            let d = c.enter_dir(&ctx, part, &has_child, &has_sibling);
            return c.dir_category(&d);
        } else {
            return c.file_category(&ctx, part);
        }
    }
    c.dir_category(&ctx)
}

fn f(path: &str) -> Category {
    classify_path(&Classifier::standard(&known()), path, false, &[], &[])
}

#[test]
fn system_and_cache_inside_windows() {
    assert_eq!(f(r"C:\Windows\System32\kernel32.dll"), Category::Sistema);
    assert_eq!(f(r"C:\Windows\Temp\x.tmp"), Category::Cache);
    assert_eq!(f(r"C:\pagefile.sys"), Category::Sistema);
    assert_eq!(f(r"C:\$MFT"), Category::Sistema);
    assert_eq!(f(r"C:\$Extend\$UsnJrnl"), Category::Sistema);
    assert_eq!(f(r"C:\$Recycle.Bin\S-1-5\$R1.mp4"), Category::Cache);
    assert_eq!(f(r"D:\System Volume Information\x"), Category::Sistema);
}

#[test]
fn user_folders() {
    assert_eq!(f(r"C:\Users\Ana\Downloads\setup.exe"), Category::Downloads);
    assert_eq!(f(r"C:\Users\Ana\Documents\nota.txt"), Category::Documentos);
    assert_eq!(f(r"C:\Users\Ana\Documents\viagem.mp4"), Category::Midia);
    assert_eq!(f(r"C:\Users\Ana\Documents\app.exe"), Category::Documentos);
    assert_eq!(f(r"C:\Users\Ana\AppData\Local\Programs\Code\Code.exe"), Category::Apps);
    assert_eq!(f(r"C:\Users\Ana\AppData\Roaming\Discord\settings.json"), Category::DadosApps);
    assert_eq!(f(r"C:\Users\Ana\AppData\Roaming\Discord\Cache\data_1"), Category::Cache);
    assert_eq!(f(r"C:\Users\Ana\AppData\Local\Temp\a.log"), Category::Cache);
    assert_eq!(f(r"C:\Users\Ana\Pictures\foto.JPG"), Category::Midia);
    assert_eq!(f(r"C:\Users\Ana\Desktop\coisa.bin"), Category::Outros);
}

#[test]
fn games_and_dev() {
    let c = Classifier::standard(&known());
    assert_eq!(f(r"D:\SteamLibrary\steamapps\common\Hades\Hades.exe"), Category::Jogos);
    assert_eq!(f(r"D:\XboxGames\Forza\a.pak"), Category::Jogos);
    assert_eq!(f(r"C:\Program Files\7-Zip\7z.exe"), Category::Apps);
    assert_eq!(classify_path(&c, r"C:\Users\Ana\repos\site", true, &[".git"], &[]), Category::Desenvolvimento);
    assert_eq!(f(r"C:\Users\Ana\Downloads\proj\node_modules\x\i.js"), Category::Desenvolvimento);
    assert_eq!(classify_path(&c, r"C:\code\app\target", true, &[], &["Cargo.toml"]), Category::Desenvolvimento);
    assert_eq!(classify_path(&c, r"C:\code\app\target", true, &[], &[]), Category::Outros);
    assert_eq!(f(r"C:\Windows\node_modules\a.js"), Category::Sistema);
}

#[test]
fn case_insensitive() {
    assert_eq!(f(r"c:\WINDOWS\system32\a.dll"), Category::Sistema);
}
