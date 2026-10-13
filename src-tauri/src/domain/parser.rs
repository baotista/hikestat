use std::io::Read;

use crate::domain::Result;

#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub lat: f64,
    pub lon: f64,
    pub ele: Option<f64>,
    /// Seconds since the Unix epoch, UTC.
    pub time: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    pub name: Option<String>,
    /// `<trk><type>` when the file has one; never guessed from the name.
    pub kind: Option<String>,
    pub segments: Vec<Vec<Point>>,
}

/// What a GPX file contains that matters to us, plus a count of what is deliberately ignored.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedGpx {
    pub traces: Vec<Trace>,
    pub ignored_routes: usize,
    pub ignored_waypoints: usize,
}

pub fn parse<R: Read>(reader: R) -> Result<ParsedGpx> {
    let gpx = gpx::read(reader)?;
    let traces = gpx
        .tracks
        .into_iter()
        .map(|trk| Trace {
            name: trk.name,
            kind: trk.type_,
            segments: trk
                .segments
                .into_iter()
                .map(|seg| {
                    seg.points
                        .into_iter()
                        .map(|wpt| {
                            let p = wpt.point();
                            Point {
                                lat: p.y(),
                                lon: p.x(),
                                ele: wpt.elevation,
                                time: wpt
                                    .time
                                    .map(|t| time::OffsetDateTime::from(t).unix_timestamp()),
                            }
                        })
                        .collect()
                })
                .collect(),
        })
        .collect();
    Ok(ParsedGpx {
        traces,
        ignored_routes: gpx.routes.len(),
        ignored_waypoints: gpx.waypoints.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GPX: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx xmlns="http://www.topografix.com/GPX/1.1" version="1.1" creator="test">
  <metadata><time>2024-11-24T09:09:37Z</time></metadata>
  <trk>
    <name>lundi Lunch</name>
    <trkseg>
      <trkpt lat="50.5" lon="3.2"><ele>30.0</ele><time>2024-11-25T11:35:18Z</time></trkpt>
      <trkpt lat="50.6" lon="3.3"><time>2024-11-25T11:35:20Z</time></trkpt>
    </trkseg>
  </trk>
</gpx>"#;

    #[test]
    fn parses_name_points_and_missing_elevation() {
        let traces = parse(GPX.as_bytes()).unwrap().traces;
        assert_eq!(traces.len(), 1);
        let t = &traces[0];
        assert_eq!(t.name.as_deref(), Some("lundi Lunch"));
        assert_eq!(t.segments.len(), 1);
        let pts = &t.segments[0];
        assert_eq!(pts[0].lat, 50.5);
        assert_eq!(pts[0].lon, 3.2);
        assert_eq!(pts[0].ele, Some(30.0));
        assert_eq!(pts[0].time, Some(1_732_534_518));
        assert_eq!(pts[1].ele, None);
    }

    #[test]
    fn invalid_xml_is_an_error() {
        assert!(parse("not xml".as_bytes()).is_err());
    }

    #[test]
    fn counts_ignored_routes_and_waypoints() {
        let gpx = r#"<?xml version="1.0"?>
<gpx xmlns="http://www.topografix.com/GPX/1.1" version="1.1" creator="t">
  <wpt lat="50.0" lon="3.0"/><wpt lat="50.1" lon="3.1"/>
  <rte><rtept lat="50.0" lon="3.0"/><rtept lat="50.1" lon="3.1"/></rte>
</gpx>"#;
        let parsed = parse(gpx.as_bytes()).unwrap();
        assert!(parsed.traces.is_empty());
        assert_eq!((parsed.ignored_routes, parsed.ignored_waypoints), (1, 2));
    }

    #[test]
    fn reads_the_trk_type_when_present() {
        let gpx = r#"<?xml version="1.0"?>
<gpx xmlns="http://www.topografix.com/GPX/1.1" version="1.1" creator="t">
  <trk><name>a</name><type>hiking</type><trkseg><trkpt lat="50.0" lon="3.0"/></trkseg></trk>
  <trk><name>lundi Lunch À pied</name><trkseg><trkpt lat="50.0" lon="3.0"/></trkseg></trk>
</gpx>"#;
        let traces = parse(gpx.as_bytes()).unwrap().traces;
        assert_eq!(traces[0].kind.as_deref(), Some("hiking"));
        assert_eq!(traces[1].kind, None);
    }
}
