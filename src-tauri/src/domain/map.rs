//! What a map of one hike draws, computed from its stored points. A pure function, nothing stored.

use crate::domain::parser::Point;
use crate::domain::stats::{MAX_VALID_ELEVATION_M, MIN_VALID_ELEVATION_M, Step, haversine_m, step};

/// The distance (centred on an intervalle) over which its vitesse lissée is averaged. It only
/// changes the map, not the stored stats: no `STATS_VERSION` bump.
pub const SPEED_SMOOTHING_M: f64 = 50.0;

/// A point on the map: only its coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    pub lat: f64,
    pub lon: f64,
}

/// An intervalle, from one point to the next in a segment, with the stats' stop thresholds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Interval {
    Moving {
        /// Vitesse lissée, in m/s.
        speed_mps: f64,
    },
    Stopped,
    /// A point without time, or a non-positive time step.
    Untimed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub positions: Vec<Position>,
    /// One per pair of consecutive positions (none for a segment of a single point).
    pub intervals: Vec<Interval>,
    /// For the profil: the distance from the start of the hike at each position, in metres, counted
    /// like the stats' distance (nothing across a trou de signal).
    pub distances_m: Vec<f64>,
    /// For the profil: the elevation at each position, `None` if missing or outside the stats'
    /// valid range (a GPS glitch).
    pub elevations_m: Vec<Option<f64>>,
}

/// A trou de signal: from the last point of a segment to the first point of the next one.
/// Nothing is measured across it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gap {
    pub from: Position,
    pub to: Position,
}

/// The bounds of the speed colours: the 5th and 95th centiles of the vitesses lissées, so that a
/// lone spike doesn't flatten the colours of the whole hike.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedScale {
    pub low_mps: f64,
    pub high_mps: f64,
}

/// A point kilométrique: where the hike reaches `km` whole kilometres, counted like the stats'
/// distance (nothing across a trou de signal).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KmMarker {
    pub km: u32,
    pub position: Position,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MapTrack {
    /// The segments in order, each drawn as its own line. Segments without points are left out.
    pub segments: Vec<Segment>,
    /// One gap between each pair of consecutive segments.
    pub gaps: Vec<Gap>,
    /// `None` if no intervalle is moving.
    pub speed_scale: Option<SpeedScale>,
    /// One per whole kilometre, in order; none at 0.
    pub km_markers: Vec<KmMarker>,
}

pub fn map_track(points: &[Vec<Point>]) -> MapTrack {
    let mut along = 0.0;
    let segments: Vec<Segment> = points
        .iter()
        .filter(|seg| !seg.is_empty())
        .map(|seg| segment(seg, &mut along))
        .collect();
    let gaps = segments
        .windows(2)
        .map(|w| Gap {
            from: *w[0].positions.last().expect("empty segments are left out"),
            to: w[1].positions[0],
        })
        .collect();
    let mut speeds: Vec<f64> = segments
        .iter()
        .flat_map(|seg| &seg.intervals)
        .filter_map(|i| match i {
            Interval::Moving { speed_mps } => Some(*speed_mps),
            _ => None,
        })
        .collect();
    MapTrack {
        segments,
        gaps,
        speed_scale: speed_scale(&mut speeds),
        km_markers: km_markers(points),
    }
}

/// Summed in the same order as the stats' distance, so the last marker is never beyond it.
fn km_markers(segments: &[Vec<Point>]) -> Vec<KmMarker> {
    let mut markers = Vec::new();
    let mut along = 0.0;
    for w in segments.iter().flat_map(|seg| seg.windows(2)) {
        let d = haversine_m(&w[0], &w[1]);
        let end = along + d;
        // Every kilometre up to `along` is placed already, so `d` is positive here.
        let mut next = markers.len() as u32 + 1;
        while f64::from(next) * 1000.0 <= end {
            let t = (f64::from(next) * 1000.0 - along) / d;
            markers.push(KmMarker {
                km: next,
                position: Position {
                    lat: w[0].lat + t * (w[1].lat - w[0].lat),
                    lon: w[0].lon + t * (w[1].lon - w[0].lon),
                },
            });
            next += 1;
        }
        along = end;
    }
    markers
}

