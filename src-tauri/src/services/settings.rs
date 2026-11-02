use std::fs;
use std::path::Path;

use crate::error::AppResult;
use crate::models::settings::Settings;

pub const SETTINGS_FILE: &str = "settings.json";

/// Loads settings from `dir`, falling back to defaults if the file does not exist.
pub fn load(dir: &Path) -> AppResult<Settings> {
    let path = dir.join(SETTINGS_FILE);
    if !path.exists() {
        return Ok(Settings::default());
    }
    let raw = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

/// Persists settings to `dir`, creating the directory if needed.
pub fn save(dir: &Path, settings: &Settings) -> AppResult<Settings> {
    fs::create_dir_all(dir)?;
    let raw = serde_json::to_string_pretty(settings)?;
    fs::write(dir.join(SETTINGS_FILE), raw)?;
    Ok(settings.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::settings::Theme;

    #[test]
    fn load_returns_defaults_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).unwrap(), Settings::default());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings {
            theme: Theme::Dark,
            notifications_enabled: false,
        };
        save(dir.path(), &settings).unwrap();
        assert_eq!(load(dir.path()).unwrap(), settings);
    }
}
