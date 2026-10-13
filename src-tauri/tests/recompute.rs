mod common;

use common::{gpx_with, import_into, trk};
use hikestat_app_lib::domain::stats::STATS_VERSION;
use hikestat_app_lib::domain::storage::Db;

const STATS_COLUMNS: &str = "distance_m, ascent_m, descent_m, min_ele_m, max_ele_m, \
     duration_total_s, duration_moving_s, avg_speed_mps, stats_version";
const UNTOUCHED_COLUMNS: &str = "name, kind, started_at, trace_hash";

fn dump(db: &Db, table_query: &str) -> Vec<String> {
    let mut stmt = db.connection().prepare(table_query).unwrap();
    let n = stmt.column_count();
    stmt.query_map([], |r| {
        Ok((0..n)
            .map(|i| format!("{:?}", r.get_ref(i).unwrap()))
            .collect::<Vec<_>>()
            .join("|"))
    })
    .unwrap()
    .map(Result::unwrap)
    .collect()
}

fn stats_rows(db: &Db) -> Vec<String> {
    dump(
        db,
        &format!("SELECT {STATS_COLUMNS} FROM hikes ORDER BY id"),
    )
}

fn untouched_rows(db: &Db) -> Vec<String> {
    dump(
        db,
        &format!("SELECT {UNTOUCHED_COLUMNS} FROM hikes ORDER BY id"),
    )
}

fn point_rows(db: &Db) -> Vec<String> {
    dump(
        db,
        "SELECT hike_id, segment_idx, idx, lat, lon, ele, time FROM track_points ORDER BY id",
    )
}

fn seeded() -> Db {
    let mut db = Db::open_in_memory().unwrap();
    let gpx = gpx_with(&[trk("a", 50.0), trk("b", 51.0), trk("c", 52.0)]);
    import_into(&mut db, &gpx, Some("walking"));
    db
}

#[test]
fn an_up_to_date_database_is_left_alone() {
    let mut db = seeded();
    let before = stats_rows(&db);
    assert_eq!(db.recompute_stats().unwrap(), 0);
    assert_eq!(stats_rows(&db), before);
}

#[test]
fn stale_stats_and_version_are_recomputed_from_the_points() {
    let mut db = seeded();
    let (stats, untouched, points) = (stats_rows(&db), untouched_rows(&db), point_rows(&db));
    db.connection()
        .execute_batch(
            "UPDATE hikes SET ascent_m = 9999, distance_m = 1, avg_speed_mps = NULL,
                              stats_version = 1 WHERE id IN (1, 3)",
        )
        .unwrap();
    assert_ne!(stats_rows(&db), stats);

    assert_eq!(db.recompute_stats().unwrap(), 2);

    assert_eq!(
        stats_rows(&db),
        stats,
        "stats are back to what the points give"
    );
    assert_eq!(
        untouched_rows(&db),
        untouched,
        "name, kind, date and hash are kept"
    );
    assert_eq!(point_rows(&db), points, "points are kept");
}

#[test]
fn rows_without_a_stats_version_are_brought_up_to_date() {
    let mut db = seeded();
    let stats = stats_rows(&db);
    db.connection()
        .execute_batch(
            "UPDATE hikes SET ascent_m = NULL, descent_m = NULL, min_ele_m = NULL,
                              max_ele_m = NULL, duration_moving_s = NULL,
                              avg_speed_mps = NULL, stats_version = NULL WHERE id = 2",
        )
        .unwrap();
    assert_eq!(db.recompute_stats().unwrap(), 1);
    assert_eq!(stats_rows(&db), stats);
    let version: i64 = db
        .connection()
        .query_row("SELECT stats_version FROM hikes WHERE id = 2", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(version, STATS_VERSION);
}

fn start_tz_rows(db: &Db) -> Vec<String> {
    dump(db, "SELECT start_tz FROM hikes ORDER BY id")
}

#[test]
fn a_missing_or_outdated_start_time_zone_is_backfilled() {
    let mut db = seeded();
    let (zones, stats, untouched, points) = (
        start_tz_rows(&db),
        stats_rows(&db),
        untouched_rows(&db),
        point_rows(&db),
    );
    db.connection()
        .execute_batch(
            "UPDATE hikes SET start_tz = NULL WHERE id = 1;
             UPDATE hikes SET start_tz = 'Asia/Tokyo' WHERE id = 3;",
        )
        .unwrap();

    assert_eq!(db.recompute_stats().unwrap(), 2);
    assert_eq!(db.recompute_stats().unwrap(), 0);

    assert!(
        !zones.contains(&"Null".to_string()),
        "import set every zone"
    );
    assert_eq!(
        start_tz_rows(&db),
        zones,
        "zones are back to what import gave"
    );
    assert_eq!(stats_rows(&db), stats);
    assert_eq!(
        untouched_rows(&db),
        untouched,
        "name, kind, date and hash are kept"
    );
    assert_eq!(point_rows(&db), points, "points are kept");
}

#[test]
fn a_failure_part_way_leaves_every_row_as_it_was() {
    let mut db = seeded();
    db.connection()
        .execute_batch(
            "UPDATE hikes SET distance_m = 1, stats_version = 1;
             CREATE TRIGGER fail_on_second BEFORE UPDATE ON hikes WHEN NEW.id = 2
             BEGIN SELECT RAISE(ABORT, 'boom'); END;",
        )
        .unwrap();
    let before = stats_rows(&db);

    assert!(db.recompute_stats().is_err());

    assert_eq!(
        stats_rows(&db),
        before,
        "the first row must not stay updated"
    );
}

#[test]
fn an_empty_database_has_nothing_to_recompute() {
    assert_eq!(Db::open_in_memory().unwrap().recompute_stats().unwrap(), 0);
}
