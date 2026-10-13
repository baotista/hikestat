use crate::domain::parser::{Point, Trace};

const EARTH_RADIUS_M: f64 = 6_371_008.8;

/// Bump when the algorithm or any constant below changes, so `recompute` can find stale rows.
pub const STATS_VERSION: i64 = 2;
/// Elevations outside this range are GPS glitches (the example export has a lone -4979 m point):
/// they are ignored by the elevation stats. The raw points are kept untouched in the database.
pub const MIN_VALID_ELEVATION_M: f64 = -500.0;
pub const MAX_VALID_ELEVATION_M: f64 = 9_000.0;
/// A climb or descent only counts once it exceeds this cumulative change since the last counted point.
pub const ELEVATION_HYSTERESIS_M: f64 = 3.0;
/// Below this speed an interval counts as stopped.
pub const STOP_SPEED_MPS: f64 = 0.3;
/// A longer time gap between two points counts as stopped (e.g. signal loss).
pub const MAX_MOVING_GAP_S: i64 = 60;

#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub distance_m: f64,
    /// `None` if the trace has no elevation data.
    pub ascent_m: Option<f64>,
    pub descent_m: Option<f64>,
    pub min_ele_m: Option<f64>,
    pub max_ele_m: Option<f64>,
    /// Last timestamp minus first timestamp, `None` if fewer than two timed points.
    pub duration_total_s: Option<i64>,
    /// Sum of the intervals that are not stopped, `None` if no interval has usable timestamps.
    pub duration_moving_s: Option<i64>,
    /// Distance covered while moving divided by the moving duration, `None` if never moving.
    pub avg_speed_mps: Option<f64>,
}

pub fn compute(trace: &Trace) -> Stats {
    let (ascent_m, descent_m) = elevation_change(&trace.segments);
    let ele = || valid_elevations(&trace.segments);
    let moving = moving(&trace.segments);
    Stats {
        distance_m: distance_m(&trace.segments),
        ascent_m,
        descent_m,
        min_ele_m: ele().reduce(f64::min),
        max_ele_m: ele().reduce(f64::max),
        duration_total_s: duration_total_s(&trace.segments),
        duration_moving_s: moving.as_ref().map(|m| m.duration_s),
        avg_speed_mps: moving
            .filter(|m| m.duration_s > 0)
            .map(|m| m.distance_m / m.duration_s as f64),
    }
}

/// Haversine, 2D, summed per segment so a gap between segments adds nothing.
fn distance_m(segments: &[Vec<Point>]) -> f64 {
    segments
        .iter()
        .flat_map(|seg| seg.windows(2))
        .map(|w| haversine_m(&w[0], &w[1]))
        .sum()
}

pub(crate) fn haversine_m(a: &Point, b: &Point) -> f64 {
    let (lat1, lat2) = (a.lat.to_radians(), b.lat.to_radians());
    let dlat = lat2 - lat1;
    let dlon = (b.lon - a.lon).to_radians();
    let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_M * h.sqrt().asin()
}

fn duration_total_s(segments: &[Vec<Point>]) -> Option<i64> {
    let mut times = segments.iter().flatten().filter_map(|p| p.time);
    let first = times.next()?;
    let last = times.next_back()?;
    Some(last - first)
}

fn valid_elevations(segments: &[Vec<Point>]) -> impl Iterator<Item = f64> + '_ {
    segments
        .iter()
        .flatten()
        .filter_map(|p| p.ele)
        .filter(|e| (MIN_VALID_ELEVATION_M..=MAX_VALID_ELEVATION_M).contains(e))
}

/// Hysteresis over the points that have a valid elevation, in order, across segments.
fn elevation_change(segments: &[Vec<Point>]) -> (Option<f64>, Option<f64>) {
    let mut eles = valid_elevations(segments);
    let Some(mut reference) = eles.next() else {
        return (None, None);
    };
    let (mut ascent, mut descent) = (0.0, 0.0);
    for e in eles {
        let diff = e - reference;
        if diff >= ELEVATION_HYSTERESIS_M {
            ascent += diff;
            reference = e;
        } else if diff <= -ELEVATION_HYSTERESIS_M {
            descent -= diff;
            reference = e;
        }
    }
    (Some(ascent), Some(descent))
}

