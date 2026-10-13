//! Regression test on a reduced, anonymised slice of a real Gaia GPS export: 3 traces (one with a
//! glitch elevation, one short climb, one at altitude) and a planned route that must be ignored.
//! Expected values are a snapshot of the current algorithm (`STATS_VERSION` 2): if they change on
//! purpose, bump the version and update them.

use std::path::PathBuf;

use hikestat_app_lib::domain::import::{ImportOptions, ImportSummary, import_file};
use hikestat_app_lib::domain::storage::Db;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/export_sample.gpx")
}

fn import(db: &mut Db) -> ImportSummary {
    import_file(db, &fixture(), &ImportOptions::default())
}

fn count(db: &Db, sql: &str) -> i64 {
    db.connection().query_row(sql, [], |r| r.get(0)).unwrap()
}

struct Expected {
    name: &'static str,
    points: i64,
    distance_m: f64,
    ascent_m: f64,
    descent_m: f64,
    min_ele_m: f64,
    max_ele_m: f64,
    total_s: i64,
    moving_s: i64,
}

const EXPECTED: [Expected; 3] = [
    Expected {
        name: "Sortie A",
        points: 329,
        distance_m: 4431.1,
        ascent_m: 26.9,
        descent_m: 26.9,
        min_ele_m: 23.5,
        max_ele_m: 32.5,
        total_s: 2534,
        moving_s: 2255,
    },
    Expected {
        name: "Sortie B",
        points: 69,
        distance_m: 668.6,
        ascent_m: 120.8,
        descent_m: 47.3,
        min_ele_m: 47.0,
        max_ele_m: 141.0,
        total_s: 946,
        moving_s: 643,
    },
    Expected {
        name: "Sortie C",
        points: 240,
        distance_m: 3464.2,
        ascent_m: 156.6,
        descent_m: 192.6,
        min_ele_m: 828.0,
        max_ele_m: 967.0,
        total_s: 5746,
        moving_s: 1933,
    },
];

#[test]
fn imports_three_hikes_and_ignores_the_route() {
    let mut db = Db::open_in_memory().unwrap();
    let s = import(&mut db);
    assert_eq!((s.imported, s.already_present, s.rejected.len()), (3, 0, 0));
    assert!(
        s.warnings.iter().any(|w| w.contains("1 route")),
        "{:?}",
        s.warnings
    );
    assert_eq!(count(&db, "SELECT COUNT(*) FROM hikes"), 3);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM track_points"), 638);
}

#[test]
fn stats_match_the_snapshot() {
    let mut db = Db::open_in_memory().unwrap();
    import(&mut db);
    for e in EXPECTED {
        let (points, dist, up, down, min, max, total, moving): (
            i64,
            f64,
            f64,
            f64,
            f64,
            f64,
            i64,
            i64,
        ) = db
            .connection()
            .query_row(
                "SELECT (SELECT COUNT(*) FROM track_points WHERE hike_id = h.id),
                        distance_m, ascent_m, descent_m, min_ele_m, max_ele_m,
                        duration_total_s, duration_moving_s
                 FROM hikes h WHERE name = ?1",
                [e.name],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                    ))
                },
            )
            .unwrap();
        let near = |got: f64, want: f64, what: &str| {
            assert!(
                (got - want).abs() < 0.1,
                "{}: {what} was {got}, expected {want}",
                e.name
            );
        };
        assert_eq!(points, e.points, "{}: points", e.name);
        near(dist, e.distance_m, "distance");
        near(up, e.ascent_m, "ascent");
        near(down, e.descent_m, "descent");
        near(min, e.min_ele_m, "min elevation");
        near(max, e.max_ele_m, "max elevation");
        assert_eq!(
            (total, moving),
            (e.total_s, e.moving_s),
            "{}: durations",
            e.name
        );
    }
}

#[test]
fn the_glitch_elevation_is_stored_but_does_not_count() {
    let mut db = Db::open_in_memory().unwrap();
    import(&mut db);
    assert_eq!(
        count(&db, "SELECT COUNT(*) FROM track_points WHERE ele < -4000"),
        1
    );
    let (ascent, min): (f64, f64) = db
        .connection()
        .query_row(
            "SELECT ascent_m, min_ele_m FROM hikes WHERE name = 'Sortie A'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(
        ascent < 100.0,
        "a 5000 m jump leaked into the ascent: {ascent}"
    );
    assert!(min > -500.0, "the glitch leaked into the minimum: {min}");
}

#[test]
fn a_second_import_creates_nothing() {
    let mut db = Db::open_in_memory().unwrap();
    import(&mut db);
    let s = import(&mut db);
    assert_eq!((s.imported, s.already_present, s.rejected.len()), (0, 3, 0));
    assert_eq!(count(&db, "SELECT COUNT(*) FROM hikes"), 3);
    assert_eq!(count(&db, "SELECT COUNT(*) FROM track_points"), 638);
}

#[test]
fn recompute_changes_nothing() {
    let mut db = Db::open_in_memory().unwrap();
    import(&mut db);
    assert_eq!(db.recompute_stats().unwrap(), 0);
}
