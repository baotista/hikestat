//! Whether the database can be opened, and what it contains.

use std::path::Path;

use hikestat::storage::Db;
use serde::Serialize;
use specta::Type;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum DbStatus {
    Ready {
        path: String,
        #[serde(rename = "hikeCount")]
        hike_count: u32,
    },
    /// No file at `path`; nothing was created.
    Missing { path: String },
    /// The database was written by a newer `hikestat`; nothing was written.
    TooNew { path: String },
    /// Any other failure (not a database, permissions…).
    Error { path: String, message: String },
}

/// Opens the database at `path` without ever creating it, and reports its status.
pub fn status_of(path: &Path) -> DbStatus {
    let shown = path.display().to_string();
    match Db::open_existing(path).and_then(|db| count_hikes(&db)) {
        Ok(hike_count) => DbStatus::Ready {
            path: shown,
            hike_count,
        },
        Err(hikestat::Error::DatabaseNotFound { .. }) => DbStatus::Missing { path: shown },
        Err(hikestat::Error::DatabaseTooNew { .. }) => DbStatus::TooNew { path: shown },
        Err(err) => DbStatus::Error {
            path: shown,
            message: err.to_string(),
        },
    }
}

// Raw SQL until the library gets its read API (issue #3); it will move there.
fn count_hikes(db: &Db) -> hikestat::Result<u32> {
    Ok(db
        .connection()
        .query_row("SELECT count(*) FROM hikes", [], |r| r.get(0))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_database_is_reported_and_not_created() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nowhere").join("hikestat.db");

        assert_eq!(
            status_of(&path),
            DbStatus::Missing {
                path: path.display().to_string()
            }
        );
        assert!(!path.exists(), "the database was created");
        assert!(
            !dir.path().join("nowhere").exists(),
            "its directory was created"
        );
    }

    #[test]
    fn an_existing_database_reports_its_hike_count() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hikestat.db");
        {
            let db = Db::open(&path).unwrap();
            db.connection()
                .execute_batch(
                    "INSERT INTO hikes (name, distance_m) VALUES ('a', 1000), ('b', 2000);",
                )
                .unwrap();
        }

        assert_eq!(
            status_of(&path),
            DbStatus::Ready {
                path: path.display().to_string(),
                hike_count: 2
            }
        );
    }

    #[test]
    fn an_empty_database_is_ready_with_no_hikes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hikestat.db");
        drop(Db::open(&path).unwrap());

        assert!(matches!(
            status_of(&path),
            DbStatus::Ready { hike_count: 0, .. }
        ));
    }

    #[test]
    fn a_newer_database_is_reported_and_left_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("future.db");
        {
            let db = Db::open(&path).unwrap();
            // Same trick as the library's own test: a schema version it doesn't know yet.
            db.connection()
                .execute_batch("PRAGMA journal_mode = DELETE; PRAGMA user_version = 99;")
                .unwrap();
        }
        let before = std::fs::read(&path).unwrap();

        assert_eq!(
            status_of(&path),
            DbStatus::TooNew {
                path: path.display().to_string()
            }
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            before,
            "the file was modified"
        );
        assert!(
            !dir.path().join("future.db-wal").exists(),
            "a WAL was created"
        );
    }

    #[test]
    fn a_file_that_is_not_a_database_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, "definitely not SQLite, just some plain text here").unwrap();

        assert!(matches!(status_of(&path), DbStatus::Error { .. }));
    }
}
