//! Importing what is dropped on the window, through the library (ADR 0004), and what to tell the
//! user about it.

use std::path::{Path, PathBuf};

use hikestat::import::{self, ImportOptions, ImportSummary};
use hikestat::storage::{Db, Hike};
use jiff::civil::Date;
use serde::Serialize;
use specta::Type;

use crate::aggregate::{KindFilter, local_date, matches};

/// A trace or a whole file that was not imported.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Rejection {
    /// The path as dropped (or found in a dropped folder).
    pub file: String,
    /// 1-based position of the trace in the file; `null` when the whole file was rejected.
    pub trace_number: Option<u32>,
    pub reason: String,
}

/// What an import did, as the banner shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub imported: u32,
    pub already_present: u32,
    pub rejections: Vec<Rejection>,
    /// Things ignored on purpose (routes, waypoints, a file or folder without track).
    pub warnings: Vec<String>,
    /// Where the dashboard goes: the local date (start time zone) of the most recently started
    /// imported hike; `null` if nothing was imported (or none has a start time).
    pub anchor: Option<Date>,
    /// How many imported hikes the current kind filter hides.
    pub hidden_by_filter: u32,
}

/// `imported` are the hikes the import created.
fn report(summary: ImportSummary, imported: &[Hike], filter: &KindFilter) -> ImportReport {
    let latest = imported
        .iter()
        .filter(|h| h.started_at.is_some())
        .max_by_key(|h| h.started_at);
    ImportReport {
        anchor: latest.and_then(local_date),
        hidden_by_filter: imported.iter().filter(|h| !matches(filter, h)).count() as u32,
        imported: summary.imported as u32,
        already_present: summary.already_present as u32,
        rejections: summary
            .rejected
            .into_iter()
            .map(|r| Rejection {
                file: r.file.display().to_string(),
                trace_number: r.trace_number.map(|n| n as u32),
                reason: r.reason,
            })
            .collect(),
        warnings: summary.warnings,
    }
}

/// Imports the dropped `paths` into `db`, with the CLI's rules; `filter` is the dashboard's.
pub fn import(db: &mut Db, paths: &[PathBuf], filter: &KindFilter) -> Result<ImportReport, String> {
    let summary = import::import_paths(db, paths, &ImportOptions::default());
    let mut imported = Vec::with_capacity(summary.imported_ids.len());
    for &id in &summary.imported_ids {
        imported.extend(db.hike(id).map_err(|e| e.to_string())?);
    }
    Ok(report(summary, &imported, filter))
}

