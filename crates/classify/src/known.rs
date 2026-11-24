use std::path::{Path, PathBuf};

use crate::{parse_epic_manifest, parse_riot_installs, parse_steam_libraryfolders};

/// Pastas de um perfil de usuário.
#[derive(Clone, Debug)]
pub struct UserProfile {
    pub root: PathBuf,
    pub downloads: PathBuf,
    pub documents: PathBuf,
    pub appdata_roaming: PathBuf,
    pub appdata_local: PathBuf,
    pub appdata_locallow: PathBuf,
    pub temp: PathBuf,
}

/// Caminhos conhecidos da máquina, usados para montar as regras padrão.
#[derive(Clone, Debug)]
pub struct KnownPaths {
    pub windows_dir: PathBuf,
    pub program_files: Vec<PathBuf>,
    pub program_data: PathBuf,
    pub profiles: Vec<UserProfile>,
    pub game_libraries: Vec<PathBuf>,
    pub drives: Vec<char>,
}

impl KnownPaths {
    /// Impressão digital estável (FNV-1a 64) dos caminhos e da versão das regras.
    /// Muda quando a classificação de um snapshot antigo pode ter ficado errada.
    pub fn fingerprint(&self) -> u64 {
        const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0100_0000_01b3;
        let mut h = OFFSET;
        let mut feed = |bytes: &[u8]| {
            for b in bytes {
                h ^= *b as u64;
                h = h.wrapping_mul(PRIME);
            }
            h ^= 0xFF;
            h = h.wrapping_mul(PRIME);
        };
        feed(&crate::standard::RULES_VERSION.to_le_bytes());
        let mut path = |p: &Path| feed(p.to_string_lossy().to_lowercase().as_bytes());
        path(&self.windows_dir);
        self.program_files.iter().for_each(|p| path(p));
        path(&self.program_data);
        for u in &self.profiles {
            for p in [&u.root, &u.downloads, &u.documents, &u.appdata_roaming, &u.appdata_local, &u.appdata_locallow, &u.temp] {
                path(p);
            }
        }
        self.game_libraries.iter().for_each(|p| path(p));
        for d in &self.drives {
            feed(&[*d as u8]);
        }
        h
    }
}

impl UserProfile {
    fn with_defaults(root: PathBuf) -> UserProfile {
        UserProfile {
            downloads: root.join("Downloads"),
            documents: root.join("Documents"),
            appdata_roaming: root.join(r"AppData\Roaming"),
            appdata_local: root.join(r"AppData\Local"),
            appdata_locallow: root.join(r"AppData\LocalLow"),
            temp: root.join(r"AppData\Local\Temp"),
            root,
        }
    }
}

mod win {
    use std::path::PathBuf;

    use windows::core::{GUID, PCWSTR};
    use windows::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives, GetTempPathW};
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::System::SystemInformation::GetWindowsDirectoryW;
    use windows::Win32::UI::Shell::{SHGetKnownFolderPath, KF_FLAG_DEFAULT};

    const DRIVE_FIXED: u32 = 3;

    pub fn known_folder(id: &GUID) -> Option<PathBuf> {
        unsafe {
            let p = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).ok()?;
            let s = p.to_string().ok();
            CoTaskMemFree(Some(p.0 as *const _));
            s.map(PathBuf::from)
        }
    }

    pub fn windows_dir() -> PathBuf {
        let mut buf = [0u16; 260];
        let n = unsafe { GetWindowsDirectoryW(Some(&mut buf)) } as usize;
        if n == 0 || n > buf.len() {
            return PathBuf::from(r"C:\Windows");
        }
        PathBuf::from(String::from_utf16_lossy(&buf[..n]))
    }

    pub fn temp_dir() -> Option<PathBuf> {
        let mut buf = [0u16; 261];
        let n = unsafe { GetTempPathW(Some(&mut buf)) } as usize;
        (n > 0 && n <= buf.len()).then(|| {
            PathBuf::from(String::from_utf16_lossy(&buf[..n]).trim_end_matches('\\'))
        })
    }

    pub fn fixed_drives() -> Vec<char> {
        let mask = unsafe { GetLogicalDrives() };
        (0..26u8)
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| (b'A' + i) as char)
            .filter(|&d| {
                let root: Vec<u16> = format!("{d}:\\").encode_utf16().chain([0]).collect();
                unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) == DRIVE_FIXED }
            })
            .collect()
    }
}

