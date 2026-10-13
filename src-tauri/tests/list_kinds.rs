mod common;

use common::{gpx_with, import_into, trk};
use hikestat_app_lib::domain::storage::Db;

fn insert_kinds(db: &Db, kinds: &[Option<&str>]) {
    for kind in kinds {
        db.connection()
            .execute(
                "INSERT INTO hikes (kind, distance_m) VALUES (?1, 0)",
                [kind],
            )
            .unwrap();
    }
}

#[test]
fn an_empty_database_has_no_kind() {
    assert_eq!(
        Db::open_in_memory().unwrap().kinds().unwrap(),
        Vec::<String>::new()
    );
}

#[test]
fn hikes_without_a_kind_are_not_listed() {
    let db = Db::open_in_memory().unwrap();
    insert_kinds(&db, &[None, Some("walking"), None]);
    assert_eq!(db.kinds().unwrap(), vec!["walking"]);
}

#[test]
fn only_hikes_without_a_kind_list_nothing() {
    let db = Db::open_in_memory().unwrap();
    insert_kinds(&db, &[None, None]);
    assert_eq!(db.kinds().unwrap(), Vec::<String>::new());
}

#[test]
fn a_kind_shared_by_several_hikes_is_listed_once() {
    let db = Db::open_in_memory().unwrap();
    insert_kinds(&db, &[Some("walking"), Some("cycling"), Some("walking")]);
    assert_eq!(db.kinds().unwrap(), vec!["cycling", "walking"]);
}

#[test]
fn kinds_are_kept_as_stored_and_sorted_ignoring_case() {
    let db = Db::open_in_memory().unwrap();
    insert_kinds(
        &db,
        &[
            Some("walking"),
            Some("Running"),
            Some("Walking"),
            Some("cycling"),
            Some("Ski touring"),
        ],
    );
    assert_eq!(
        db.kinds().unwrap(),
        vec!["cycling", "Running", "Ski touring", "Walking", "walking"]
    );
}

#[test]
fn imported_kinds_are_listed() {
    let mut db = Db::open_in_memory().unwrap();
    import_into(&mut db, &gpx_with(&[trk("Lille", 50.0)]), Some("walking"));
    import_into(&mut db, &gpx_with(&[trk("Arras", 50.3)]), None);
    assert_eq!(db.kinds().unwrap(), vec!["walking"]);
}