/// Imports the dropped `paths` into the database at `db_path`, creating it (and its folder) if it
/// doesn't exist yet, as `hikestat import` does (ADR 0004). A database written by a newer library
/// is refused before anything is written.
pub fn import_into(
    db_path: &Path,
    paths: &[PathBuf],
    filter: &KindFilter,
) -> Result<ImportReport, String> {
    let mut db = Db::open(db_path).map_err(|e| e.to_string())?;
    import(&mut db, paths, filter)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GPX: &str = r#"<?xml version="1.0"?>
<gpx xmlns="http://www.topografix.com/GPX/1.1" version="1.1" creator="t">
  <trk><name>Lille</name><trkseg>
    <trkpt lat="50.000" lon="3.0"><time>2024-11-25T11:00:00Z</time></trkpt>
    <trkpt lat="50.001" lon="3.0"><time>2024-11-25T11:01:00Z</time></trkpt>
  </trkseg></trk>
  <trk><name>No time</name><trkseg><trkpt lat="50.0" lon="3.0"/></trkseg></trk>
  <wpt lat="50.0" lon="3.0"/>
</gpx>"#;

    #[test]
    fn a_drop_is_imported_and_reported() {
        let dir = tempfile::tempdir().unwrap();
        let gpx = dir.path().join("walk.gpx");
        std::fs::write(&gpx, GPX).unwrap();
        let photo = dir.path().join("photo.jpg");
        std::fs::write(&photo, "jpeg").unwrap();
        let mut db = Db::open_in_memory().unwrap();

        let first = import(&mut db, &[gpx.clone(), photo.clone()], &KindFilter::All).unwrap();
        assert_eq!((first.imported, first.already_present), (1, 0));
        assert_eq!(
            first.rejections,
            vec![
                Rejection {
                    file: gpx.display().to_string(),
                    trace_number: Some(2),
                    reason: "no point has a timestamp".to_string(),
                },
                Rejection {
                    file: photo.display().to_string(),
                    trace_number: None,
                    reason: "not a .gpx file".to_string(),
                },
            ]
        );
        assert_eq!(first.warnings.len(), 1, "{:?}", first.warnings);
        assert_eq!(db.list_hikes().unwrap().len(), 1);

        let again = import(&mut db, &[gpx], &KindFilter::All).unwrap();
        assert_eq!((again.imported, again.already_present), (0, 1));
    }

    #[test]
    fn the_first_import_creates_the_database() {
        let dir = tempfile::tempdir().unwrap();
        let gpx = dir.path().join("walk.gpx");
        std::fs::write(&gpx, GPX).unwrap();
        let db_path = dir.path().join("not/yet/there/hikestat.db");

        let r = import_into(&db_path, &[gpx], &KindFilter::All).unwrap();
        assert_eq!(r.imported, 1);
        let db = Db::open_existing(&db_path).unwrap();
        assert_eq!(db.list_hikes().unwrap().len(), 1);
    }

    #[test]
    fn a_database_from_a_newer_library_is_refused_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let gpx = dir.path().join("walk.gpx");
        std::fs::write(&gpx, GPX).unwrap();
        let db_path = dir.path().join("hikestat.db");
        let user_version = |db: &Db| -> i64 {
            db.connection()
                .pragma_query_value(None, "user_version", |r| r.get(0))
                .unwrap()
        };
        // A database a later library has migrated further than this one knows.
        let db = Db::open(&db_path).unwrap();
        let newer = user_version(&db) + 1;
        db.connection()
            .pragma_update(None, "user_version", newer)
            .unwrap();
        drop(db);

        assert!(import_into(&db_path, &[gpx], &KindFilter::All).is_err());
        // Still newer than this library: nothing migrated it back or wrote to it.
        assert!(matches!(
            Db::open_existing(&db_path),
            Err(hikestat::Error::DatabaseTooNew { .. })
        ));
    }

    #[test]
    fn nothing_dropped_reports_nothing() {
        let mut db = Db::open_in_memory().unwrap();
        let r = import(&mut db, &[], &KindFilter::All).unwrap();
        assert_eq!(
            (r.imported, r.already_present, r.rejections, r.warnings),
            (0, 0, vec![], vec![])
        );
        assert_eq!((r.anchor, r.hidden_by_filter), (None, 0));
    }

    /// A trace of `kind` (if any) starting at `start` (UTC) at `lat`, `lon`.
    fn trk(kind: Option<&str>, start: &str, lat: f64, lon: f64) -> String {
        let kind = kind.map_or(String::new(), |k| format!("<type>{k}</type>"));
        format!(
            r#"<trk><name>{start}</name>{kind}<trkseg>
              <trkpt lat="{lat}" lon="{lon}"><time>{start}</time></trkpt>
              <trkpt lat="{}" lon="{lon}"><time>{start}</time></trkpt>
            </trkseg></trk>"#,
            lat + 0.001
        )
    }

    fn drop_traces(db: &mut Db, traces: &[String], filter: &KindFilter) -> ImportReport {
        let dir = tempfile::tempdir().unwrap();
        let gpx = dir.path().join("drop.gpx");
        let body = traces.concat();
        std::fs::write(
            &gpx,
            format!(r#"<?xml version="1.0"?><gpx xmlns="http://www.topografix.com/GPX/1.1" version="1.1" creator="t">{body}</gpx>"#),
        )
        .unwrap();
        import(db, &[gpx], filter).unwrap()
    }

    #[test]
    fn the_anchor_is_the_local_date_of_the_most_recently_started_imported_hike() {
        let mut db = Db::open_in_memory().unwrap();
        // Tokyo, 15:30 UTC on the 25th is 00:30 on the 26th there; Lille is earlier.
        let r = drop_traces(
            &mut db,
            &[
                trk(None, "2024-11-25T15:30:00Z", 35.68, 139.76),
                trk(None, "2024-11-25T10:00:00Z", 50.6, 3.0),
            ],
            &KindFilter::All,
        );
        assert_eq!(r.anchor, Some(jiff::civil::date(2024, 11, 26)));
    }

    #[test]
    fn duplicates_give_no_anchor() {
        let mut db = Db::open_in_memory().unwrap();
        let lille = [trk(None, "2024-11-25T10:00:00Z", 50.6, 3.0)];
        drop_traces(&mut db, &lille, &KindFilter::All);
        let again = drop_traces(&mut db, &lille, &KindFilter::All);
        assert_eq!((again.already_present, again.anchor), (1, None));
    }

    #[test]
    fn the_hikes_the_kind_filter_hides_are_counted() {
        let traces = [
            trk(Some("walking"), "2024-11-25T10:00:00Z", 50.6, 3.0),
            trk(Some("cycling"), "2024-11-26T10:00:00Z", 50.7, 3.0),
            trk(None, "2024-11-27T10:00:00Z", 50.8, 3.0),
        ];
        let hidden = |filter: KindFilter| {
            drop_traces(&mut Db::open_in_memory().unwrap(), &traces, &filter).hidden_by_filter
        };
        assert_eq!(hidden(KindFilter::All), 0);
        assert_eq!(hidden(KindFilter::NotSet), 2);
        assert_eq!(hidden(KindFilter::Kind("cycling".into())), 2);
    }
}
