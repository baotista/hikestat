mod common;

use common::{gpx_with, import_into, trk};
use hikestat_app_lib::domain::storage::Db;

#[test]
fn an_existing_hike_is_returned_as_listed() {
    let mut db = Db::open_in_memory().unwrap();
    import_into(
        &mut db,
        &gpx_with(&[trk("Lille", 50.0), trk("Arras", 50.3)]),
        Some("walking"),
    );
    let listed = db.list_hikes().unwrap();
    assert_eq!(listed.len(), 2);

    for h in &listed {
        assert_eq!(db.hike(h.id).unwrap().as_ref(), Some(h));
    }
}

#[test]
fn a_hike_with_null_columns_is_returned() {
    let db = Db::open_in_memory().unwrap();
    db.connection()
        .execute("INSERT INTO hikes (id, distance_m) VALUES (7, 1234.5)", [])
        .unwrap();
    let h = db.hike(7).unwrap().expect("hike 7 exists");
    assert_eq!(
        (h.id, h.name, h.distance_m, h.ascent_m),
        (7, None, 1234.5, None)
    );
}

#[test]
fn an_unknown_id_returns_nothing() {
    let mut db = Db::open_in_memory().unwrap();
    assert_eq!(db.hike(1).unwrap(), None, "empty database");

    import_into(&mut db, &gpx_with(&[trk("Lille", 50.0)]), None);
    assert_eq!(db.hike(2).unwrap(), None);
    assert_eq!(db.hike(-1).unwrap(), None);
}
