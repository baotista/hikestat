// GeoJSON drawn by the hike map, built from the `track` command's result. Pure functions.

import type { Feature, FeatureCollection, LineString, Point } from "geojson";
import type { Interval, MapTrack, SpeedScale } from "../bindings";

type LonLat = [number, number];

function line<P>(coordinates: LonLat[], properties: P): Feature<LineString, P> {
  return { type: "Feature", properties, geometry: { type: "LineString", coordinates } };
}

function lines(coordinates: LonLat[][]): FeatureCollection<LineString> {
  return { type: "FeatureCollection", features: coordinates.map((c) => line(c, {})) };
}

/** One line per segment; a segment of a single point is not a line and is left out. */
export function segmentLines(track: MapTrack): FeatureCollection<LineString> {
  return lines(track.segments.map((s) => s.positions).filter((positions) => positions.length >= 2));
}

/** One line per trou de signal, from the end of a segment to the start of the next. */
export function gapLines(track: MapTrack): FeatureCollection<LineString> {
  return lines(track.gaps.map((gap) => [gap.from, gap.to]));
}

export type Role = "start" | "end";

/** The start and end markers, the end last so that it is drawn on top on a loop. */
export function endpoints(track: MapTrack): FeatureCollection<Point, { role: Role }> {
  const first = track.segments[0]?.positions[0];
  const last = track.segments.at(-1)?.positions.at(-1);
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
  const points = track.segments.flatMap((s) => s.positions);
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

// --- Speed colours -------------------------------------------------------------------------------

/** Where `speed` sits between the scale's bounds, from 0 to 1 (clamped); the middle if they meet. */
export function speedFraction(speed: number, scale: SpeedScale): number {
  const span = scale.highMps - scale.lowMps;
  if (span <= 0) return 0.5;
  return Math.min(1, Math.max(0, (speed - scale.lowMps) / span));
}

const channels = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));

/** The colour at `t` (0 to 1) along evenly spaced `#rrggbb` stops. */
export function mixColor(stops: string[], t: number): string {
  const at = t * (stops.length - 1);
  const i = Math.min(Math.floor(at), stops.length - 2);
  const [from, to] = [channels(stops[i]), channels(stops[i + 1])];
  const mixed = from.map((c, k) => Math.round(c + (to[k] - c) * (at - i)));
  return `#${mixed.map((c) => c.toString(16).padStart(2, "0")).join("")}`;
}

export type TrackColors = {
  /** The whole track when nothing moves. */
  accent: string;
  /** Stopped and untimed intervalles. */
  stopped: string;
  /** From slow to fast. */
  speed: string[];
};

function intervalColor(interval: Interval, scale: SpeedScale, colors: TrackColors): string {
  if (interval.kind !== "moving") return colors.stopped;
  return mixColor(colors.speed, speedFraction(interval.speedMps, scale));
}

/**
 * The coloured track: one line per intervalle in the colour of its smoothed speed (grey when
 * stopped or untimed), or whole segments in the accent colour when nothing moves.
 */
export function speedLines(track: MapTrack, colors: TrackColors): FeatureCollection<LineString, { color: string }> {
  const scale = track.speedScale;
  const features =
    scale === null
      ? segmentLines(track).features.map((f) => line(f.geometry.coordinates as LonLat[], { color: colors.accent }))
      : track.segments.flatMap((s) =>
          s.intervals.map((interval, i) =>
            line([s.positions[i], s.positions[i + 1]], { color: intervalColor(interval, scale, colors) }),
          ),
        );
  return { type: "FeatureCollection", features };
}
