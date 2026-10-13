import { describe, expect, it } from "vitest";
import type { MapTrack, Segment } from "../bindings";
import { kmTicks, positionAt, profileExtent, profileSeries } from "./profile";

/** A segment from `[distance, elevation]` pairs; positions don't matter here. */
function seg(points: [number, number | null][]): Segment {
  return {
    positions: points.map(() => [6.0, 45.0]),
    intervals: points.slice(1).map(() => ({ kind: "stopped" })),
    distancesM: points.map(([d]) => d),
    elevationsM: points.map(([, e]) => e),
  };
}

const track = (...segments: Segment[]): MapTrack => ({ segments, gaps: [], speedScale: null, kmMarkers: [] });

describe("profileSeries", () => {
  it("gives one series per segment, breaking across the trou de signal", () => {
    const series = profileSeries(
      track(
        seg([
          [0, 100],
          [500, 120],
        ]),
        seg([
          [500, 300],
          [900, 310],
        ]),
      ),
    );
    expect(series).toEqual([
      [
        { distanceM: 0, elevationM: 100 },
        { distanceM: 500, elevationM: 120 },
      ],
      [
        { distanceM: 500, elevationM: 300 },
        { distanceM: 900, elevationM: 310 },
      ],
    ]);
  });

  it("breaks a series where elevations are missing, and drops lone points", () => {
    const series = profileSeries(
      track(
        seg([
          [0, 100],
          [100, 110],
          [200, null],
          [300, 130],
          [400, null],
          [500, 150],
          [600, 160],
        ]),
      ),
    );
    expect(series.map((s) => s.map((p) => p.distanceM))).toEqual([
      [0, 100],
      [500, 600],
    ]);
  });
});

describe("profileExtent", () => {
  it("spans the whole distance and the elevations drawn", () => {
    const t = track(
      seg([
        [0, 100],
        [500, 80],
        [800, null],
      ]),
      seg([
        [800, 300],
        [1200, 310],
      ]),
    );
    expect(profileExtent(t, profileSeries(t))).toEqual({ distanceM: 1200, minElevationM: 80, maxElevationM: 310 });
  });

  it("is null without elevations", () => {
    const t = track(
      seg([
        [0, null],
        [500, null],
      ]),
    );
    expect(profileExtent(t, profileSeries(t))).toBeNull();
  });
});

describe("kmTicks", () => {
  it("picks the smallest round step that leaves room between labels", () => {
    // 12 km over 300 px with at least 50 px between ticks: 2 km.
    expect(kmTicks(12_000, 300, 50)).toEqual([0, 2000, 4000, 6000, 8000, 10_000, 12_000]);
    // 3.4 km over 300 px: 0.5 km would be 44 px apart, so 1 km; over 400 px, 0.5 km fits.
    expect(kmTicks(3400, 300, 50)).toEqual([0, 1000, 2000, 3000]);
    expect(kmTicks(3400, 400, 50)).toEqual([0, 500, 1000, 1500, 2000, 2500, 3000]);
  });

  it("has only 0 for a hike of no length", () => {
    expect(kmTicks(0, 300, 50)).toEqual([0]);
  });
});

describe("positionAt", () => {
  // Two segments: 0 → 1000 m along a meridian, then (after a trou de signal) 1000 → 1500 m.
  const t: MapTrack = {
    segments: [
      { ...seg([[0, 100], [500, 110], [1000, 120]]), positions: [[6.0, 45.0], [6.0, 45.005], [6.0, 45.01]] },
      { ...seg([[1000, 300], [1500, 310]]), positions: [[6.1, 45.1], [6.1, 45.105]] },
    ],
    gaps: [],
    speedScale: null,
    kmMarkers: [],
  };

  it("interpolates between the two positions around the distance", () => {
    const [lon, lat] = positionAt(t, 250)!;
    expect(lon).toBeCloseTo(6.0);
    expect(lat).toBeCloseTo(45.0025);
  });

  it("lands on a position at its exact distance", () => {
    expect(positionAt(t, 500)).toEqual([6.0, 45.005]);
  });

  it("puts a distance shared by two segments at the end of the first", () => {
    expect(positionAt(t, 1000)).toEqual([6.0, 45.01]);
  });

  it("follows the next segment after the trou de signal", () => {
    const [lon, lat] = positionAt(t, 1250)!;
    expect(lon).toBeCloseTo(6.1);
    expect(lat).toBeCloseTo(45.1025);
  });

  it("clamps outside the hike", () => {
    expect(positionAt(t, -10)).toEqual([6.0, 45.0]);
    expect(positionAt(t, 9999)).toEqual([6.1, 45.105]);
  });

  it("is null without points", () => {
    expect(positionAt(track(), 10)).toBeNull();
  });
});
