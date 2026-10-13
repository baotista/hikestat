use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid GPX: {0}")]
    Gpx(#[from] gpx::errors::GpxError),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error(
        "database schema version {found} is newer than this Hikestat supports ({supported}): \
         update Hikestat, nothing was written"
    )]
    DatabaseTooNew { found: i64, supported: i64 },
    #[error("no database at {path}")]
    DatabaseNotFound { path: PathBuf },
    #[error("no hike with id {0}")]
    HikeNotFound(i64),
    #[error("migration error: {0}")]
    Migration(#[from] rusqlite_migration::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
