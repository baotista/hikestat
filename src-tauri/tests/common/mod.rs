#![allow(dead_code)]

use hikestat_app_lib::domain::import::{ImportOptions, import_file};
use hikestat_app_lib::domain::storage::Db;

/// One `<trk>` of three timed points a minute apart, heading north from `lat0`.
pub fn trk(name: &str, lat0: f64) -> String {
    let pts: String = (0..3)
        .map(|i| {
            format!(
                r#"<trkpt lat="{:.4}" lon="3.0"><ele>{}.0</ele><time>2024-11-25T11:0{i}:00Z</time></trkpt>"#,
                lat0 + 0.001 * i as f64,
                30 + 5 * i
            )
        })
        .collect();
    format!("<trk><name>{name}</name><trkseg>{pts}</trkseg></trk>")
}

pub fn gpx_with(trks: &[String]) -> String {
    format!(
        r#"<?xml version="1.0"?><gpx xmlns="http://www.topografix.com/GPX/1.1" version="1.1" creator="t">{}</gpx>"#,
        trks.concat()
    )
}

pub fn import_into(db: &mut Db, gpx: &str, kind: Option<&str>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("f.gpx");
    std::fs::write(&path, gpx).unwrap();
    let opts = ImportOptions {
        kind: kind.map(str::to_string),
        ..ImportOptions::default()
    };
    let s = import_file(db, &path, &opts);
    assert!(s.rejected.is_empty(), "{:?}", s.rejected);
}
