mod common;

use common::{gpx_with, import_into, trk};
use hikestat_app_lib::domain::Error;
use hikestat_app_lib::domain::storage::Db;

/// One hike, imported as "walking".
fn seeded() -> (Db, i64) {
    let mut db = Db::open_in_memory().unwrap();
    import_into(&mut db, &gpx_with(&[trk("Lille", 50.0)]), Some("walking"));
    let id = db.list_hikes().unwrap()[0].id;
    (db, id)
}

fn kind(db: &Db, id: i64) -> Option<String> {
    db.hike(id).unwrap().expect("hike exists").kind
}

#[test]
fn a_kind_is_set_then_cleared() {
    let (db, id) = seeded();

    db.set_kind(id, "cycling").unwrap();
    assert_eq!(kind(&db, id).as_deref(), Some("cycling"));

    db.set_kind(id, "").unwrap();
    assert_eq!(kind(&db, id), None);
}

#[test]
fn a_kind_is_set_on_a_hike_without_one() {
    let mut db = Db::open_in_memory().unwrap();
    import_into(&mut db, &gpx_with(&[trk("Lille", 50.0)]), None);
    let id = db.list_hikes().unwrap()[0].id;

    db.set_kind(id, "walking").unwrap();
    assert_eq!(kind(&db, id).as_deref(), Some("walking"));
}

#[test]
fn an_empty_kind_is_stored_as_null() {
    let (db, id) = seeded();
    db.set_kind(id, "").unwrap();
    let raw: Option<String> = db
        .connection()
        .query_row("SELECT kind FROM hikes WHERE id = ?1", [id], |r| r.get(0))
        .unwrap();
    assert_eq!(raw, None);
}

#[test]
fn a_whitespace_only_kind_is_stored_as_null() {
    let (db, id) = seeded();
    db.set_kind(id, " \t\n ").unwrap();
    assert_eq!(kind(&db, id), None);
}

#[test]
fn a_kind_is_trimmed_and_otherwise_kept_as_typed() {
    let (db, id) = seeded();
    db.set_kind(id, "  Ski touring \n").unwrap();
    assert_eq!(kind(&db, id).as_deref(), Some("Ski touring"));
}

#[test]
fn only_the_given_hike_is_edited() {
    let mut db = Db::open_in_memory().unwrap();
    import_into(
        &mut db,
        &gpx_with(&[trk("Lille", 50.0), trk("Arras", 50.3)]),
        Some("walking"),
    );
    let ids: Vec<i64> = db.list_hikes().unwrap().iter().map(|h| h.id).collect();

    db.set_kind(ids[0], "cycling").unwrap();
    assert_eq!(kind(&db, ids[0]).as_deref(), Some("cycling"));
    assert_eq!(kind(&db, ids[1]).as_deref(), Some("walking"));
}

#[test]
fn an_unknown_id_is_an_error_and_writes_nothing() {
    let (db, id) = seeded();
    let before = db.list_hikes().unwrap();

    for unknown in [id + 1, -1] {
        let err = db.set_kind(unknown, "cycling").unwrap_err();
        assert!(
            matches!(err, Error::HikeNotFound(i) if i == unknown),
            "got {err:?}"
        );
        assert_eq!(err.to_string(), format!("no hike with id {unknown}"));
    }
    assert_eq!(db.list_hikes().unwrap(), before);
}

#[test]
fn clearing_an_unknown_id_is_an_error_too() {
    let db = Db::open_in_memory().unwrap();
    assert!(matches!(db.set_kind(1, ""), Err(Error::HikeNotFound(1))));
}

#[test]
fn a_reimport_never_overwrites_an_edited_kind() {
    let gpx = gpx_with(&[trk("Lille", 50.0)]);
    let (mut db, id) = seeded();
    db.set_kind(id, "Trail running").unwrap();

    import_into(&mut db, &gpx, Some("walking"));
    import_into(&mut db, &gpx, Some("cycling"));
    import_into(&mut db, &gpx, None);

    assert_eq!(db.list_hikes().unwrap().len(), 1);
    assert_eq!(kind(&db, id).as_deref(), Some("Trail running"));
}

#[test]
fn a_reimport_never_restores_a_cleared_kind() {
    let gpx = gpx_with(&[trk("Lille", 50.0)]);
    let (mut db, id) = seeded();
    db.set_kind(id, "").unwrap();

    import_into(&mut db, &gpx, Some("walking"));

    assert_eq!(kind(&db, id), None);
}

#[test]
fn recompute_leaves_an_edited_kind_untouched() {
    let (mut db, id) = seeded();
    db.set_kind(id, "cycling").unwrap();
    // Stale stats, so that recompute does write this row.
    db.connection()
        .execute("UPDATE hikes SET stats_version = 1 WHERE id = ?1", [id])
        .unwrap();

    assert_eq!(db.recompute_stats().unwrap(), 1);
    assert_eq!(kind(&db, id).as_deref(), Some("cycling"));
}

#[test]
fn kinds_reflect_an_edit() {
    let mut db = Db::open_in_memory().unwrap();
    import_into(
        &mut db,
        &gpx_with(&[trk("Lille", 50.0), trk("Arras", 50.3)]),
        Some("walking"),
    );
    let ids: Vec<i64> = db.list_hikes().unwrap().iter().map(|h| h.id).collect();

    db.set_kind(ids[0], "cycling").unwrap();
    assert_eq!(db.kinds().unwrap(), vec!["cycling", "walking"]);

    db.set_kind(ids[1], "").unwrap();
    assert_eq!(
        db.kinds().unwrap(),
        vec!["cycling"],
        "the last walk is gone"
    );

    db.set_kind(ids[0], "").unwrap();
    assert_eq!(db.kinds().unwrap(), Vec::<String>::new());
}