/// `start_m`: the distance covered before this segment, advanced to its end.
fn segment(points: &[Point], start_m: &mut f64) -> Segment {
    let positions = points
        .iter()
        .map(|p| Position {
            lat: p.lat,
            lon: p.lon,
        })
        .collect();
    let steps: Vec<Step> = points.windows(2).map(|w| step(&w[0], &w[1])).collect();

    // Distance along the segment to the middle of each intervalle, as the stats count it.
    let mut along = 0.0;
    let middles: Vec<f64> = points
        .windows(2)
        .map(|w| {
            let d = haversine_m(&w[0], &w[1]);
            along += d;
            along - d / 2.0
        })
        .collect();

    // The moving intervalles, with running sums of their distance and time.
    let moving: Vec<(usize, f64)> = steps
        .iter()
        .enumerate()
        .filter(|(_, s)| matches!(s, Step::Moving { .. }))
        .map(|(i, _)| (i, middles[i]))
        .collect();
    let (mut sum_d, mut sum_t) = (vec![0.0], vec![0i64]);
    for &(i, _) in &moving {
        let Step::Moving {
            distance_m,
            duration_s,
        } = steps[i]
        else {
            unreachable!("only moving steps are kept");
        };
        sum_d.push(sum_d.last().unwrap() + distance_m);
        sum_t.push(sum_t.last().unwrap() + duration_s);
    }

    let half = SPEED_SMOOTHING_M / 2.0;
    let mut speeds = moving.iter().map(|&(_, middle)| {
        let lo = moving.partition_point(|&(_, m)| m < middle - half);
        let hi = moving.partition_point(|&(_, m)| m <= middle + half);
        (sum_d[hi] - sum_d[lo]) / (sum_t[hi] - sum_t[lo]) as f64
    });
    let intervals = steps
        .iter()
        .map(|s| match s {
            Step::Moving { .. } => Interval::Moving {
                speed_mps: speeds.next().expect("one speed per moving step"),
            },
            Step::Stopped => Interval::Stopped,
            Step::Untimed => Interval::Untimed,
        })
        .collect();
    let mut distances_m = vec![*start_m];
    for w in points.windows(2) {
        // Summed in the same order as the stats' distance, so the last one equals it.
        *start_m += haversine_m(&w[0], &w[1]);
        distances_m.push(*start_m);
    }
    let elevations_m = points
        .iter()
        .map(|p| {
            p.ele
                .filter(|e| (MIN_VALID_ELEVATION_M..=MAX_VALID_ELEVATION_M).contains(e))
        })
        .collect();
    Segment {
        positions,
        intervals,
        distances_m,
        elevations_m,
    }
}

