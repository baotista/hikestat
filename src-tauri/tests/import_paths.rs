//! Importing what is dropped on the app's window: files and folders in one run, one summary.

mod common;

use std::path::PathBuf;

use common::{gpx_with, trk};
use hikestat_app_lib::domain::import::{ImportOptions, import_paths};
use hikestat_app_lib::domain::storage::Db;

fn write(path: &PathBuf, text: &str) {
    std::fs::write(path, text).unwrap();
}

#[test]
fn files_and_folders_are_imported_in_one_summary() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("one.gpx");
    write(&file, &gpx_with(&[trk("Lille", 50.0)]));
    let folder = dir.path().join("folder");
    std::fs::create_dir(&folder).unwrap();
    write(
        &folder.join("two.gpx"),
        &gpx_with(&[trk("Arras", 50.3), trk("Douai", 50.4)]),
    );
    let photo = dir.path().join("photo.jpg");
    write(&photo, "not a gpx");

    let mut db = Db::open_in_memory().unwrap();
    let summary = import_paths(
        &mut db,
        &[file, folder, photo.clone()],
        &ImportOptions::default(),
    );
    assert_eq!((summary.imported, summary.already_present), (3, 0));
    assert_eq!(summary.rejected.len(), 1);
    assert_eq!(summary.rejected[0].file, photo);
    assert_eq!(summary.rejected[0].trace_number, None);
    assert_eq!(summary.rejected[0].reason, "not a .gpx file");
    assert_eq!(db.list_hikes().unwrap().len(), 3);
}

#[test]
fn the_extension_is_matched_in_any_case() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("LOUD.GPX");
    write(&file, &gpx_with(&[trk("Lille", 50.0)]));
    let mut db = Db::open_in_memory().unwrap();
    let summary = import_paths(&mut db, &[file], &ImportOptions::default());
    assert_eq!((summary.imported, summary.rejected.len()), (1, 0));
}

#[test]
fn subfolders_of_a_dropped_folder_are_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("nested");
    std::fs::create_dir(&nested).unwrap();
    write(&nested.join("deep.gpx"), &gpx_with(&[trk("Lille", 50.0)]));
    write(
        &dir.path().join("top.gpx"),
        &gpx_with(&[trk("Arras", 50.3)]),
    );
    let mut db = Db::open_in_memory().unwrap();
    let summary = import_paths(
        &mut db,
        &[dir.path().to_path_buf()],
        &ImportOptions::default(),
    );
    assert_eq!(summary.imported, 1);
}

#[test]
fn the_same_trace_dropped_twice_in_one_run_is_imported_once() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.gpx");
    let b = dir.path().join("b.gpx");
    write(&a, &gpx_with(&[trk("Lille", 50.0)]));
    write(&b, &gpx_with(&[trk("Lille again", 50.0)]));
    let mut db = Db::open_in_memory().unwrap();
    let summary = import_paths(&mut db, &[a, b], &ImportOptions::default());
    assert_eq!((summary.imported, summary.already_present), (1, 1));
}

#[test]
fn a_dry_run_writes_nothing_but_detects_duplicates_across_paths() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.gpx");
    let b = dir.path().join("b.gpx");
    write(&a, &gpx_with(&[trk("Lille", 50.0)]));
    write(&b, &gpx_with(&[trk("Lille again", 50.0)]));
    let mut db = Db::open_in_memory().unwrap();
    let opts = ImportOptions {
        dry_run: true,
        ..ImportOptions::default()
    };
    let summary = import_paths(&mut db, &[a, b], &opts);
    assert_eq!((summary.imported, summary.already_present), (1, 1));
    assert!(db.list_hikes().unwrap().is_empty());
}

#[test]
fn nothing_dropped_imports_nothing() {
    let mut db = Db::open_in_memory().unwrap();
    let summary = import_paths(&mut db, &[], &ImportOptions::default());
    assert_eq!(
        (
            summary.imported,
            summary.already_present,
            summary.rejected.len()
        ),
        (0, 0, 0)
    );
}

#[test]
fn the_ids_of_the_created_hikes_are_reported_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.gpx");
    let b = dir.path().join("b.gpx");
    write(&a, &gpx_with(&[trk("Lille", 50.0), trk("Arras", 50.3)]));
    write(
        &b,
        &gpx_with(&[trk("Lille again", 50.0), trk("Douai", 50.4)]),
    );
    let mut db = Db::open_in_memory().unwrap();

    let summary = import_paths(&mut db, &[a, b.clone()], &ImportOptions::default());
    let listed: Vec<(i64, String)> = db
        .list_hikes()
        .unwrap()
        .into_iter()
        .map(|h| (h.id, h.name.unwrap()))
        .collect();
    let id_of = |name: &str| listed.iter().find(|(_, n)| n == name).unwrap().0;
    // "Lille again" is the same trace as "Lille": already present, not created.
    assert_eq!(
        summary.imported_ids,
        vec![id_of("Lille"), id_of("Arras"), id_of("Douai")]
    );

    let again = import_paths(&mut db, &[b], &ImportOptions::default());
    assert_eq!(again.imported_ids, Vec::<i64>::new());
}

#[test]
fn a_dry_run_reports_no_ids() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.gpx");
    write(&a, &gpx_with(&[trk("Lille", 50.0)]));
    let mut db = Db::open_in_memory().unwrap();
    let opts = ImportOptions {
        dry_run: true,
        ..ImportOptions::default()
    };
    let summary = import_paths(&mut db, &[a], &opts);
    assert_eq!((summary.imported, summary.imported_ids), (1, vec![]));
}
