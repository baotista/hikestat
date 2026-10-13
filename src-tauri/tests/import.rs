use hikestat_app_lib::domain::{
    import::{ImportOptions, ImportSummary, import_file, import_path},
    stats::STATS_VERSION,
    storage::Db,
};
use std::path::PathBuf;

const GPX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx xmlns="http://www.topografix.com/GPX/1.1" version="1.1" creator="test">
  <metadata><time>2024-11-24T09:09:37Z</time></metadata>
  <trk>
    <name>lundi Lunch</name>
    <trkseg>
      <trkpt lat="50.000" lon="3.0"><ele>30.0</ele><time>2024-11-25T11:00:00Z</time></trkpt>
      <trkpt lat="50.001" lon="3.0"><time>2024-11-25T11:01:00Z</time></trkpt>
      <trkpt lat="50.002" lon="3.0"><ele>31.0</ele><time>2024-11-25T11:02:00Z</time></trkpt>
    </trkseg>
  </trk>
</gpx>"#;

fn write_gpx(dir: &tempfile::TempDir) -> PathBuf {
    let p = dir.path().join("a.gpx");
    std::fs::write(&p, GPX).unwrap();
    p
}

#[test]
fn imports_hike_with_points_and_stats() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Db::open_in_memory().unwrap();
    let summary = import_file(&mut db, &write_gpx(&dir), &ImportOptions::default());
    assert_eq!((summary.imported, summary.already_present), (1, 0));

    let c = db.connection();
    let (name, started_at, dist, dur): (String, i64, f64, i64) = c
        .query_row(
            "SELECT name, started_at, distance_m, duration_total_s FROM hikes",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(name, "lundi Lunch");
    // First point time, not <metadata><time>.
    assert_eq!(started_at, 1_732_532_400);
    assert!((dist - 222.4).abs() < 1.0, "got {dist}");
    assert_eq!(dur, 120);

    // 111 m per minute is 1.85 m/s: always moving. Elevation 30 -> 31 is below the 3 m threshold.
    let (asc, desc, min, max, moving, speed, version): (f64, f64, f64, f64, i64, f64, i64) = c
        .query_row(
            "SELECT ascent_m, descent_m, min_ele_m, max_ele_m, duration_moving_s, avg_speed_mps,
                    stats_version FROM hikes",
            [],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )
        .unwrap();
    assert_eq!((asc, desc, min, max, moving), (0.0, 0.0, 30.0, 31.0, 120));
    assert!((speed - 1.853).abs() < 0.01, "got {speed}");
    assert_eq!(version, STATS_VERSION);

    let n: i64 = c
        .query_row("SELECT COUNT(*) FROM track_points", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 3);
    let null_ele: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM track_points WHERE ele IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(null_ele, 1);
}

fn trk(name: &str, lat0: f64) -> String {
    let pts: String = (0..3)
        .map(|i| {
            format!(
                r#"<trkpt lat="{:.4}" lon="3.0"><ele>30.0</ele><time>2024-11-25T11:0{i}:00Z</time></trkpt>"#,
                lat0 + 0.001 * i as f64
            )
        })
        .collect();
    format!("<trk><name>{name}</name><trkseg>{pts}</trkseg></trk>")
}

fn gpx_with(trks: &[String]) -> String {
    format!(
        r#"<?xml version="1.0"?><gpx xmlns="http://www.topografix.com/GPX/1.1" version="1.1" creator="t">{}</gpx>"#,
        trks.concat()
    )
}

fn import_str(db: &mut Db, dir: &tempfile::TempDir, gpx: &str) -> (usize, usize) {
    let p = dir.path().join("f.gpx");
    std::fs::write(&p, gpx).unwrap();
    let s = import_file(db, &p, &ImportOptions::default());
    (s.imported, s.already_present)
}

