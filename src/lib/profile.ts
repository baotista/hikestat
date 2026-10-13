// The elevation profile of a hike, from the `track` command's result. Pure functions.

import type { MapTrack } from "../bindings";

export type ProfilePoint = { distanceM: number; elevationM: number };

/**
 * The stretches drawn as areas: one per run of positions with an elevation, so a series breaks
 * across each trou de signal and wherever elevations are missing. A lone point draws nothing.
 */
export function profileSeries(track: MapTrack): ProfilePoint[][] {
  const series: ProfilePoint[][] = [];
  for (const segment of track.segments) {
    let run: ProfilePoint[] = [];
    segment.elevationsM.forEach((elevationM, i) => {
      if (elevationM === null) {
        if (run.length >= 2) series.push(run);
        run = [];
      } else {
        run.push({ distanceM: segment.distancesM[i], elevationM });
      }
    });
    if (run.length >= 2) series.push(run);
  }
  return series;
}

export type ProfileExtent = { distanceM: number; minElevationM: number; maxElevationM: number };

/** The whole hike's distance and the range of the elevations drawn; null if nothing is drawn. */
export function profileExtent(track: MapTrack, series: ProfilePoint[][]): ProfileExtent | null {
  const elevations = series.flat().map((p) => p.elevationM);
  if (elevations.length === 0) return null;
  return {
    distanceM: track.segments.at(-1)?.distancesM.at(-1) ?? 0,
    minElevationM: Math.min(...elevations),
    maxElevationM: Math.max(...elevations),
  };
}

const STEPS_KM = [0.1, 0.2, 0.5, 1, 2, 5, 10, 20, 50, 100];

/** Distances (m) of the km axis ticks: the smallest round step at least `minGapPx` apart. */
export function kmTicks(distanceM: number, widthPx: number, minGapPx: number): number[] {
  if (distanceM <= 0) return [0];
  const stepKm = STEPS_KM.find((s) => ((s * 1000) / distanceM) * widthPx >= minGapPx) ?? STEPS_KM.at(-1)!;
  const step = stepKm * 1000;
  const ticks: number[] = [];
  for (let i = 0; i * step <= distanceM + 1e-6; i++) ticks.push(Math.round(i * step));
  return ticks;
}

type LonLat = [number, number];

/**
 * Where the hike is at `distanceM` (clamped to the hike), interpolated between the two positions
 * around it; a distance shared by two segments (a trou de signal) is the end of the first. Null
 * without points.
 */
export function positionAt(track: MapTrack, distanceM: number): LonLat | null {
  const segments = track.segments.filter((s) => s.positions.length > 0);
  if (segments.length === 0) return null;
  const first = segments[0];
  if (distanceM <= first.distancesM[0]) return first.positions[0];
  for (const s of segments) {
    const i = s.distancesM.findIndex((d) => d >= distanceM);
    if (i === -1) continue;
    if (i === 0) return s.positions[0];
    const [d0, d1] = [s.distancesM[i - 1], s.distancesM[i]];
    const t = d1 === d0 ? 1 : (distanceM - d0) / (d1 - d0);
    const [a, b] = [s.positions[i - 1], s.positions[i]];
    return [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
  }
  return segments.at(-1)!.positions.at(-1)!;
}

/**
 * The distance (m) of the point of the track nearest to `at`: the nearest point on the segments'
 * lines (never across a trou de signal), in a local plane where a degree of longitude is shrunk
 * by the cosine of the latitude; its distance is interpolated like the positions. Null without
 * points.
 */
export function distanceAt(track: MapTrack, at: LonLat): number | null {
  const shrink = Math.cos((at[1] * Math.PI) / 180);
  const plane = ([lon, lat]: LonLat): LonLat => [lon * shrink, lat];
  const [px, py] = plane(at);
  let bestGap = Infinity;
  let bestDistance: number | null = null;
  const consider = (gap: number, distanceM: number) => {
    if (gap < bestGap) [bestGap, bestDistance] = [gap, distanceM];
  };
  for (const s of track.segments) {
    if (s.positions.length === 1) {
      const [x, y] = plane(s.positions[0]);
      consider(Math.hypot(px - x, py - y), s.distancesM[0]);
    }
    for (let i = 1; i < s.positions.length; i++) {
      const [ax, ay] = plane(s.positions[i - 1]);
      const [bx, by] = plane(s.positions[i]);
      const [dx, dy] = [bx - ax, by - ay];
      const length2 = dx * dx + dy * dy;
      const t = length2 === 0 ? 0 : Math.min(1, Math.max(0, ((px - ax) * dx + (py - ay) * dy) / length2));
      const gap = Math.hypot(px - (ax + t * dx), py - (ay + t * dy));
      consider(gap, s.distancesM[i - 1] + t * (s.distancesM[i] - s.distancesM[i - 1]));
    }
  }
  return bestDistance;
}
