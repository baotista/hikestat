//! Importing what is dropped on the window, through the library (ADR 0004), and what to tell the
//! user about it.

use std::path::PathBuf;

use hikestat::import::{self, ImportOptions, ImportSummary};
use hikestat::storage::Db;
use serde::Serialize;
use specta::Type;

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
}

fn report(summary: ImportSummary) -> ImportReport {
    ImportReport {
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

/// Imports the dropped `paths` into `db`, with the CLI's rules.
pub fn import(db: &mut Db, paths: &[PathBuf]) -> ImportReport {
    report(import::import_paths(db, paths, &ImportOptions::default()))
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

        let first = import(&mut db, &[gpx.clone(), photo.clone()]);
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

        let again = import(&mut db, &[gpx]);
        assert_eq!((again.imported, again.already_present), (0, 1));
    }

    #[test]
    fn nothing_dropped_reports_nothing() {
        let mut db = Db::open_in_memory().unwrap();
        let r = import(&mut db, &[]);
        assert_eq!(
            (r.imported, r.already_present, r.rejections, r.warnings),
            (0, 0, vec![], vec![])
        );
    }
}