fn hike_count(db: &Db) -> i64 {
    db.connection()
        .query_row("SELECT COUNT(*) FROM hikes", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn each_trk_becomes_a_hike() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Db::open_in_memory().unwrap();
    let gpx = gpx_with(&[trk("a", 50.0), trk("b", 51.0), trk("c", 52.0)]);
    assert_eq!(import_str(&mut db, &dir, &gpx), (3, 0));
    assert_eq!(hike_count(&db), 3);
}

#[test]
fn reimporting_the_same_file_adds_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Db::open_in_memory().unwrap();
    let gpx = gpx_with(&[trk("a", 50.0), trk("b", 51.0)]);
    import_str(&mut db, &dir, &gpx);
    assert_eq!(import_str(&mut db, &dir, &gpx), (0, 2));
    assert_eq!(hike_count(&db), 2);
    let points: i64 = db
        .connection()
        .query_row("SELECT COUNT(*) FROM track_points", [], |r| r.get(0))
        .unwrap();
    assert_eq!(points, 6);
}

#[test]
fn a_grown_export_only_adds_the_new_trace() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Db::open_in_memory().unwrap();
    import_str(&mut db, &dir, &gpx_with(&[trk("a", 50.0), trk("b", 51.0)]));
    let grown = gpx_with(&[trk("a", 50.0), trk("b", 51.0), trk("c", 52.0)]);
    assert_eq!(import_str(&mut db, &dir, &grown), (1, 2));
    assert_eq!(hike_count(&db), 3);
}

#[test]
fn a_renamed_trace_is_not_duplicated() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Db::open_in_memory().unwrap();
    import_str(&mut db, &dir, &gpx_with(&[trk("before", 50.0)]));
    assert_eq!(
        import_str(&mut db, &dir, &gpx_with(&[trk("after", 50.0)])),
        (0, 1)
    );
    let name: String = db
        .connection()
        .query_row("SELECT name FROM hikes", [], |r| r.get(0))
        .unwrap();
    assert_eq!(name, "before");
}

#[test]
fn identical_traces_in_one_file_import_once() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Db::open_in_memory().unwrap();
    let gpx = gpx_with(&[trk("a", 50.0), trk("a copy", 50.0)]);
    assert_eq!(import_str(&mut db, &dir, &gpx), (1, 1));
}

fn untimed_trk(name: &str) -> String {
    format!(
        r#"<trk><name>{name}</name><trkseg><trkpt lat="50.0" lon="3.0"><ele>30.0</ele></trkpt><trkpt lat="50.001" lon="3.0"/></trkseg></trk>"#
    )
}

fn import_text(
    db: &mut Db,
    dir: &tempfile::TempDir,
    text: &str,
) -> hikestat_app_lib::domain::import::ImportSummary {
    let p = dir.path().join("f.gpx");
    std::fs::write(&p, text).unwrap();
    import_file(db, &p, &ImportOptions::default())
}

fn point_count(db: &Db) -> i64 {
    db.connection()
        .query_row("SELECT COUNT(*) FROM track_points", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn trace_without_timestamps_is_rejected_and_the_others_go_on() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Db::open_in_memory().unwrap();
    let gpx = gpx_with(&[trk("a", 50.0), untimed_trk("bad"), trk("c", 52.0)]);
    let s = import_text(&mut db, &dir, &gpx);
    assert_eq!((s.imported, s.already_present), (2, 0));
    assert_eq!(s.rejected.len(), 1);
    assert_eq!(s.rejected[0].trace_number, Some(2));
    assert!(s.rejected[0].reason.contains("timestamp"));
    assert_eq!(hike_count(&db), 2);
    assert_eq!(
        point_count(&db),
        6,
        "the rejected trace left no points behind"
    );
}

#[test]
fn invalid_xml_rejects_the_whole_file() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Db::open_in_memory().unwrap();
    let s = import_text(&mut db, &dir, "this is not xml");
    assert_eq!(s.imported, 0);
    assert_eq!(s.rejected.len(), 1);
    assert_eq!(s.rejected[0].trace_number, None);
    assert_eq!(hike_count(&db), 0);
}

