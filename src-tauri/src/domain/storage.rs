use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use rusqlite_migration::{M, Migrations};

use crate::domain::Result;
use crate::domain::parser::{Point, Trace};
use crate::domain::stats::{self, STATS_VERSION, Stats};
use crate::domain::timezone;

const MIGRATIONS: &[M<'static>] = &[
    M::up(
        "CREATE TABLE hikes (
        id INTEGER PRIMARY KEY,
        name TEXT,
        started_at INTEGER,
        distance_m REAL NOT NULL,
        duration_total_s INTEGER
    );
    CREATE TABLE track_points (
        id INTEGER PRIMARY KEY,
        hike_id INTEGER NOT NULL REFERENCES hikes(id) ON DELETE CASCADE,
        segment_idx INTEGER NOT NULL,
        idx INTEGER NOT NULL,
        lat REAL NOT NULL,
        lon REAL NOT NULL,
        ele REAL,
        time INTEGER
    );
    CREATE INDEX track_points_hike ON track_points(hike_id, segment_idx, idx);",
    ),
    // Rows from the first migration keep a NULL stats_version: they predate the full stats.
    M::up(
        "ALTER TABLE hikes ADD COLUMN ascent_m REAL;
    ALTER TABLE hikes ADD COLUMN descent_m REAL;
    ALTER TABLE hikes ADD COLUMN min_ele_m REAL;
    ALTER TABLE hikes ADD COLUMN max_ele_m REAL;
    ALTER TABLE hikes ADD COLUMN duration_moving_s INTEGER;
    ALTER TABLE hikes ADD COLUMN avg_speed_mps REAL;
    ALTER TABLE hikes ADD COLUMN stats_version INTEGER;",
    ),
    // Rows imported before this migration keep a NULL hash; NULLs do not collide in a unique index.
    M::up(
        "ALTER TABLE hikes ADD COLUMN trace_hash TEXT;
    CREATE UNIQUE INDEX hikes_trace_hash ON hikes(trace_hash);",
    ),
    M::up("ALTER TABLE hikes ADD COLUMN kind TEXT;"),
    // Rows imported before this migration keep a NULL zone until the next `recompute`.
    M::up("ALTER TABLE hikes ADD COLUMN start_tz TEXT;"),
    // Set by the user only (see `Db::set_title` and `Db::set_note`); import and recompute never
    // write them.
    M::up(
        "ALTER TABLE hikes ADD COLUMN title TEXT;
    ALTER TABLE hikes ADD COLUMN note TEXT;",
    ),
];

/// How long a connection waits for another one's lock before failing with "database is locked".
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inserted {
    Created(i64),
    /// A hike with the same `trace_hash` already exists; nothing was written.
    AlreadyPresent,
}

/// A row of `hikes`, without its points. Units and nullability follow the schema.
#[derive(Debug, Clone, PartialEq)]
pub struct Hike {
    pub id: i64,
    pub name: Option<String>,
    pub kind: Option<String>,
    /// Seconds since the Unix epoch (UTC).
    pub started_at: Option<i64>,
    /// IANA time zone name, e.g. `Europe/Paris`.
    pub start_tz: Option<String>,
    pub distance_m: f64,
    pub ascent_m: Option<f64>,
    pub descent_m: Option<f64>,
    pub min_ele_m: Option<f64>,
    pub max_ele_m: Option<f64>,
    pub duration_total_s: Option<i64>,
    pub duration_moving_s: Option<i64>,
    pub avg_speed_mps: Option<f64>,
    pub stats_version: Option<i64>,
    /// Chosen by the user; displayed instead of `name` when set.
    pub title: Option<String>,
    /// Free text written by the user.
    pub note: Option<String>,
}

pub struct Db {
    conn: Connection,
}

