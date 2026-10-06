//! What the hike panel's map draws: the library's map track, typed for TypeScript.

use hikestat::map::{self, Position};
use hikestat::storage::Db;
use serde::Serialize;
use specta::Type;

/// `[lon, lat]`, the GeoJSON order.
pub type LonLat = (f64, f64);
/// How `LonLat` crosses to TypeScript: coordinates are never null (specta assumes an `f64` may be).
type TsLonLat = (specta_typescript::Number, specta_typescript::Number);

/// A trou de signal, drawn as a dashed line from the end of a segment to the start of the next.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct Gap {
    #[specta(type = TsLonLat)]
    pub from: LonLat,
    #[specta(type = TsLonLat)]
    pub to: LonLat,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct MapTrack {
    /// The segments in order, each drawn as its own line; none if the hike has no points.
    #[specta(type = Vec<Vec<TsLonLat>>)]
    pub segments: Vec<Vec<LonLat>>,
    pub gaps: Vec<Gap>,
}

fn lon_lat(p: Position) -> LonLat {
    (p.lon, p.lat)
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
            .map(|seg| seg.into_iter().map(lon_lat).collect())
            .collect(),
        gaps: track
            .gaps
            .into_iter()
            .map(|g| Gap {
                from: lon_lat(g.from),
                to: lon_lat(g.to),
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_with_points(id: i64, points: &[(i64, f64, f64)]) -> Db {
        let db = Db::open_in_memory().unwrap();
        let conn = db.connection();
        conn.execute("INSERT INTO hikes (id, distance_m) VALUES (?1, 0)", [id])
            .unwrap();
        for (idx, &(segment_idx, lat, lon)) in points.iter().enumerate() {
            conn.execute(
                "INSERT INTO track_points (hike_id, segment_idx, idx, lat, lon) VALUES (?1, ?2, ?3, ?4, ?5)",
                (id, segment_idx, idx as i64, lat, lon),
            )
            .unwrap();
        }
        db
    }

    #[test]
    fn coordinates_are_sent_lon_first_with_one_gap_between_segments() {
        let db = db_with_points(3, &[(0, 45.0, 6.0), (0, 45.1, 6.1), (1, 45.2, 6.2)]);
        assert_eq!(
            track(&db, 3).unwrap(),
            MapTrack {
                segments: vec![vec![(6.0, 45.0), (6.1, 45.1)], vec![(6.2, 45.2)]],
                gaps: vec![Gap {
                    from: (6.1, 45.1),
                    to: (6.2, 45.2)
                }],
            }
        );
    }

    #[test]
    fn a_hike_without_points_has_no_segments() {
        let db = db_with_points(3, &[]);
        assert_eq!(track(&db, 3).unwrap().segments, Vec::<Vec<LonLat>>::new());
    }

    #[test]
    fn an_unknown_hike_is_an_error() {
        let db = db_with_points(3, &[]);
        assert_eq!(track(&db, 4), Err("no hike with id 4".to_string()));
    }
}
