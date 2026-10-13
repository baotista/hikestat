//! Where the database is: the file chosen in the app's settings, otherwise the default.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The app's persisted settings (`settings.json` in the app's config directory).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// The database file chosen by the user; `None` means the default path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub db_path: Option<PathBuf>,
}

/// The default database path, in the user data directory.
pub fn default_db_path() -> PathBuf {
    crate::domain::config::default_db_path()
}

/// The chosen file if there is one, otherwise `default`.
pub fn resolve(settings: &Settings, default: PathBuf) -> PathBuf {
    settings.db_path.clone().unwrap_or(default)
}

/// Reads the settings; a missing, unreadable or invalid file means default settings.
pub fn load_settings(file: &Path) -> Settings {
    std::fs::read(file)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Writes the settings, creating the config directory if needed.
pub fn save_settings(file: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(settings).map_err(std::io::Error::other)?;
    std::fs::write(file, json)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default() -> PathBuf {
        PathBuf::from("/default/hikestat.db")
    }

    #[test]
    fn a_chosen_file_wins_over_the_default() {
        let settings = Settings {
            db_path: Some("/elsewhere/mine.db".into()),
        };
        assert_eq!(
            resolve(&settings, default()),
            PathBuf::from("/elsewhere/mine.db")
        );
    }

    #[test]
    fn without_a_chosen_file_the_default_is_used() {
        assert_eq!(resolve(&Settings::default(), default()), default());
    }

    #[test]
    fn missing_settings_mean_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let settings = load_settings(&dir.path().join("settings.json"));
        assert_eq!(resolve(&settings, default()), default());
    }

    #[test]
    fn unreadable_settings_mean_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.json");
        std::fs::write(&file, "{ not json").unwrap();
        assert_eq!(resolve(&load_settings(&file), default()), default());

        // A directory where the file should be cannot be read either.
        let not_a_file = dir.path().join("dir.json");
        std::fs::create_dir(&not_a_file).unwrap();
        assert_eq!(load_settings(&not_a_file), Settings::default());
    }

    #[test]
    fn saved_settings_are_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config").join("settings.json");
        let settings = Settings {
            db_path: Some("/elsewhere/mine.db".into()),
        };
        save_settings(&file, &settings).unwrap();
        assert_eq!(load_settings(&file), settings);

        save_settings(&file, &Settings::default()).unwrap();
        assert_eq!(load_settings(&file), Settings::default());
    }
}
