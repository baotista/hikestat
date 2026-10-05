//! Creates a throwaway database with a few made-up hikes this week and last week, to look at the
//! dashboard without real data:
//!
//! ```sh
//! cargo run --example demo_db -- /tmp/hikestat-demo.db
//! ```
//!
//! Then choose that file in the app (Settings → Choose file…).

use std::path::PathBuf;

use hikestat::parser::{Point, Trace};
use hikestat::storage::Db;
use hikestat::{identity, stats};
use jiff::{ToSpan, Zoned, civil::Time};

/// A zone and the coordinates of a start point.
type Place = (&'static str, f64, f64);
/// (days from this week's Monday, local start time, place, km, with elevation?, kind)
type Demo = (i64, Time, Place, f64, bool, Option<&'static str>);

const PARIS: Place = ("Europe/Paris", 48.85, 2.35);
const GRENOBLE: Place = ("Europe/Paris", 45.19, 5.72);
const TOKYO: Place = ("Asia/Tokyo", 35.68, 139.65);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path: PathBuf = std::env::args()
        .nth(1)
        .ok_or("usage: demo_db <path of a new database>")?
        .into();
    if path.exists() {
        return Err(format!("{} already exists", path.display()).into());
    }

    let today = Zoned::now().date();
    let monday = today.checked_sub(i64::from(today.weekday().to_monday_zero_offset()).days())?;
    let t = |h, m| Time::constant(h, m, 0, 0);
    let demos: &[Demo] = &[
        // Last week.
        (-6, t(9, 0), GRENOBLE, 12.4, true, Some("hiking")),
        (-2, t(14, 30), PARIS, 4.1, true, Some("walking")),
        // This week.
        (0, t(8, 15), GRENOBLE, 9.8, true, Some("hiking")),
        (2, t(7, 0), PARIS, 5.6, true, Some("walking")),
        (2, t(18, 0), PARIS, 3.2, false, None),
        // A 6 a.m. hike in Japan: still the previous evening in Paris.
        (4, t(6, 0), TOKYO, 14.2, true, Some("hiking")),
        // Crosses midnight: counted on its start day.
        (5, t(22, 30), GRENOBLE, 7.5, true, Some("hiking")),
        (6, t(10, 0), PARIS, 31.0, true, Some("cycling")),
    ];

    let mut db = Db::open(&path)?;
    for (n, &(day, time, (zone, lat, lon), km, with_ele, kind)) in demos.iter().enumerate() {
        let start = monday
            .checked_add(day.days())?
            .to_datetime(time)
            .in_tz(zone)?
            .timestamp()
            .as_second();
        let trace = trace(
            &format!("Demo track {}", n + 1),
            start,
            lat,
            lon,
            km,
            with_ele,
        );
        db.insert_hike(
            &trace,
            &stats::compute(&trace),
            &identity::trace_hash(&trace),
            kind,
        )?;
    }
    println!("{} demo hikes written to {}", demos.len(), path.display());
    Ok(())
}

/// A straight track heading north at walking pace, one point every 30 s.
fn trace(name: &str, start: i64, lat: f64, lon: f64, km: f64, with_ele: bool) -> Trace {
    const STEP_M: f64 = 40.0;
    let points = (km * 1000.0 / STEP_M) as i64;
    let segment = (0..=points)
        .map(|i| Point {
            lat: lat + (i as f64 * STEP_M) / 111_195.0,
            lon,
            // Up and down every 25 points, 4 m per point.
            ele: with_ele.then(|| 400.0 + 4.0 * ((i % 50) - 25).abs() as f64),
            time: Some(start + 30 * i),
        })
        .collect();
    Trace {
        name: Some(name.into()),
        kind: None,
        segments: vec![segment],
    }
}