/// Nearest-rank centiles.
fn speed_scale(speeds: &mut [f64]) -> Option<SpeedScale> {
    if speeds.is_empty() {
        return None;
    }
    speeds.sort_by(f64::total_cmp);
    let rank = |p: f64| speeds[((p * speeds.len() as f64).ceil() as usize).max(1) - 1];
    Some(SpeedScale {
        low_mps: rank(0.05),
        high_mps: rank(0.95),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(lat: f64, lon: f64) -> Point {
        Point {
            lat,
            lon,
            ele: None,
            time: None,
        }
    }

    fn pos(lat: f64, lon: f64) -> Position {
        Position { lat, lon }
    }

    /// On the equator, `m` metres east of longitude 0, at `time`.
    fn east(m: f64, time: Option<i64>) -> Point {
        Point {
            lat: 0.0,
            lon: m / 111_195.08,
            ele: None,
            time,
        }
    }

    fn positions(track: &MapTrack) -> Vec<Vec<Position>> {
        track.segments.iter().map(|s| s.positions.clone()).collect()
    }

    fn speeds(track: &MapTrack) -> Vec<f64> {
        track.segments[0]
            .intervals
            .iter()
            .map(|i| match i {
                Interval::Moving { speed_mps } => *speed_mps,
                other => panic!("not moving: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn no_points_give_an_empty_map_track() {
        assert_eq!(map_track(&[]), MapTrack::default());
        assert_eq!(map_track(&[vec![], vec![]]), MapTrack::default());
    }

    #[test]
    fn one_segment_keeps_its_positions_in_order_and_has_no_gap() {
        let track = map_track(&[vec![p(45.0, 6.0), p(45.1, 6.1), p(45.2, 6.0)]]);
        assert_eq!(
            positions(&track),
            vec![vec![pos(45.0, 6.0), pos(45.1, 6.1), pos(45.2, 6.0)]]
        );
        assert_eq!(track.gaps, vec![]);
    }

    #[test]
    fn consecutive_segments_are_joined_by_one_gap_each() {
        let track = map_track(&[
            vec![p(45.0, 6.0), p(45.1, 6.0)],
            vec![p(45.2, 6.0), p(45.3, 6.0)],
            vec![p(45.4, 6.0)],
        ]);
        assert_eq!(track.segments.len(), 3);
        assert_eq!(
            track.gaps,
            vec![
                Gap {
                    from: pos(45.1, 6.0),
                    to: pos(45.2, 6.0)
                },
                Gap {
                    from: pos(45.3, 6.0),
                    to: pos(45.4, 6.0)
                },
            ]
        );
    }

    #[test]
    fn an_empty_segment_is_skipped_and_the_gap_spans_it() {
        let track = map_track(&[vec![p(45.0, 6.0)], vec![], vec![p(45.2, 6.0)]]);
        assert_eq!(
            positions(&track),
            vec![vec![pos(45.0, 6.0)], vec![pos(45.2, 6.0)]]
        );
        assert_eq!(
            track.gaps,
            vec![Gap {
                from: pos(45.0, 6.0),
                to: pos(45.2, 6.0)
            }]
        );
    }

    #[test]
    fn intervals_are_classified_with_the_stats_thresholds() {
        let track = map_track(&[vec![
            east(0.0, Some(0)),
            east(11.0, Some(10)),  // 1.1 m/s: moving
            east(11.0, Some(20)),  // same place: stopped
            east(22.0, Some(120)), // 100 s gap: stopped
            east(33.0, Some(120)), // zero time step: untimed
            east(44.0, None),      // no time: untimed
        ]]);
        let classes: Vec<&str> = track.segments[0]
            .intervals
            .iter()
            .map(|i| match i {
                Interval::Moving { .. } => "moving",
                Interval::Stopped => "stopped",
                Interval::Untimed => "untimed",
            })
            .collect();
        assert_eq!(
            classes,
            vec!["moving", "stopped", "stopped", "untimed", "untimed"]
        );
        let Interval::Moving { speed_mps } = track.segments[0].intervals[0] else {
            unreachable!()
        };
        assert!((speed_mps - 1.1).abs() < 0.01, "got {speed_mps}");
    }

    #[test]
    fn a_single_point_segment_has_no_interval() {
        assert_eq!(
            map_track(&[vec![east(0.0, Some(0))]]).segments[0].intervals,
            vec![]
        );
    }

    #[test]
    fn smoothing_flattens_a_jittery_point() {
        // 11 m every 10 s (1.1 m/s), but point 10 is 5 m too far east: raw speeds 1.6 then 0.6.
        let points: Vec<Point> = (0..20)
            .map(|i| {
                let jitter = if i == 10 { 5.0 } else { 0.0 };
                east(11.0 * i as f64 + jitter, Some(10 * i))
            })
            .collect();
        for speed in speeds(&map_track(&[points])) {
            assert!((speed - 1.1).abs() < 0.2, "got {speed}");
        }
    }

    #[test]
    fn the_scale_spans_the_5th_to_the_95th_centile() {
        // Intervalles longer than the smoothing distance keep their own speed: 10, 20, … 200 m/s.
        let mut points = vec![east(0.0, Some(0))];
        let mut along = 0.0;
        for i in 1..=20 {
            along += 100.0 * i as f64;
            points.push(east(along, Some(10 * i)));
        }
        let track = map_track(&[points]);
        let speeds = speeds(&track);
        assert!((speeds[0] - 10.0).abs() < 0.01 && (speeds[19] - 200.0).abs() < 0.01);
        let scale = track.speed_scale.expect("something moves");
        assert!((scale.low_mps - 10.0).abs() < 0.01, "got {scale:?}");
        assert!((scale.high_mps - 190.0).abs() < 0.01, "got {scale:?}");
    }

    #[test]
    fn nothing_moving_gives_no_scale() {
        let stopped = vec![east(0.0, Some(0)), east(0.0, Some(10))];
        assert_eq!(map_track(&[stopped]).speed_scale, None);
        let untimed = vec![east(0.0, None), east(100.0, None)];
        assert_eq!(map_track(&[untimed]).speed_scale, None);
    }

    fn km_markers(segments: &[Vec<Point>]) -> Vec<(u32, f64)> {
        map_track(segments)
            .km_markers
            .iter()
            .map(|m| (m.km, m.position.lon * 111_195.08))
            .collect()
    }

    fn assert_markers(found: Vec<(u32, f64)>, expected: &[(u32, f64)]) {
        assert_eq!(found.len(), expected.len(), "{found:?}");
        for ((km, m), (want_km, want_m)) in found.iter().zip(expected) {
            assert_eq!(km, want_km);
            assert!(
                (m - want_m).abs() < 0.5,
                "km {km} at {m} m, expected {want_m} m"
            );
        }
    }

    #[test]
    fn km_markers_are_interpolated_between_points() {
        let points = vec![east(0.0, None), east(1500.0, None), east(2500.0, None)];
        assert_markers(km_markers(&[points]), &[(1, 1000.0), (2, 2000.0)]);
    }

    #[test]
    fn a_marker_falls_on_a_point_at_a_whole_kilometre() {
        let points = vec![east(0.0, None), east(1000.0, None), east(1200.0, None)];
        assert_markers(km_markers(&[points]), &[(1, 1000.0)]);
    }

    #[test]
    fn under_a_kilometre_there_is_no_marker_not_even_0() {
        assert_eq!(
            km_markers(&[vec![east(0.0, None), east(999.0, None)]]),
            vec![]
        );
        assert_eq!(km_markers(&[vec![east(0.0, None)]]), vec![]);
        assert_eq!(km_markers(&[]), vec![]);
    }

    #[test]
    fn a_trou_de_signal_adds_no_distance() {
        // 600 m, a 4.4 km jump, then 600 m: the first kilometre is reached 400 m into the second.
        let track = [
            vec![east(0.0, None), east(600.0, None)],
            vec![east(5000.0, None), east(5600.0, None)],
        ];
        assert_markers(km_markers(&track), &[(1, 5400.0)]);
    }

    fn with_ele(m: f64, ele: Option<f64>) -> Point {
        Point {
            ele,
            ..east(m, None)
        }
    }

    #[test]
    fn distances_run_from_the_start_and_add_nothing_across_a_gap() {
        let track = map_track(&[
            vec![east(0.0, None), east(1000.0, None)],
            vec![east(5000.0, None), east(5600.0, None)],
        ]);
        let distances: Vec<Vec<f64>> = track
            .segments
            .iter()
            .map(|s| s.distances_m.iter().map(|d| d.round()).collect())
            .collect();
        assert_eq!(distances, vec![vec![0.0, 1000.0], vec![1000.0, 1600.0]]);
    }

    #[test]
    fn missing_and_glitch_elevations_are_absent() {
        let track = map_track(&[vec![
            with_ele(0.0, Some(120.0)),
            with_ele(10.0, None),
            with_ele(20.0, Some(-4979.0)),
            with_ele(30.0, Some(9_000.0)),
        ]]);
        assert_eq!(
            track.segments[0].elevations_m,
            vec![Some(120.0), None, None, Some(9_000.0)]
        );
    }

    #[test]
    fn smoothing_stays_within_a_segment() {
        // A slow segment then a fast one: the gap between them doesn't mix their speeds.
        let slow: Vec<Point> = (0..5).map(|i| east(5.0 * i as f64, Some(10 * i))).collect();
        let fast: Vec<Point> = (0..5)
            .map(|i| east(25.0 + 20.0 * i as f64, Some(100 + 10 * i)))
            .collect();
        let track = map_track(&[slow, fast]);
        for i in &track.segments[0].intervals {
            let Interval::Moving { speed_mps } = i else {
                panic!("not moving: {i:?}")
            };
            assert!((speed_mps - 0.5).abs() < 0.01, "got {speed_mps}");
        }
    }
}
