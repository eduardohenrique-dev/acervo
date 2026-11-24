use std::path::PathBuf;

/// Lê o `libraryfolders.vdf` da Steam e devolve `<biblioteca>\steamapps\common`.
pub fn parse_steam_libraryfolders(vdf: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut rest = vdf;
    while let Some(i) = rest.find("\"path\"") {
        rest = &rest[i + 6..];
        let trimmed = rest.trim_start();
        let Some(body) = trimmed.strip_prefix('"') else { continue };
        let mut value = String::new();
        let mut chars = body.chars();
        let mut closed = false;
        while let Some(ch) = chars.next() {
            match ch {
                '\\' => {
                    if let Some(next) = chars.next() {
                        value.push(next);
                    }
                }
                '"' => {
                    closed = true;
                    break;
                }
                _ => value.push(ch),
            }
        }
        if closed && !value.is_empty() {
            out.push(PathBuf::from(value).join("steamapps").join("common"));
        }
        rest = chars.as_str();
    }
    out
}

/// Lê um manifesto `.item` da Epic e devolve o `InstallLocation`.
pub fn parse_epic_manifest(json: &str) -> Option<PathBuf> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let loc = v.get("InstallLocation")?.as_str()?;
    (!loc.is_empty()).then(|| PathBuf::from(loc))
}

/// Lê o `RiotClientInstalls.json` e devolve as pastas dos jogos.
pub fn parse_riot_installs(json: &str) -> Vec<PathBuf> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let Some(map) = v.get("associated_client").and_then(|m| m.as_object()) else {
        return Vec::new();
    };
    map.keys()
        .map(|k| k.replace('/', "\\"))
        .map(|k| k.trim_end_matches('\\').to_string())
        .filter(|k| !k.is_empty())
        .map(PathBuf::from)
        .collect()
}