impl Db {
    /// Opens (creating parent directories and the file if needed) and migrates.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|source| crate::domain::Error::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        Self::migrate(Connection::open(path)?)
    }

    /// Like [`Db::open`], but never creates anything: a missing file is
    /// [`Error::DatabaseNotFound`](crate::domain::Error::DatabaseNotFound).
    pub fn open_existing(path: &Path) -> Result<Self> {
        let exists = path
            .try_exists()
            .map_err(|source| crate::domain::Error::Io {
                path: path.to_path_buf(),
                source,
            })?;
        if !exists {
            return Err(crate::domain::Error::DatabaseNotFound {
                path: path.to_path_buf(),
            });
        }
        // Without the create flag, a file removed since the check is an error, not a new database.
        let flags = OpenFlags::default().difference(OpenFlags::SQLITE_OPEN_CREATE);
        Self::migrate(Connection::open_with_flags(path, flags)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::migrate(Connection::open_in_memory()?)
    }

    fn migrate(mut conn: Connection) -> Result<Self> {
        conn.busy_timeout(BUSY_TIMEOUT)?;
        // Checked before anything is written: a newer database must stay exactly as it is.
        let found: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let supported = MIGRATIONS.len() as i64;
        if found > supported {
            return Err(crate::domain::Error::DatabaseTooNew { found, supported });
        }
        // Readers never wait for a writer (the desktop app shares the file). In memory, the mode
        // stays "memory".
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        Migrations::from_slice(MIGRATIONS).to_latest(&mut conn)?;
        Ok(Self { conn })
    }

    /// Inserts the hike and all its points in a single transaction, unless a hike with the same
    /// `trace_hash` already exists.
    pub fn insert_hike(
        &mut self,
        trace: &Trace,
        stats: &Stats,
        trace_hash: &str,
        kind: Option<&str>,
    ) -> Result<Inserted> {
        let started_at = trace.segments.iter().flatten().find_map(|p| p.time);
        // A savepoint rather than a transaction so it also works inside `rolled_back`.
        let tx = self.conn.savepoint()?;
        let changed = tx.execute(
            "INSERT INTO hikes (name, started_at, distance_m, ascent_m, descent_m, min_ele_m,
                                max_ele_m, duration_total_s, duration_moving_s, avg_speed_mps,
                                stats_version, trace_hash, kind, start_tz)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(trace_hash) DO NOTHING",
            params![
                trace.name,
                started_at,
                stats.distance_m,
                stats.ascent_m,
                stats.descent_m,
                stats.min_ele_m,
                stats.max_ele_m,
                stats.duration_total_s,
                stats.duration_moving_s,
                stats.avg_speed_mps,
                STATS_VERSION,
                trace_hash,
                kind,
                start_tz(trace),
            ],
        )?;
        if changed == 0 {
            return Ok(Inserted::AlreadyPresent);
        }
        let hike_id = tx.last_insert_rowid();
        {
            let mut stmt = tx.prepare(
                "INSERT INTO track_points (hike_id, segment_idx, idx, lat, lon, ele, time)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for (seg_idx, seg) in trace.segments.iter().enumerate() {
                for (idx, p) in seg.iter().enumerate() {
                    stmt.execute(params![
                        hike_id,
                        seg_idx as i64,
                        idx as i64,
                        p.lat,
                        p.lon,
                        p.ele,
                        p.time
                    ])?;
                }
            }
        }
        tx.commit()?;
        Ok(Inserted::Created(hike_id))
    }

    /// Recomputes every hike's stats and start time zone from its stored points, in a single
    /// transaction. Only rows whose stats, `stats_version` or zone differ are written, and only
    /// those columns; points, name, kind, title, note, date and hash are never touched.
    /// Returns the number of rows written.
    pub fn recompute_stats(&mut self) -> Result<usize> {
        let tx = self.conn.savepoint()?;
        let ids = tx
            .prepare("SELECT id FROM hikes ORDER BY id")?
            .query_map([], |r| r.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut updated = 0;
        for id in ids {
            let trace = load_trace(&tx, id)?;
            let (fresh, fresh_tz) = (stats::compute(&trace), start_tz(&trace));
            let (stored, version, stored_tz) = load_stats(&tx, id)?;
            if stored == fresh && version == Some(STATS_VERSION) && stored_tz.as_deref() == fresh_tz
            {
                continue;
            }
            tx.execute(
                "UPDATE hikes SET distance_m = ?2, ascent_m = ?3, descent_m = ?4, min_ele_m = ?5,
                                  max_ele_m = ?6, duration_total_s = ?7, duration_moving_s = ?8,
                                  avg_speed_mps = ?9, stats_version = ?10, start_tz = ?11
                 WHERE id = ?1",
                params![
                    id,
                    fresh.distance_m,
                    fresh.ascent_m,
                    fresh.descent_m,
                    fresh.min_ele_m,
                    fresh.max_ele_m,
                    fresh.duration_total_s,
                    fresh.duration_moving_s,
                    fresh.avg_speed_mps,
                    STATS_VERSION,
                    fresh_tz,
                ],
            )?;
            updated += 1;
        }
        tx.commit()?;
        Ok(updated)
    }

    /// Every hike, without its points, ordered by `started_at` (hikes without one first), then
    /// by `id`.
    pub fn list_hikes(&self) -> Result<Vec<Hike>> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {HIKE_COLUMNS} FROM hikes ORDER BY started_at, id"
        ))?;
        let hikes = stmt
            .query_map([], hike_from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(hikes)
    }

    /// The hike with this `id`, without its points, or `None` if there is none.
    pub fn hike(&self, id: i64) -> Result<Option<Hike>> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {HIKE_COLUMNS} FROM hikes WHERE id = ?1"),
                [id],
                hike_from_row,
            )
            .optional()?)
    }

    /// The raw points of hike `id`, grouped by segment, segments and points in order, or `None`
    /// if there is no such hike. A hike without points has an empty track.
    pub fn track(&self, id: i64) -> Result<Option<Vec<Vec<Point>>>> {
        let exists = self
            .conn
            .query_row("SELECT 1 FROM hikes WHERE id = ?1", [id], |_| Ok(()))
            .optional()?
            .is_some();
        if !exists {
            return Ok(None);
        }
        Ok(Some(load_trace(&self.conn, id)?.segments))
    }

    /// The distinct kinds of the hikes, `NULL` excluded, exactly as stored (no case folding:
    /// "Walking" and "walking" are two kinds). Sorted case-insensitively, ties broken by the
    /// exact value, so that a near-duplicate sits next to its twin.
    pub fn kinds(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT kind FROM hikes WHERE kind IS NOT NULL")?;
        let mut kinds = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        kinds.sort_by_cached_key(|k| (k.to_lowercase(), k.clone()));
        Ok(kinds)
    }

    /// Sets the title of hike `id`, or clears it with `None`. The value is trimmed; an empty or
    /// whitespace-only value also clears it (stored as NULL). The raw `name` is never touched.
    /// An unknown id is [`Error::HikeNotFound`](crate::domain::Error::HikeNotFound).
    pub fn set_title(&self, id: i64, title: Option<&str>) -> Result<()> {
        self.set_user_text(id, "title", title)
    }

    /// Sets the note of hike `id`, or clears it with `None`. Leading and trailing whitespace is
    /// trimmed, inner line breaks are kept; an empty or whitespace-only value also clears it
    /// (stored as NULL). An unknown id is [`Error::HikeNotFound`](crate::domain::Error::HikeNotFound).
    pub fn set_note(&self, id: i64, note: Option<&str>) -> Result<()> {
        self.set_user_text(id, "note", note)
    }

    /// Sets the kind of hike `id`, in place (the file's `<trk><type>` is not kept apart). The value
    /// is trimmed and otherwise stored exactly (no case folding); an empty or whitespace-only value
    /// clears it (stored as NULL). An unknown id is [`Error::HikeNotFound`](crate::domain::Error::HikeNotFound).
    pub fn set_kind(&self, id: i64, kind: &str) -> Result<()> {
        self.set_user_text(id, "kind", Some(kind))
    }

    /// `column` is one of our own column names, never user input.
    fn set_user_text(&self, id: i64, column: &str, value: Option<&str>) -> Result<()> {
        let value = value.map(str::trim).filter(|v| !v.is_empty());
        let changed = self.conn.execute(
            &format!("UPDATE hikes SET {column} = ?2 WHERE id = ?1"),
            params![id, value],
        )?;
        if changed == 0 {
            return Err(crate::domain::Error::HikeNotFound(id));
        }
        Ok(())
    }

    /// Runs `f` inside a transaction that is always rolled back, so nothing it writes is kept.
    pub fn rolled_back<T>(&mut self, f: impl FnOnce(&mut Db) -> T) -> Result<T> {
        self.conn.execute_batch("BEGIN")?;
        let out = f(self);
        self.conn.execute_batch("ROLLBACK")?;
        Ok(out)
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

/// The columns read by [`hike_from_row`], in its order.
const HIKE_COLUMNS: &str = "id, name, kind, started_at, start_tz, distance_m, ascent_m, descent_m,
     min_ele_m, max_ele_m, duration_total_s, duration_moving_s, avg_speed_mps, stats_version,
     title, note";

fn hike_from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Hike> {
    Ok(Hike {
        id: r.get(0)?,
        name: r.get(1)?,
        kind: r.get(2)?,
        started_at: r.get(3)?,
        start_tz: r.get(4)?,
        distance_m: r.get(5)?,
        ascent_m: r.get(6)?,
        descent_m: r.get(7)?,
        min_ele_m: r.get(8)?,
        max_ele_m: r.get(9)?,
        duration_total_s: r.get(10)?,
        duration_moving_s: r.get(11)?,
        avg_speed_mps: r.get(12)?,
        stats_version: r.get(13)?,
        title: r.get(14)?,
        note: r.get(15)?,
    })
}

fn load_trace(conn: &Connection, hike_id: i64) -> Result<Trace> {
    let mut stmt = conn.prepare(
        "SELECT segment_idx, lat, lon, ele, time FROM track_points
         WHERE hike_id = ?1 ORDER BY segment_idx, idx",
    )?;
    let rows = stmt.query_map([hike_id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            Point {
                lat: r.get(1)?,
                lon: r.get(2)?,
                ele: r.get(3)?,
                time: r.get(4)?,
            },
        ))
    })?;
    let mut segments: Vec<Vec<Point>> = Vec::new();
    let mut current = None;
    for row in rows {
        let (segment_idx, point) = row?;
        if current != Some(segment_idx) {
            segments.push(Vec::new());
            current = Some(segment_idx);
        }
        segments
            .last_mut()
            .expect("a segment was just pushed")
            .push(point);
    }
    Ok(Trace {
        name: None,
        kind: None,
        segments,
    })
}