#[test]
fn missing_file_is_rejected() {
    let mut db = Db::open_in_memory().unwrap();
    let s = import_file(
        &mut db,
        std::path::Path::new("/nonexistent/x.gpx"),
        &ImportOptions::default(),
    );
    assert_eq!(s.rejected.len(), 1);
    assert_eq!(s.rejected[0].trace_number, None);
}

#[test]
fn route_only_file_imports_nothing_and_warns() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Db::open_in_memory().unwrap();
    let gpx = r#"<?xml version="1.0"?><gpx xmlns="http://www.topografix.com/GPX/1.1" version="1.1" creator="t"><rte><rtept lat="50.0" lon="3.0"/><rtept lat="50.1" lon="3.1"/></rte></gpx>"#;
    let s = import_text(&mut db, &dir, gpx);
    assert_eq!(s.imported, 0);
    assert!(s.rejected.is_empty());
    assert!(s.warnings.iter().any(|w| w.contains("route")));
    assert!(s.warnings.iter().any(|w| w.contains("no track")));
}

#[test]
fn routes_and_waypoints_next_to_a_track_are_ignored_with_a_warning() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Db::open_in_memory().unwrap();
    let gpx = format!(
        r#"<?xml version="1.0"?><gpx xmlns="http://www.topografix.com/GPX/1.1" version="1.1" creator="t"><wpt lat="50.0" lon="3.0"/>{}<rte><rtept lat="50.0" lon="3.0"/></rte></gpx>"#,
        trk("a", 50.0)
    );
    let s = import_text(&mut db, &dir, &gpx);
    assert_eq!(s.imported, 1);
    assert!(s.rejected.is_empty());
    assert_eq!(s.warnings.len(), 2);
}

fn write_in(dir: &std::path::Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

#[test]
fn a_directory_imports_every_gpx_file() {
    let dir = tempfile::tempdir().unwrap();
    write_in(dir.path(), "a.gpx", &gpx_with(&[trk("a", 50.0)]));
    write_in(
        dir.path(),
        "b.GPX",
        &gpx_with(&[trk("b", 51.0), trk("c", 52.0)]),
    );
    write_in(dir.path(), "notes.txt", "not a gpx file");
    let mut db = Db::open_in_memory().unwrap();
    let s = import_path(&mut db, dir.path(), &ImportOptions::default());
    assert_eq!((s.imported, s.already_present), (3, 0));
    assert!(s.rejected.is_empty());
    assert_eq!(hike_count(&db), 3);
}

#[test]
fn a_broken_file_does_not_stop_the_others_and_is_named_in_the_summary() {
    let dir = tempfile::tempdir().unwrap();
    write_in(dir.path(), "a.gpx", &gpx_with(&[trk("a", 50.0)]));
    write_in(dir.path(), "b.gpx", "not xml");
    write_in(dir.path(), "c.gpx", &gpx_with(&[trk("c", 52.0)]));
    let mut db = Db::open_in_memory().unwrap();
    let s = import_path(&mut db, dir.path(), &ImportOptions::default());
    assert_eq!(s.imported, 2);
    assert_eq!(s.rejected.len(), 1);
    assert!(s.rejected[0].file.ends_with("b.gpx"));
}

#[test]
fn rerunning_on_the_same_directory_creates_nothing() {
    let dir = tempfile::tempdir().unwrap();
    write_in(dir.path(), "a.gpx", &gpx_with(&[trk("a", 50.0)]));
    write_in(dir.path(), "b.gpx", &gpx_with(&[trk("b", 51.0)]));
    let mut db = Db::open_in_memory().unwrap();
    import_path(&mut db, dir.path(), &ImportOptions::default());
    let s = import_path(&mut db, dir.path(), &ImportOptions::default());
    assert_eq!((s.imported, s.already_present), (0, 2));
    assert_eq!(hike_count(&db), 2);
}

#[test]
fn subdirectories_are_not_entered() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    write_in(
        &dir.path().join("sub"),
        "a.gpx",
        &gpx_with(&[trk("a", 50.0)]),
    );
    let mut db = Db::open_in_memory().unwrap();
    let s = import_path(&mut db, dir.path(), &ImportOptions::default());
    assert_eq!(s.imported, 0);
    assert!(s.warnings.iter().any(|w| w.contains("no .gpx file")));
}

