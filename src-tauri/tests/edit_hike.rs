mod common;

use common::{gpx_with, import_into, trk};
use hikestat_app_lib::domain::Error;
use hikestat_app_lib::domain::storage::{Db, Hike};

fn seeded() -> Db {
    let mut db = Db::open_in_memory().unwrap();
    import_into(
        &mut db,
        &gpx_with(&[trk("Track 1", 50.0), trk("Track 2", 51.0)]),
        Some("walking"),
    );
    db
}

fn hike(db: &Db, id: i64) -> Hike {
    db.hike(id).unwrap().expect("hike exists")
}

#[test]
fn a_title_is_set_then_cleared_and_the_name_is_kept() {
    let db = seeded();
    db.set_title(1, Some("Tour du Mont Aiguille")).unwrap();
    let h = hike(&db, 1);
    assert_eq!(h.title.as_deref(), Some("Tour du Mont Aiguille"));
    assert_eq!(
        h.name.as_deref(),
        Some("Track 1"),
        "the raw name is never modified"
    );
    assert_eq!(hike(&db, 2).title, None, "other hikes are untouched");

    db.set_title(1, None).unwrap();
    let h = hike(&db, 1);
    assert_eq!(h.title, None);
    assert_eq!(h.name.as_deref(), Some("Track 1"));
}

#[test]
fn a_title_is_trimmed() {
    let db = seeded();
    db.set_title(1, Some("  Mont Aiguille \n")).unwrap();
    assert_eq!(hike(&db, 1).title.as_deref(), Some("Mont Aiguille"));
}

#[test]
fn an_empty_or_blank_title_is_stored_as_null() {
    let db = seeded();
    for blank in ["", "   ", "\t\n "] {
        db.set_title(1, Some("Set")).unwrap();
        db.set_title(1, Some(blank)).unwrap();
        assert_eq!(hike(&db, 1).title, None, "{blank:?}");
    }
    let stored: Option<String> = db
        .connection()
        .query_row("SELECT title FROM hikes WHERE id = 1", [], |r| r.get(0))
        .unwrap();
    assert_eq!(stored, None);
}

#[test]
fn a_note_is_set_then_cleared() {
    let db = seeded();
    db.set_note(2, Some("Snow above 1800 m.")).unwrap();
    assert_eq!(hike(&db, 2).note.as_deref(), Some("Snow above 1800 m."));
    assert_eq!(hike(&db, 1).note, None, "other hikes are untouched");

    db.set_note(2, None).unwrap();
    assert_eq!(hike(&db, 2).note, None);
}

#[test]
fn a_note_keeps_its_inner_line_breaks_and_loses_outer_whitespace() {
    let db = seeded();
    db.set_note(1, Some("\n  First line\n\nThird line  \n"))
        .unwrap();
    assert_eq!(
        hike(&db, 1).note.as_deref(),
        Some("First line\n\nThird line")
    );
}

#[test]
fn an_empty_or_blank_note_is_stored_as_null() {
    let db = seeded();
    for blank in ["", "   ", "\n\n"] {
        db.set_note(1, Some("Set")).unwrap();
        db.set_note(1, Some(blank)).unwrap();
        assert_eq!(hike(&db, 1).note, None, "{blank:?}");
    }
}

#[test]
fn editing_an_unknown_hike_is_an_error() {
    let db = seeded();
    let before = db.list_hikes().unwrap();
    for id in [3, 0, -1] {
        let err = db.set_title(id, Some("x")).unwrap_err();
        assert!(
            matches!(err, Error::HikeNotFound(i) if i == id),
            "got {err:?}"
        );
        let err = db.set_note(id, None).unwrap_err();
        assert!(
            matches!(err, Error::HikeNotFound(i) if i == id),
            "got {err:?}"
        );
    }
    assert_eq!(db.list_hikes().unwrap(), before, "nothing was written");
}

#[test]
fn edits_are_listed() {
    let db = seeded();
    db.set_title(2, Some("Titled")).unwrap();
    db.set_note(2, Some("Noted")).unwrap();
    let edited: Vec<_> = db
        .list_hikes()
        .unwrap()
        .into_iter()
        .map(|h| (h.id, h.title, h.note))
        .collect();
    assert_eq!(
        edited,
        vec![
            (1, None, None),
            (2, Some("Titled".into()), Some("Noted".into()))
        ]
    );
}

#[test]
fn reimporting_the_same_gpx_keeps_title_and_note() {
    let mut db = seeded();
    db.set_title(1, Some("Titled")).unwrap();
    db.set_note(1, Some("Noted")).unwrap();
    let before = db.list_hikes().unwrap();

    import_into(
        &mut db,
        &gpx_with(&[trk("Renamed at the source", 50.0), trk("Track 2", 51.0)]),
        Some("cycling"),
    );

    assert_eq!(db.list_hikes().unwrap(), before);
}

#[test]
fn recompute_keeps_title_and_note() {
    let mut db = seeded();
    db.set_title(1, Some("Titled")).unwrap();
    db.set_note(1, Some("Noted")).unwrap();
    // Stale stats force recompute to write the row.
    db.connection()
        .execute_batch("UPDATE hikes SET ascent_m = 9999, stats_version = 1")
        .unwrap();

    assert_eq!(db.recompute_stats().unwrap(), 2);

    let h = hike(&db, 1);
    assert_eq!(
        (h.title.as_deref(), h.note.as_deref()),
        (Some("Titled"), Some("Noted"))
    );
}