/// Zone of the first timestamped point: the same point that gives `started_at`.
fn start_tz(trace: &Trace) -> Option<&'static str> {
    let start = trace.segments.iter().flatten().find(|p| p.time.is_some())?;
    timezone::zone_at(start.lat, start.lon)
}

fn load_stats(conn: &Connection, hike_id: i64) -> Result<(Stats, Option<i64>, Option<String>)> {
    Ok(conn.query_row(
        "SELECT distance_m, ascent_m, descent_m, min_ele_m, max_ele_m, duration_total_s,
                duration_moving_s, avg_speed_mps, stats_version, start_tz
         FROM hikes WHERE id = ?1",
        [hike_id],
        |r| {
            Ok((
                Stats {
                    distance_m: r.get(0)?,
                    ascent_m: r.get(1)?,
                    descent_m: r.get(2)?,
                    min_ele_m: r.get(3)?,
                    max_ele_m: r.get(4)?,
                    duration_total_s: r.get(5)?,
                    duration_moving_s: r.get(6)?,
                    avg_speed_mps: r.get(7)?,
                },
                r.get(8)?,
                r.get(9)?,
            ))
        },
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_valid() {
        Migrations::from_slice(MIGRATIONS).validate().unwrap();
    }

    #[test]
    fn a_failing_insert_leaves_neither_hike_nor_points() {
        use crate::domain::parser::Point;
        let ok = Point {
            lat: 50.0,
            lon: 3.0,
            ele: None,
            time: Some(1),
        };
        // NaN is bound as NULL by SQLite, which violates NOT NULL on the second point.
        let bad = Point {
            lat: f64::NAN,
            lon: 3.0,
            ele: None,
            time: Some(2),
        };
        let trace = Trace {
            name: None,
            kind: None,
            segments: vec![vec![ok, bad]],
        };
        let stats = crate::domain::stats::compute(&trace);

        let mut db = Db::open_in_memory().unwrap();
        assert!(db.insert_hike(&trace, &stats, "h", None).is_err());
        let count = |table: &str| -> i64 {
            db.connection()
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!((count("hikes"), count("track_points")), (0, 0));
    }

    fn user_version(conn: &Connection) -> i64 {
        conn.pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn a_newer_database_is_refused_and_left_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("future.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch("CREATE TABLE t (x); PRAGMA user_version = 99;")
                .unwrap();
        }
        let before = std::fs::read(&path).unwrap();

        let err = Db::open(&path).err().expect("must refuse a newer database");
        assert!(
            matches!(err, crate::domain::Error::DatabaseTooNew { found: 99, .. }),
            "got {err:?}"
        );
        assert!(err.to_string().contains("newer"), "got {err}");
        assert_eq!(std::fs::read(&path).unwrap(), before, "file was modified");
    }

    #[test]
    fn a_file_database_is_in_wal_mode_with_a_busy_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("h.db")).unwrap();
        let c = db.connection();
        let mode: String = c
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        let timeout_ms: i64 = c
            .pragma_query_value(None, "busy_timeout", |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        assert_eq!(timeout_ms, BUSY_TIMEOUT.as_millis() as i64);
    }

    #[test]
    fn a_reader_is_not_blocked_by_a_write_transaction() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("h.db");
        let writer = Db::open(&path).unwrap();
        let reader = Db::open(&path).unwrap();
        // EXCLUSIVE would lock readers out in rollback-journal mode, not in WAL mode.
        writer
            .connection()
            .execute_batch(
                "BEGIN EXCLUSIVE; INSERT INTO hikes (name, distance_m) VALUES ('pending', 1);",
            )
            .unwrap();

        let count: i64 = reader
            .connection()
            .query_row("SELECT COUNT(*) FROM hikes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "uncommitted rows are not visible");
        writer.connection().execute_batch("COMMIT").unwrap();
    }

    #[test]
    fn opening_a_missing_database_creates_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let missing_dir = dir.path().join("nowhere");
        let path = missing_dir.join("h.db");

        let err = Db::open_existing(&path)
            .err()
            .expect("must refuse a missing database");
        assert!(
            matches!(&err, crate::domain::Error::DatabaseNotFound { path: p } if *p == path),
            "got {err:?}"
        );
        assert!(!missing_dir.exists(), "directory was created");
    }

    #[test]
    fn opening_an_existing_database_applies_pending_migrations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.db");
        {
            let mut conn = Connection::open(&path).unwrap();
            Migrations::from_slice(&MIGRATIONS[..1])
                .to_latest(&mut conn)
                .unwrap();
        }

        let db = Db::open_existing(&path).unwrap();
        assert_eq!(user_version(db.connection()), MIGRATIONS.len() as i64);
    }

    #[test]
    fn opening_an_existing_newer_database_is_refused_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("future.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch("CREATE TABLE t (x); PRAGMA user_version = 99;")
                .unwrap();
        }
        let before = std::fs::read(&path).unwrap();

        let err = Db::open_existing(&path)
            .err()
            .expect("must refuse a newer database");
        assert!(
            matches!(err, crate::domain::Error::DatabaseTooNew { found: 99, .. }),
            "got {err:?}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before, "file was modified");
        assert!(
            !dir.path().join("future.db-wal").exists(),
            "WAL was created"
        );
    }

    #[test]
    fn an_older_database_is_migrated_without_losing_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.db");
        {
            let mut conn = Connection::open(&path).unwrap();
            Migrations::from_slice(&MIGRATIONS[..1])
                .to_latest(&mut conn)
                .unwrap();
            conn.execute(
                "INSERT INTO hikes (name, distance_m) VALUES ('kept', 1234.5)",
                [],
            )
            .unwrap();
            assert_eq!(user_version(&conn), 1);
        }

        let db = Db::open(&path).unwrap();
        let c = db.connection();
        assert_eq!(user_version(c), MIGRATIONS.len() as i64);
        let (name, dist, version): (String, f64, Option<i64>) = c
            .query_row(
                "SELECT name, distance_m, stats_version FROM hikes",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!((name.as_str(), dist, version), ("kept", 1234.5, None));
    }

    #[test]
    fn a_database_without_start_time_zones_is_backfilled_by_recompute() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("before_tz.db");
        let previous = MIGRATIONS.len() - 1;
        {
            let mut conn = Connection::open(&path).unwrap();
            Migrations::from_slice(&MIGRATIONS[..previous])
                .to_latest(&mut conn)
                .unwrap();
            // Tokyo: the first point has no time, so the zone comes from the second one.
            conn.execute_batch(
                "INSERT INTO hikes (id, name, kind, started_at, distance_m, trace_hash)
                 VALUES (1, 'kept', 'walking', 1000, 0, 'h');
                 INSERT INTO track_points (hike_id, segment_idx, idx, lat, lon, ele, time) VALUES
                 (1, 0, 0, 48.8566, 2.3522, NULL, NULL),
                 (1, 0, 1, 35.6762, 139.6503, NULL, 1000);",
            )
            .unwrap();
        }

        let mut db = Db::open(&path).unwrap();
        let row = |db: &Db| -> (String, String, i64, String, Option<String>) {
            db.connection()
                .query_row(
                    "SELECT name, kind, started_at, trace_hash, start_tz FROM hikes",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )
                .unwrap()
        };
        assert_eq!(row(&db).4, None, "migrating alone derives nothing");

        assert_eq!(db.recompute_stats().unwrap(), 1);
        assert_eq!(
            row(&db),
            (
                "kept".into(),
                "walking".into(),
                1000,
                "h".into(),
                Some("Asia/Tokyo".into())
            )
        );
        assert_eq!(db.recompute_stats().unwrap(), 0);
    }

    #[test]
    fn a_database_without_title_and_note_is_migrated_with_both_null() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("before_title.db");
        let previous = MIGRATIONS.len() - 1;
        {
            let mut conn = Connection::open(&path).unwrap();
            Migrations::from_slice(&MIGRATIONS[..previous])
                .to_latest(&mut conn)
                .unwrap();
            conn.execute(
                "INSERT INTO hikes (id, name, kind, distance_m) VALUES (1, 'kept', 'walking', 0)",
                [],
            )
            .unwrap();
        }

        let db = Db::open(&path).unwrap();
        assert_eq!(user_version(db.connection()), MIGRATIONS.len() as i64);
        let h = db.hike(1).unwrap().expect("hike 1 is kept");
        assert_eq!(
            (h.name.as_deref(), h.kind.as_deref(), h.title, h.note),
            (Some("kept"), Some("walking"), None, None)
        );
        db.set_title(1, Some("Titled")).unwrap();
        assert_eq!(
            db.hike(1).unwrap().unwrap().title.as_deref(),
            Some("Titled")
        );
    }

    #[test]
    fn a_current_database_reopens_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("now.db");
        drop(Db::open(&path).unwrap());
        let before = std::fs::read(&path).unwrap();
        drop(Db::open(&path).unwrap());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}
