//! Km markers on the reduced, anonymised export: they follow the stats' distance exactly.

use std::path::PathBuf;

use hikestat_app_lib::domain::map::map_track;
use hikestat_app_lib::domain::parser::parse;
use hikestat_app_lib::domain::stats::compute;

#[test]
fn km_markers_follow_the_stats_distance() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/export_sample.gpx");
    let gpx = parse(std::fs::File::open(path).unwrap()).unwrap();
    assert!(!gpx.traces.is_empty());
    for trace in &gpx.traces {
        let distance_m = compute(trace).distance_m;
        let markers = map_track(&trace.segments).km_markers;
        let kms: Vec<u32> = markers.iter().map(|m| m.km).collect();
        let whole = (distance_m / 1000.0).floor() as u32;
        assert_eq!(
            kms,
            (1..=whole).collect::<Vec<_>>(),
            "{:?}: {distance_m} m",
            trace.name
        );
    }
}

#[test]
fn the_profile_ends_at_the_stats_distance() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/export_sample.gpx");
    let gpx = parse(std::fs::File::open(path).unwrap()).unwrap();
    for trace in &gpx.traces {
        let track = map_track(&trace.segments);
        let last = *track.segments.last().unwrap().distances_m.last().unwrap();
        assert_eq!(last, compute(trace).distance_m, "{:?}", trace.name);
    }
}