fn typed_trk(name: &str, lat0: f64, kind: &str) -> String {
    trk(name, lat0).replace("</trk>", &format!("<type>{kind}</type></trk>"))
}

fn kinds(db: &Db) -> Vec<Option<String>> {
    let mut stmt = db
        .connection()
        .prepare("SELECT kind FROM hikes ORDER BY id")
        .unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn with_kind(kind: &str) -> ImportOptions {
    ImportOptions {
        kind: Some(kind.into()),
        ..ImportOptions::default()
    }
}

fn import_gpx(
    db: &mut Db,
    gpx: &str,
    opts: &ImportOptions,
) -> hikestat_app_lib::domain::import::ImportSummary {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("f.gpx");
    std::fs::write(&p, gpx).unwrap();
    import_file(db, &p, opts)
}

#[test]
fn kind_flag_applies_to_every_trace() {
    let mut db = Db::open_in_memory().unwrap();
    let gpx = gpx_with(&[trk("a", 50.0), trk("b", 51.0)]);
    import_gpx(&mut db, &gpx, &with_kind("walking"));
    assert_eq!(
        kinds(&db),
        vec![Some("walking".into()), Some("walking".into())]
    );
}

#[test]
fn file_type_is_used_without_the_flag() {
    let mut db = Db::open_in_memory().unwrap();
    let gpx = gpx_with(&[typed_trk("a", 50.0, "hiking"), trk("b", 51.0)]);
    import_gpx(&mut db, &gpx, &ImportOptions::default());
    assert_eq!(kinds(&db), vec![Some("hiking".into()), None]);
}

#[test]
fn flag_wins_over_the_file_type() {
    let mut db = Db::open_in_memory().unwrap();
    let gpx = gpx_with(&[typed_trk("a", 50.0, "hiking")]);
    import_gpx(&mut db, &gpx, &with_kind("walking"));
    assert_eq!(kinds(&db), vec![Some("walking".into())]);
}

#[test]
fn kind_is_never_guessed_from_the_name() {
    let mut db = Db::open_in_memory().unwrap();
    import_gpx(
        &mut db,
        &gpx_with(&[trk("lundi Lunch À pied", 50.0)]),
        &ImportOptions::default(),
    );
    assert_eq!(kinds(&db), vec![None]);
}

#[test]
fn blank_kind_counts_as_absent() {
    let mut db = Db::open_in_memory().unwrap();
    let gpx = gpx_with(&[typed_trk("a", 50.0, "  hiking ")]);
    import_gpx(&mut db, &gpx, &with_kind("   "));
    assert_eq!(kinds(&db), vec![Some("hiking".into())]);
}

#[test]
fn reimporting_with_another_kind_neither_duplicates_nor_changes_it() {
    let mut db = Db::open_in_memory().unwrap();
    let gpx = gpx_with(&[trk("a", 50.0)]);
    import_gpx(&mut db, &gpx, &with_kind("walking"));
    let s = import_gpx(&mut db, &gpx, &with_kind("cycling"));
    assert_eq!((s.imported, s.already_present), (0, 1));
    assert_eq!(kinds(&db), vec![Some("walking".into())]);
}

fn start_tzs(db: &Db) -> Vec<Option<String>> {
    let mut stmt = db
        .connection()
        .prepare("SELECT start_tz FROM hikes ORDER BY id")
        .unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn the_start_time_zone_comes_from_the_coordinates() {
    let mut db = Db::open_in_memory().unwrap();
    let tokyo = trk("tokyo", 35.68).replace(r#"lon="3.0""#, r#"lon="139.65""#);
    let gpx = gpx_with(&[trk("lille", 50.0), tokyo]);
    import_gpx(&mut db, &gpx, &ImportOptions::default());
    assert_eq!(
        start_tzs(&db),
        vec![Some("Europe/Paris".into()), Some("Asia/Tokyo".into())]
    );
}

#[test]
fn the_start_time_zone_is_the_one_of_the_first_timestamped_point() {
    let mut db = Db::open_in_memory().unwrap();
    // An untimed point in Tokyo, then the timed points near Lille.
    let untimed_tokyo = r#"<trkseg><trkpt lat="35.68" lon="139.65"/></trkseg>"#;
    let gpx = gpx_with(&[trk("a", 50.0).replace("<trkseg>", &format!("{untimed_tokyo}<trkseg>"))]);
    import_gpx(&mut db, &gpx, &ImportOptions::default());
    assert_eq!(start_tzs(&db), vec![Some("Europe/Paris".into())]);
}

fn dry() -> ImportOptions {
    ImportOptions {
        dry_run: true,
        ..ImportOptions::default()
    }
}

fn tables_are_empty(db: &Db) -> bool {
    hike_count(db) == 0 && point_count(db) == 0
}

#[test]
fn dry_run_reports_what_would_be_imported_and_writes_nothing() {
    let mut db = Db::open_in_memory().unwrap();
    let gpx = gpx_with(&[trk("a", 50.0), trk("b", 51.0)]);
    let s = import_gpx(&mut db, &gpx, &dry());
    assert_eq!((s.imported, s.already_present), (2, 0));
    assert!(tables_are_empty(&db));
}

#[test]
fn dry_run_matches_the_real_import_summary() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("f.gpx");
    std::fs::write(
        &f,
        gpx_with(&[trk("a", 50.0), untimed_trk("bad"), trk("a copy", 50.0)]),
    )
    .unwrap();
    let real = import_file(
        &mut Db::open_in_memory().unwrap(),
        &f,
        &ImportOptions::default(),
    );
    let dry = import_file(&mut Db::open_in_memory().unwrap(), &f, &dry());
    // The same summary, except the ids of the created hikes: a dry run keeps none.
    assert_eq!(dry.imported_ids, Vec::<i64>::new());
    assert_eq!(
        ImportSummary {
            imported_ids: vec![],
            ..real
        },
        dry
    );
    assert_eq!(
        (dry.imported, dry.already_present, dry.rejected.len()),
        (1, 1, 1)
    );
}

#[test]
fn dry_run_sees_what_is_already_in_the_database() {
    let mut db = Db::open_in_memory().unwrap();
    import_gpx(
        &mut db,
        &gpx_with(&[trk("a", 50.0)]),
        &ImportOptions::default(),
    );
    let gpx = gpx_with(&[trk("a", 50.0), trk("b", 51.0)]);
    let s = import_gpx(&mut db, &gpx, &dry());
    assert_eq!((s.imported, s.already_present), (1, 1));
    assert_eq!(hike_count(&db), 1, "only the earlier real import is there");
}

#[test]
fn dry_run_over_a_directory_spots_duplicates_across_files() {
    let dir = tempfile::tempdir().unwrap();
    write_in(dir.path(), "a.gpx", &gpx_with(&[trk("a", 50.0)]));
    write_in(dir.path(), "b.gpx", &gpx_with(&[trk("a again", 50.0)]));
    let mut db = Db::open_in_memory().unwrap();
    let s = import_path(&mut db, dir.path(), &dry());
    assert_eq!((s.imported, s.already_present), (1, 1));
    assert!(tables_are_empty(&db));
}

#[test]
fn a_real_import_still_works_after_a_dry_run() {
    let mut db = Db::open_in_memory().unwrap();
    let gpx = gpx_with(&[trk("a", 50.0)]);
    import_gpx(&mut db, &gpx, &dry());
    let s = import_gpx(&mut db, &gpx, &ImportOptions::default());
    assert_eq!((s.imported, s.already_present), (1, 0));
    assert_eq!(hike_count(&db), 1);
}
