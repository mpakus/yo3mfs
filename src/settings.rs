use crate::i18n::{Key, Language, Message};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub const DARK: [u8; 3] = [32, 36, 43];
pub const LIGHT: [u8; 3] = [242, 244, 247];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    pub version: u32,
    pub background: [u8; 3],
    #[serde(default)]
    pub language: Language,
    #[serde(default = "project_colors_default")]
    pub project_colors: bool,
}
fn project_colors_default() -> bool {
    true
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            background: DARK,
            language: Language::English,
            project_colors: true,
        }
    }
}
pub fn path() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "yo3mfs", "yo3mfs")
        .map(|d| d.config_dir().join("settings.json"))
}
pub fn read(path: &Path) -> (Settings, Option<Message>) {
    match fs::metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Settings::default(), None),
        Ok(m) if m.len() <= 64 * 1024 => {}
        _ => {
            return (Settings::default(), Some(Key::SettingsUnreadable.into()));
        }
    }
    let result = fs::read(path)
        .ok()
        .and_then(|data| serde_json::from_slice::<Settings>(&data).ok())
        .filter(|s| s.version == 1);
    match result {
        Some(s) => (s, None),
        None => (Settings::default(), Some(Key::SettingsCorrupt.into())),
    }
}
pub fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("Settings directory not found"))?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut temporary, settings)?;
    temporary.write_all(b"\n")?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|e| e.error)?;
    Ok(())
}
pub fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}
pub fn parse_hex(value: &str) -> Option<[u8; 3]> {
    let value = value.strip_prefix('#')?;
    if value.len() != 6 || !value.is_ascii() {
        return None;
    }
    Some([
        u8::from_str_radix(&value[0..2], 16).ok()?,
        u8::from_str_radix(&value[2..4], 16).ok()?,
        u8::from_str_radix(&value[4..6], 16).ok()?,
    ])
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn atomic_settings_roundtrip_and_corrupt_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("prefs/settings.json");
        assert_eq!(read(&p), (Settings::default(), None));
        let settings = Settings {
            background: [1, 2, 3],
            ..Settings::default()
        };
        save(&p, &settings).unwrap();
        assert_eq!(read(&p), (settings, None));
        save(&p, &Settings::default()).unwrap();
        assert_eq!(read(&p).0, Settings::default());
        fs::write(&p, "broken").unwrap();
        assert!(read(&p).1.is_some());
    }
    #[test]
    fn hex_validates_before_slicing() {
        assert_eq!(parse_hex("#aAbBcC"), Some([170, 187, 204]));
        for value in ["112233", "#zz0000", "#12", "#ааббвв"] {
            assert!(parse_hex(value).is_none());
        }
    }
}