struct Moving {
    duration_s: i64,
    distance_m: f64,
}

/// What the passage from one point to the next (an intervalle) is, for the stats and the map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Step {
    Moving {
        distance_m: f64,
        duration_s: i64,
    },
    Stopped,
    /// A missing timestamp or a non-positive time step: no usable time (no division by zero).
    Untimed,
}

pub(crate) fn step(a: &Point, b: &Point) -> Step {
    let (Some(t0), Some(t1)) = (a.time, b.time) else {
        return Step::Untimed;
    };
    let dt = t1 - t0;
    if dt <= 0 {
        return Step::Untimed;
    }
    let d = haversine_m(a, b);
    if dt <= MAX_MOVING_GAP_S && d / dt as f64 >= STOP_SPEED_MPS {
        Step::Moving {
            distance_m: d,
            duration_s: dt,
        }
    } else {
        Step::Stopped
    }
}

/// Untimed intervals are ignored; `None` if every interval is.
fn moving(segments: &[Vec<Point>]) -> Option<Moving> {
    let mut acc: Option<Moving> = None;
    for w in segments.iter().flat_map(|seg| seg.windows(2)) {
        let step = step(&w[0], &w[1]);
        if step == Step::Untimed {
            continue;
        }
        let acc = acc.get_or_insert(Moving {
            duration_s: 0,
            distance_m: 0.0,
        });
        if let Step::Moving {
            distance_m,
            duration_s,
        } = step
        {
            acc.duration_s += duration_s;
            acc.distance_m += distance_m;
        }
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(lat: f64, lon: f64, time: Option<i64>) -> Point {
        Point {
            lat,
            lon,
            ele: None,
            time,
        }
    }

    #[test]
    fn one_degree_of_latitude_is_about_111_km() {
        let seg = vec![pt(0.0, 0.0, None), pt(1.0, 0.0, None)];
        let d = distance_m(&[seg]);
        assert!((d - 111_195.0).abs() < 10.0, "got {d}");
    }

    #[test]
    fn gap_between_segments_adds_no_distance() {
        let a = vec![pt(0.0, 0.0, None), pt(0.0, 0.001, None)];
        let b = vec![pt(10.0, 10.0, None), pt(10.0, 10.001, None)];
        let joined = distance_m(&[a.clone(), b.clone()]);
        let separate = distance_m(&[a]) + distance_m(&[b]);
        assert_eq!(joined, separate);
    }

    #[test]
    fn total_duration_spans_first_to_last_timed_point() {
        let seg = vec![
            pt(0.0, 0.0, None),
            pt(0.0, 0.0, Some(100)),
            pt(0.0, 0.0, Some(160)),
        ];
        assert_eq!(duration_total_s(&[seg]), Some(60));
    }

    #[test]
    fn no_timestamps_gives_no_duration() {
        let seg = vec![pt(0.0, 0.0, None), pt(0.0, 0.1, None)];
        assert_eq!(duration_total_s(&[seg]), None);
    }

    fn ele_pt(ele: Option<f64>) -> Point {
        Point {
            lat: 0.0,
            lon: 0.0,
            ele,
            time: None,
        }
    }

    fn eles(values: &[f64]) -> Vec<Vec<Point>> {
        vec![values.iter().map(|&e| ele_pt(Some(e))).collect()]
    }

    #[test]
    fn climb_counts_once_it_reaches_the_threshold() {
        assert_eq!(
            elevation_change(&eles(&[0.0, 1.0, 2.0, 3.0])),
            (Some(3.0), Some(0.0))
        );
    }

    #[test]
    fn oscillation_below_threshold_counts_for_nothing() {
        assert_eq!(
            elevation_change(&eles(&[0.0, 2.0, 0.0, 2.0, 0.0])),
            (Some(0.0), Some(0.0))
        );
    }

    #[test]
    fn real_gps_noise_on_flat_ground_gives_no_ascent() {
        // Taken from the first points of the example export.
        let noisy = eles(&[30.7, 29.7, 29.2, 29.7, 27.0, 26.0, 28.7, 29.1]);
        let (ascent, descent) = elevation_change(&noisy);
        assert_eq!(ascent, Some(0.0));
        assert!((descent.unwrap() - 3.7).abs() < 1e-9);
    }

    #[test]
    fn ascent_and_descent_accumulate() {
        assert_eq!(
            elevation_change(&eles(&[0.0, 5.0, 1.0, 6.0])),
            (Some(10.0), Some(4.0))
        );
    }

    #[test]
    fn missing_elevations_are_skipped_and_none_means_no_data() {
        let seg = vec![ele_pt(Some(0.0)), ele_pt(None), ele_pt(Some(4.0))];
        assert_eq!(elevation_change(&[seg]), (Some(4.0), Some(0.0)));
        assert_eq!(elevation_change(&[vec![ele_pt(None)]]), (None, None));
    }

    fn at(lon: f64, time: Option<i64>) -> Point {
        Point {
            lat: 0.0,
            lon,
            ele: None,
            time,
        }
    }

    #[test]
    fn moving_time_excludes_stops_gaps_and_zero_steps() {
        // 0.0001 deg of longitude at the equator is about 11.1 m.
        let seg = vec![
            at(0.0, Some(0)),
            at(0.0001, Some(10)),  // 1.1 m/s: moving
            at(0.0001, Some(20)),  // same place: stopped
            at(0.0002, Some(120)), // 100 s gap: stopped
            at(0.0003, Some(120)), // zero time step: ignored
        ];
        let m = moving(std::slice::from_ref(&seg)).unwrap();
        assert_eq!(m.duration_s, 10);
        assert!((m.distance_m - 11.12).abs() < 0.05, "got {}", m.distance_m);

        let stats = compute(&Trace {
            name: None,
            kind: None,
            segments: vec![seg],
        });
        assert_eq!(stats.duration_total_s, Some(120));
        assert_eq!(stats.duration_moving_s, Some(10));
        assert!((stats.avg_speed_mps.unwrap() - 1.112).abs() < 0.01);
    }

    #[test]
    fn intervals_without_timestamps_do_not_count_as_moving() {
        let seg = vec![at(0.0, None), at(0.0001, Some(10)), at(0.0002, None)];
        assert!(moving(&[seg]).is_none());
    }

    #[test]
    fn never_moving_has_zero_moving_time_and_no_speed() {
        let seg = vec![at(0.0, Some(0)), at(0.0, Some(10)), at(0.0, Some(20))];
        let stats = compute(&Trace {
            name: None,
            kind: None,
            segments: vec![seg],
        });
        assert_eq!(stats.duration_moving_s, Some(0));
        assert_eq!(stats.avg_speed_mps, None);
    }

    #[test]
    fn min_and_max_use_raw_elevations() {
        let stats = compute(&Trace {
            name: None,
            kind: None,
            segments: eles(&[10.0, 12.0, 9.0]),
        });
        assert_eq!((stats.min_ele_m, stats.max_ele_m), (Some(9.0), Some(12.0)));
    }

    #[test]
    fn a_lone_glitch_elevation_is_ignored() {
        // The -4979 m point of the example export, between two ordinary readings.
        let stats = compute(&Trace {
            name: None,
            kind: None,
            segments: eles(&[30.0, -4979.0, 31.0]),
        });
        assert_eq!(stats.ascent_m, Some(0.0));
        assert_eq!(stats.descent_m, Some(0.0));
        assert_eq!((stats.min_ele_m, stats.max_ele_m), (Some(30.0), Some(31.0)));
    }

    #[test]
    fn range_bounds_are_inclusive() {
        let inside = eles(&[-500.0, 9_000.0]);
        assert_eq!(valid_elevations(&inside).count(), 2);
        let outside = eles(&[-500.1, 9_000.1]);
        assert_eq!(valid_elevations(&outside).count(), 0);
    }

    #[test]
    fn only_glitches_means_no_elevation_data() {
        let stats = compute(&Trace {
            name: None,
            kind: None,
            segments: eles(&[-4979.0]),
        });
        assert_eq!(stats.ascent_m, None);
        assert_eq!(stats.min_ele_m, None);
    }
}