fn current_profile() -> Option<UserProfile> {
    use windows::Win32::UI::Shell::*;
    let root = win::known_folder(&FOLDERID_Profile)?;
    let mut p = UserProfile::with_defaults(root);
    if let Some(v) = win::known_folder(&FOLDERID_Downloads) {
        p.downloads = v;
    }
    if let Some(v) = win::known_folder(&FOLDERID_Documents) {
        p.documents = v;
    }
    if let Some(v) = win::known_folder(&FOLDERID_RoamingAppData) {
        p.appdata_roaming = v;
    }
    if let Some(v) = win::known_folder(&FOLDERID_LocalAppData) {
        p.appdata_local = v;
    }
    if let Some(v) = win::known_folder(&FOLDERID_LocalAppDataLow) {
        p.appdata_locallow = v;
    }
    if let Some(v) = win::temp_dir() {
        p.temp = v;
    }
    Some(p)
}

fn other_profiles() -> Vec<PathBuf> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    let hklm = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
    let Ok(list) = hklm.open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProfileList") else {
        return Vec::new();
    };
    list.enum_keys()
        .flatten()
        .filter_map(|sid| list.open_subkey(&sid).ok())
        .filter_map(|k| k.get_value::<String, _>("ProfileImagePath").ok())
        .map(PathBuf::from)
        .filter(|p| {
            p.to_string_lossy()
                .to_lowercase()
                .starts_with(r"c:\users\")
        })
        .collect()
}

fn steam_libraries() -> Vec<PathBuf> {
    use winreg::enums::HKEY_CURRENT_USER;
    let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
    let Ok(path) = hkcu
        .open_subkey(r"Software\Valve\Steam")
        .and_then(|k| k.get_value::<String, _>("SteamPath"))
    else {
        return Vec::new();
    };
    let vdf = PathBuf::from(path.replace('/', "\\")).join(r"steamapps\libraryfolders.vdf");
    std::fs::read_to_string(vdf)
        .map(|s| parse_steam_libraryfolders(&s))
        .unwrap_or_default()
}

fn epic_libraries(program_data: &Path) -> Vec<PathBuf> {
    let dir = program_data.join(r"Epic\EpicGamesLauncher\Data\Manifests");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("item")))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|s| parse_epic_manifest(&s))
        .collect()
}

fn riot_libraries(program_data: &Path) -> Vec<PathBuf> {
    let mut out = std::fs::read_to_string(program_data.join(r"Riot Games\RiotClientInstalls.json"))
        .map(|s| parse_riot_installs(&s))
        .unwrap_or_default();
    let default = PathBuf::from(r"C:\Riot Games");
    if default.is_dir() {
        out.push(default);
    }
    out
}

/// Descobre os caminhos desta máquina. Fontes que faltam são ignoradas.
pub fn discover() -> KnownPaths {
    use windows::Win32::UI::Shell::*;
    let windows_dir = win::windows_dir();
    let program_files = [FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX86]
        .iter()
        .filter_map(win::known_folder)
        .collect::<Vec<_>>();
    let program_data =
        win::known_folder(&FOLDERID_ProgramData).unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));

    let mut profiles: Vec<UserProfile> = current_profile().into_iter().collect();
    for root in other_profiles() {
        let dup = profiles
            .iter()
            .any(|p| p.root.to_string_lossy().eq_ignore_ascii_case(&root.to_string_lossy()));
        if !dup {
            profiles.push(UserProfile::with_defaults(root));
        }
    }

    let mut game_libraries = steam_libraries();
    game_libraries.extend(epic_libraries(&program_data));
    game_libraries.extend(riot_libraries(&program_data));

    KnownPaths {
        windows_dir,
        program_files,
        program_data,
        profiles,
        game_libraries,
        drives: win::fixed_drives(),
    }
}
