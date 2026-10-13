//! What the hike panel's map draws: the library's map track, typed for TypeScript.

use hikestat::map::{self, Position};
use hikestat::storage::Db;
use serde::Serialize;
use specta::Type;

/// `[lon, lat]`, the GeoJSON order.
pub type LonLat = (f64, f64);
/// How `LonLat` crosses to TypeScript: coordinates are never null (specta assumes an `f64` may be).
type TsLonLat = (specta_typescript::Number, specta_typescript::Number);

/// An intervalle, from one position to the next.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Interval {
    /// Coloured by its smoothed speed, in m/s.
    #[serde(rename_all = "camelCase")]
    Moving {
        #[specta(type = specta_typescript::Number)]
        speed_mps: f64,
    },
    Stopped,
    /// A point without time, or a non-positive time step.
    Untimed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    #[specta(type = Vec<TsLonLat>)]
    pub positions: Vec<LonLat>,
    /// One per pair of consecutive positions.
    pub intervals: Vec<Interval>,
    /// For the profile: the distance from the start of the hike at each position, in metres.
    #[specta(type = Vec<specta_typescript::Number>)]
    pub distances_m: Vec<f64>,
    /// For the profile: the elevation at each position, `null` if missing or a GPS glitch.
    pub elevations_m: Vec<Option<f64>>,
}

/// A trou de signal, drawn as a dashed line from the end of a segment to the start of the next.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct Gap {
    #[specta(type = TsLonLat)]
    pub from: LonLat,
    #[specta(type = TsLonLat)]
    pub to: LonLat,
}

/// The speeds at both ends of the colour scale, in m/s.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SpeedScale {
    #[specta(type = specta_typescript::Number)]
    pub low_mps: f64,
    #[specta(type = specta_typescript::Number)]
    pub high_mps: f64,
}

