use std::path::PathBuf;

/// The database path when none is chosen in Settings, in the user data directory.
pub fn default_db_path() -> PathBuf {
    match directories::ProjectDirs::from("", "", "hikestat") {
        Some(dirs) => dirs.data_dir().join("hikestat.db"),
        None => PathBuf::from("hikestat.db"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_in_a_hikestat_folder() {
        assert!(default_db_path().ends_with("hikestat/hikestat.db"));
    }
}
