mod common;

use common::{gpx_with, import_into, trk};
use hikestat_app_lib::domain::storage::Db;

#[test]
fn the_points_come_back_by_segment_in_order() {
    let mut db = Db::open_in_memory().unwrap();
    let two_segments = r#"<trk><name>Gap</name>
        <trkseg>
          <trkpt lat="45.0" lon="6.0"><ele>1000</ele><time>2024-11-25T11:00:00Z</time></trkpt>
          <trkpt lat="45.001" lon="6.0"><time>2024-11-25T11:01:00Z</time></trkpt>
        </trkseg>
        <trkseg>
          <trkpt lat="45.01" lon="6.0"><ele>1100</ele><time>2024-11-25T11:20:00Z</time></trkpt>
        </trkseg>
      </trk>"#;
    import_into(
        &mut db,
        &gpx_with(&[trk("Lille", 50.0), two_segments.to_string()]),
        None,
    );
    let id = db.list_hikes().unwrap()[1].id;

    let segments = db.track(id).unwrap().expect("the hike exists");
    let summary: Vec<Vec<_>> = segments
        .iter()
        .map(|seg| seg.iter().map(|p| (p.lat, p.ele, p.time)).collect())
        .collect();
    assert_eq!(
        summary,
        vec![
            vec![
                (45.0, Some(1000.0), Some(1_732_532_400)),
                (45.001, None, Some(1_732_532_460))
            ],
            vec![(45.01, Some(1100.0), Some(1_732_533_600))],
        ]
    );
}

#[test]
fn a_hike_without_points_has_an_empty_track() {
    let db = Db::open_in_memory().unwrap();
    db.connection()
        .execute("INSERT INTO hikes (id, distance_m) VALUES (7, 0)", [])
        .unwrap();
    assert_eq!(db.track(7).unwrap(), Some(vec![]));
}

#[test]
fn an_unknown_id_returns_nothing() {
    let mut db = Db::open_in_memory().unwrap();
    assert_eq!(db.track(1).unwrap(), None, "empty database");

    import_into(&mut db, &gpx_with(&[trk("Lille", 50.0)]), None);
    assert_eq!(db.track(2).unwrap(), None);
}