/// Where the hike reaches `km` whole kilometres (counted like the stats' distance).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Type)]
pub struct KmMarker {
    pub km: u32,
    #[specta(type = TsLonLat)]
    pub position: LonLat,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MapTrack {
    /// The segments in order, each drawn as its own line; none if the hike has no points.
    pub segments: Vec<Segment>,
    pub gaps: Vec<Gap>,
    /// `null` if no intervalle is moving: the track is drawn in the accent colour.
    pub speed_scale: Option<SpeedScale>,
    /// One per whole kilometre, in order; none at 0.
    pub km_markers: Vec<KmMarker>,
}

fn lon_lat(p: Position) -> LonLat {
    (p.lon, p.lat)
}

fn interval(i: map::Interval) -> Interval {
    match i {
        map::Interval::Moving { speed_mps } => Interval::Moving { speed_mps },
        map::Interval::Stopped => Interval::Stopped,
        map::Interval::Untimed => Interval::Untimed,
    }
}

/// The map track of the hike `id`, or an error naming it if there is none.
pub fn track(db: &Db, id: i64) -> Result<MapTrack, String> {
    let segments = db
        .track(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("no hike with id {id}"))?;
    let track = map::map_track(&segments);
    Ok(MapTrack {
        segments: track
            .segments
            .into_iter()
            .map(|seg| Segment {
                positions: seg.positions.into_iter().map(lon_lat).collect(),
                intervals: seg.intervals.into_iter().map(interval).collect(),
                distances_m: seg.distances_m,
                elevations_m: seg.elevations_m,
            })
            .collect(),
        gaps: track
            .gaps
            .into_iter()
            .map(|g| Gap {
                from: lon_lat(g.from),
                to: lon_lat(g.to),
            })
            .collect(),
        speed_scale: track.speed_scale.map(|s| SpeedScale {
            low_mps: s.low_mps,
            high_mps: s.high_mps,
        }),
        km_markers: track
            .km_markers
            .into_iter()
            .map(|m| KmMarker {
                km: m.km,
                position: lon_lat(m.position),
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `(segment_idx, lat, lon, time)` rows.
    fn db_with_points(id: i64, points: &[(i64, f64, f64, Option<i64>)]) -> Db {
        let db = Db::open_in_memory().unwrap();
        let conn = db.connection();
        conn.execute("INSERT INTO hikes (id, distance_m) VALUES (?1, 0)", [id])
            .unwrap();
        for (idx, &(segment_idx, lat, lon, time)) in points.iter().enumerate() {
            conn.execute(
                "INSERT INTO track_points (hike_id, segment_idx, idx, lat, lon, time) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                (id, segment_idx, idx as i64, lat, lon, time),
            )
            .unwrap();
        }
        db
    }

    #[test]
    fn coordinates_are_sent_lon_first_with_one_gap_between_segments() {
        let db = db_with_points(
            3,
            &[
                (0, 45.0, 6.0, None),
                (0, 45.1, 6.1, None),
                (1, 45.2, 6.2, None),
            ],
        );
        let t = track(&db, 3).unwrap();
        let positions: Vec<_> = t.segments.iter().map(|s| s.positions.clone()).collect();
        assert_eq!(
            positions,
            vec![vec![(6.0, 45.0), (6.1, 45.1)], vec![(6.2, 45.2)]]
        );
        assert_eq!(
            t.gaps,
            vec![Gap {
                from: (6.1, 45.1),
                to: (6.2, 45.2)
            }]
        );
    }

    #[test]
    fn intervals_and_the_speed_scale_are_passed_on() {
        // About 11 m in 10 s, then the same place: moving, then stopped, then untimed.
        let db = db_with_points(
            3,
            &[
                (0, 0.0, 0.0, Some(0)),
                (0, 0.0, 0.0001, Some(10)),
                (0, 0.0, 0.0001, Some(20)),
                (0, 0.0, 0.0002, None),
            ],
        );
        let t = track(&db, 3).unwrap();
        let intervals = &t.segments[0].intervals;
        let Interval::Moving { speed_mps } = intervals[0] else {
            panic!("not moving: {:?}", intervals[0]);
        };
        assert!((speed_mps - 1.112).abs() < 0.01, "got {speed_mps}");
        assert_eq!(intervals[1..], [Interval::Stopped, Interval::Untimed]);
        let scale = t.speed_scale.unwrap();
        assert_eq!((scale.low_mps, scale.high_mps), (speed_mps, speed_mps));
    }

    #[test]
    fn km_markers_are_passed_on_lon_first() {
        // 0.01 degree of latitude is about 1.1 km.
        let db = db_with_points(3, &[(0, 45.0, 6.0, None), (0, 45.01, 6.0, None)]);
        let markers = track(&db, 3).unwrap().km_markers;
        assert_eq!(markers.len(), 1);
        assert_eq!((markers[0].km, markers[0].position.0), (1, 6.0));
        assert!((markers[0].position.1 - 45.009).abs() < 0.001);
    }

    #[test]
    fn the_profile_is_passed_on_aligned_with_the_positions() {
        let db = db_with_points(3, &[(0, 45.0, 6.0, None), (0, 45.01, 6.0, None)]);
        db.connection()
            .execute("UPDATE track_points SET ele = 850 WHERE idx = 0", [])
            .unwrap();
        let segment = &track(&db, 3).unwrap().segments[0];
        assert_eq!(segment.distances_m.len(), segment.positions.len());
        assert_eq!(segment.distances_m[0], 0.0);
        assert!((segment.distances_m[1] - 1112.0).abs() < 1.0);
        assert_eq!(segment.elevations_m, vec![Some(850.0), None]);
    }

    #[test]
    fn a_hike_without_points_has_no_segments_and_no_scale() {
        let db = db_with_points(3, &[]);
        let t = track(&db, 3).unwrap();
        assert_eq!((t.segments, t.speed_scale), (vec![], None));
    }

    #[test]
    fn an_unknown_hike_is_an_error() {
        let db = db_with_points(3, &[]);
        assert_eq!(track(&db, 4), Err("no hike with id 4".to_string()));
    }
}
