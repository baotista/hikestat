mod common;

use common::{gpx_with, import_into, trk};
use hikestat_app_lib::domain::stats::STATS_VERSION;
use hikestat_app_lib::domain::storage::{Db, Hike};

/// 2024-11-25T11:00:00Z, the first point of `common::trk`.
const TRK_START: i64 = 1_732_532_400;

fn insert(db: &Db, id: i64, started_at: Option<i64>) {
    db.connection()
        .execute(
            "INSERT INTO hikes (id, name, started_at, distance_m) VALUES (?1, ?2, ?3, 0)",
            rusqlite::params![id, format!("h{id}"), started_at],
        )
        .unwrap();
}

fn ids(db: &Db) -> Vec<i64> {
    db.list_hikes().unwrap().iter().map(|h| h.id).collect()
}

#[test]
fn an_empty_database_lists_no_hike() {
    assert_eq!(Db::open_in_memory().unwrap().list_hikes().unwrap(), vec![]);
}

#[test]
fn an_imported_hike_is_listed_with_every_column_and_no_points() {
    let mut db = Db::open_in_memory().unwrap();
    import_into(&mut db, &gpx_with(&[trk("Lille", 50.0)]), Some("walking"));

    let hikes = db.list_hikes().unwrap();
    assert_eq!(hikes.len(), 1);
    let h = &hikes[0];
    let distance_m = h.distance_m;
    assert!(distance_m > 200.0 && distance_m < 250.0, "got {distance_m}");
    assert_eq!(
        *h,
        Hike {
            id: 1,
            name: Some("Lille".into()),
            kind: Some("walking".into()),
            started_at: Some(TRK_START),
            start_tz: Some("Europe/Paris".into()),
            distance_m,
            ascent_m: Some(10.0),
            descent_m: Some(0.0),
            min_ele_m: Some(30.0),
            max_ele_m: Some(40.0),
            duration_total_s: Some(120),
            duration_moving_s: Some(120),
            avg_speed_mps: Some(distance_m / 120.0),
            stats_version: Some(STATS_VERSION),
            title: None,
            note: None,
        }
    );
}

#[test]
fn several_hikes_are_ordered_by_start_then_id() {
    let db = Db::open_in_memory().unwrap();
    insert(&db, 1, Some(300));
    insert(&db, 2, Some(100));
    insert(&db, 3, Some(200));
    insert(&db, 4, Some(100));
    assert_eq!(ids(&db), vec![2, 4, 3, 1]);
}

#[test]
fn a_hike_without_a_start_comes_first() {
    let db = Db::open_in_memory().unwrap();
    insert(&db, 1, Some(100));
    insert(&db, 2, None);
    assert_eq!(ids(&db), vec![2, 1]);
}

#[test]
fn null_columns_are_listed_as_none() {
    let db = Db::open_in_memory().unwrap();
    db.connection()
        .execute("INSERT INTO hikes (id, distance_m) VALUES (7, 1234.5)", [])
        .unwrap();
    assert_eq!(
        db.list_hikes().unwrap(),
        vec![Hike {
            id: 7,
            name: None,
            kind: None,
            started_at: None,
            start_tz: None,
            distance_m: 1234.5,
            ascent_m: None,
            descent_m: None,
            min_ele_m: None,
            max_ele_m: None,
            duration_total_s: None,
            duration_moving_s: None,
            avg_speed_mps: None,
            stats_version: None,
            title: None,
            note: None,
        }]
    );
}

#[test]
fn the_start_time_zone_of_each_hike_is_listed() {
    let mut db = Db::open_in_memory().unwrap();
    import_into(&mut db, &gpx_with(&[trk("Lille", 50.0)]), None);
    db.connection()
        .execute_batch(
            "INSERT INTO hikes (id, started_at, distance_m, start_tz)
             VALUES (2, 1, 0, 'Asia/Tokyo')",
        )
        .unwrap();
    let zones: Vec<_> = db
        .list_hikes()
        .unwrap()
        .into_iter()
        .map(|h| h.start_tz)
        .collect();
    assert_eq!(
        zones,
        vec![Some("Asia/Tokyo".into()), Some("Europe/Paris".into())]
    );
}
