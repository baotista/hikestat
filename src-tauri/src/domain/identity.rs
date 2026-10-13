use sha2::{Digest, Sha256};

use crate::domain::parser::Trace;

/// SHA-256 (hex) of the trace's points, so the same trace gets the same identity whatever the
/// file it comes from. The name is deliberately left out: renaming a trace must not make it new.
///
/// Canonical form: for each segment, its point count, then each point's lat, lon, elevation and
/// time as fixed-width little-endian values, with a presence byte for the optional ones.
pub fn trace_hash(trace: &Trace) -> String {
    let mut h = Sha256::new();
    h.update((trace.segments.len() as u64).to_le_bytes());
    for seg in &trace.segments {
        h.update((seg.len() as u64).to_le_bytes());
        for p in seg {
            h.update(p.lat.to_bits().to_le_bytes());
            h.update(p.lon.to_bits().to_le_bytes());
            match p.ele {
                Some(e) => {
                    h.update([1]);
                    h.update(e.to_bits().to_le_bytes());
                }
                None => h.update([0]),
            }
            match p.time {
                Some(t) => {
                    h.update([1]);
                    h.update(t.to_le_bytes());
                }
                None => h.update([0]),
            }
        }
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::parser::Point;

    fn pt(lat: f64, time: Option<i64>) -> Point {
        Point {
            lat,
            lon: 3.0,
            ele: Some(30.0),
            time,
        }
    }

    fn trace(name: &str, segments: Vec<Vec<Point>>) -> Trace {
        Trace {
            name: Some(name.into()),
            kind: None,
            segments,
        }
    }

    #[test]
    fn same_points_give_same_hash() {
        let a = trace("a", vec![vec![pt(50.0, Some(1)), pt(50.1, Some(2))]]);
        assert_eq!(trace_hash(&a), trace_hash(&a.clone()));
        assert_eq!(trace_hash(&a).len(), 64);
    }

    #[test]
    fn name_is_not_part_of_the_identity() {
        let pts = vec![vec![pt(50.0, Some(1))]];
        assert_eq!(
            trace_hash(&trace("before", pts.clone())),
            trace_hash(&trace("after", pts))
        );
    }

    #[test]
    fn any_point_change_changes_the_hash() {
        let base = trace("t", vec![vec![pt(50.0, Some(1)), pt(50.1, Some(2))]]);
        let moved = trace("t", vec![vec![pt(50.0, Some(1)), pt(50.2, Some(2))]]);
        let retimed = trace("t", vec![vec![pt(50.0, Some(1)), pt(50.1, Some(3))]]);
        let untimed = trace("t", vec![vec![pt(50.0, Some(1)), pt(50.1, None)]]);
        let h = trace_hash(&base);
        assert_ne!(h, trace_hash(&moved));
        assert_ne!(h, trace_hash(&retimed));
        assert_ne!(h, trace_hash(&untimed));
    }

    #[test]
    fn segment_boundaries_matter() {
        let one = trace("t", vec![vec![pt(50.0, Some(1)), pt(50.1, Some(2))]]);
        let two = trace("t", vec![vec![pt(50.0, Some(1))], vec![pt(50.1, Some(2))]]);
        assert_ne!(trace_hash(&one), trace_hash(&two));
    }
}
