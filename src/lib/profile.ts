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
