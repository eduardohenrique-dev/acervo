use acervo_classify::{parse_epic_manifest, parse_riot_installs, parse_steam_libraryfolders};
use std::path::PathBuf;

const VDF: &str = r#""libraryfolders"
{
	"0" { "path"		"C:\\Program Files (x86)\\Steam" "apps" { "228980" "1" } }
	"1" { "path"		"D:\\SteamLibrary" }
}"#;

#[test]
fn steam() {
    assert_eq!(
        parse_steam_libraryfolders(VDF),
        vec![
            PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common"),
            PathBuf::from(r"D:\SteamLibrary\steamapps\common")
        ]
    );
    assert!(parse_steam_libraryfolders("lixo").is_empty());
}

#[test]
fn epic() {
    let j = r#"{"DisplayName":"Fortnite","InstallLocation":"D:\\Epic Games\\Fortnite"}"#;
    assert_eq!(parse_epic_manifest(j), Some(PathBuf::from(r"D:\Epic Games\Fortnite")));
    assert_eq!(parse_epic_manifest("{}"), None);
}

#[test]
fn riot() {
    let j = r#"{"associated_client":{"C:/Riot Games/League of Legends/":"C:/Riot Games/Riot Client/RiotClientServices.exe"},"rc_default":"x"}"#;
    assert_eq!(parse_riot_installs(j), vec![PathBuf::from(r"C:\Riot Games\League of Legends")]);
}
