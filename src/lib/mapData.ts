// GeoJSON drawn by the hike map, built from the `track` command's result. Pure functions.

import type { FeatureCollection, LineString, Point } from "geojson";
import type { MapTrack } from "../bindings";

type LonLat = [number, number];

function lines(coordinates: LonLat[][]): FeatureCollection<LineString> {
  return {
    type: "FeatureCollection",
    features: coordinates.map((line) => ({
      type: "Feature",
      properties: {},
      geometry: { type: "LineString", coordinates: line },
    })),
  };
}

/** One line per segment; a segment of a single point is not a line and is left out. */
export function segmentLines(track: MapTrack): FeatureCollection<LineString> {
  return lines(track.segments.filter((segment) => segment.length >= 2));
}

/** One line per trou de signal, from the end of a segment to the start of the next. */
export function gapLines(track: MapTrack): FeatureCollection<LineString> {
  return lines(track.gaps.map((gap) => [gap.from, gap.to]));
}

export type Role = "start" | "end";

/** The start and end markers, the end last so that it is drawn on top on a loop. */
export function endpoints(track: MapTrack): FeatureCollection<Point, { role: Role }> {
  const first = track.segments[0]?.[0];
  const last = track.segments.at(-1)?.at(-1);
  if (first === undefined || last === undefined) return { type: "FeatureCollection", features: [] };
  const marker = (role: Role, coordinates: LonLat) => ({
    type: "Feature" as const,
    properties: { role },
    geometry: { type: "Point" as const, coordinates },
  });
  return { type: "FeatureCollection", features: [marker("start", first), marker("end", last)] };
}

/** `[[west, south], [east, north]]` around every point, or null without points. */
export function trackBounds(track: MapTrack): [LonLat, LonLat] | null {
  const points = track.segments.flat();
  if (points.length === 0) return null;
  const lons = points.map(([lon]) => lon);
  const lats = points.map(([, lat]) => lat);
  return [
    [Math.min(...lons), Math.min(...lats)],
    [Math.max(...lons), Math.max(...lats)],
  ];
}

export function isEmpty(track: MapTrack): boolean {
  return track.segments.length === 0;
}
