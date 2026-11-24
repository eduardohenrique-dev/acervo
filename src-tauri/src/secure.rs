//! Protege a pasta dos snapshots: o app roda como administrador e lê o que estiver
//! nela, então usuários comuns não podem conseguir gravar ali.

use std::ffi::OsString;
use std::path::Path;

const SYSTEM: &str = "*S-1-5-18";
const ADMINISTRATORS: &str = "*S-1-5-32-544";
/// Usuários, Todos, Usuários autenticados e CRIADOR PROPRIETÁRIO.
const UNTRUSTED: [&str; 4] = ["*S-1-5-32-545", "*S-1-1-0", "*S-1-5-11", "*S-1-3-0"];

/// As chamadas do `icacls`, em ordem: herança cortada e acesso total só para
/// SYSTEM e Administradores; grupos comuns removidos; dono = Administradores.
pub fn icacls_calls(dir: &Path) -> Vec<Vec<OsString>> {
    let d = || OsString::from(dir.as_os_str());
    let s = |x: &str| OsString::from(x);
    let mut remove = vec![d(), s("/remove:g")];
    remove.extend(UNTRUSTED.iter().map(|x| s(x)));
    remove.extend([s("/T"), s("/C"), s("/Q")]);
    vec![
        vec![
            d(),
            s("/inheritance:r"),
            s("/grant:r"),
            s(&format!("{SYSTEM}:(OI)(CI)F")),
            s(&format!("{ADMINISTRATORS}:(OI)(CI)F")),
            s("/T"),
            s("/C"),
            s("/Q"),
        ],
        remove,
        vec![d(), s("/setowner"), s(ADMINISTRATORS), s("/T"), s("/C"), s("/Q")],
    ]
}

/// Cria a pasta (se preciso) e aplica as permissões. Erro → o app segue sem snapshot.
pub fn secure_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::fs::create_dir_all(dir)?;
    for args in icacls_calls(dir) {
        let status = std::process::Command::new("icacls").args(args).creation_flags(CREATE_NO_WINDOW).status()?;
        if !status.success() {
            return Err(std::io::Error::other(format!("icacls terminou com {status}")));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_system_and_admins_keep_access() {
        let calls = icacls_calls(Path::new(r"C:\ProgramData\Acervo"));
        let text: Vec<String> = calls.iter().map(|c| c.iter().map(|a| a.to_string_lossy().into_owned()).collect::<Vec<_>>().join(" ")).collect();
        assert_eq!(text.len(), 3);
        assert!(text[0].starts_with(r"C:\ProgramData\Acervo /inheritance:r /grant:r *S-1-5-18:(OI)(CI)F *S-1-5-32-544:(OI)(CI)F"));
        assert!(text[1].contains("/remove:g *S-1-5-32-545 *S-1-1-0 *S-1-5-11 *S-1-3-0"));
        assert!(text[2].contains("/setowner *S-1-5-32-544"));
        assert!(text.iter().all(|t| t.contains("/T /C")));
    }
}
