import { describe, expect, it } from "vitest";
import type { Interval, MapTrack, Segment } from "../bindings";
import {
  type TrackColors,
  endpoints,
  gapLines,
  isEmpty,
  mixColor,
  segmentLines,
  speedFraction,
  speedLines,
  trackBounds,
} from "./mapData";

type LonLat = [number, number];

/** A segment whose intervalles are all stopped unless given. */
function seg(positions: LonLat[], intervals?: Interval[]): Segment {
  return { positions, intervals: intervals ?? positions.slice(1).map(() => ({ kind: "stopped" })) };
}

const twoSegments: MapTrack = {
  segments: [
    seg([
      [6.0, 45.0],
      [6.1, 45.1],
    ]),
    seg([
      [6.3, 45.2],
      [6.2, 44.9],
    ]),
  ],
  gaps: [{ from: [6.1, 45.1], to: [6.3, 45.2] }],
  speedScale: null,
};

const empty: MapTrack = { segments: [], gaps: [], speedScale: null };

const colors: TrackColors = {
  accent: "#0a7c42",
  stopped: "#888888",
  speed: ["#000000", "#ffffff"],
};

describe("segmentLines", () => {
  it("draws each segment as its own line", () => {
    expect(segmentLines(twoSegments).features.map((f) => f.geometry.coordinates)).toEqual(
      twoSegments.segments.map((s) => s.positions),
    );
  });

  it("leaves out a segment of a single point, which is not a line", () => {
    const track: MapTrack = { ...twoSegments, segments: [seg([[6.0, 45.0]]), ...twoSegments.segments] };
    expect(segmentLines(track).features).toHaveLength(2);
  });
});

describe("gapLines", () => {
  it("draws one line per trou de signal", () => {
    expect(gapLines(twoSegments).features.map((f) => f.geometry.coordinates)).toEqual([
      [
        [6.1, 45.1],
        [6.3, 45.2],
      ],
    ]);
    expect(gapLines(empty).features).toEqual([]);
  });
});

describe("endpoints", () => {
  it("puts the start on the first point and the end on the last one, end last", () => {
    expect(endpoints(twoSegments).features.map((f) => [f.properties.role, f.geometry.coordinates])).toEqual([
      ["start", [6.0, 45.0]],
      ["end", [6.2, 44.9]],
    ]);
  });

  it("keeps both markers on a hike of a single point", () => {
    const track: MapTrack = { ...empty, segments: [seg([[6.0, 45.0]])] };
    expect(endpoints(track).features.map((f) => f.properties.role)).toEqual(["start", "end"]);
  });

  it("has no marker without points", () => {
    expect(endpoints(empty).features).toEqual([]);
  });
});

describe("trackBounds", () => {
  it("spans every point, south-west then north-east", () => {
    expect(trackBounds(twoSegments)).toEqual([
      [6.0, 44.9],
      [6.3, 45.2],
    ]);
  });

  it("is null without points", () => {
    expect(trackBounds(empty)).toBeNull();
  });
});

describe("isEmpty", () => {
  it("tells a hike without points", () => {
    expect(isEmpty(empty)).toBe(true);
    expect(isEmpty(twoSegments)).toBe(false);
  });
});

describe("speedFraction", () => {
  const scale = { lowMps: 1, highMps: 3 };

  it("places a speed between the bounds, clamped outside them", () => {
    expect(speedFraction(2, scale)).toBe(0.5);
    expect(speedFraction(0.5, scale)).toBe(0);
    expect(speedFraction(9, scale)).toBe(1);
  });

  it("is the middle when both bounds are equal", () => {
    expect(speedFraction(2, { lowMps: 2, highMps: 2 })).toBe(0.5);
  });
});

describe("mixColor", () => {
  it("gives the end stops at 0 and 1 and blends in between", () => {
    const stops = ["#000000", "#ff0000", "#ffffff"];
    expect(mixColor(stops, 0)).toBe("#000000");
    expect(mixColor(stops, 0.5)).toBe("#ff0000");
    expect(mixColor(stops, 1)).toBe("#ffffff");
    expect(mixColor(stops, 0.25)).toBe("#800000");
  });
});

describe("speedLines", () => {
  const coloured: MapTrack = {
    segments: [
      seg(
        [
          [6.0, 45.0],
          [6.1, 45.0],
          [6.2, 45.0],
          [6.3, 45.0],
          [6.4, 45.0],
        ],
        [
          { kind: "moving", speedMps: 1 },
          { kind: "moving", speedMps: 3 },
          { kind: "stopped" },
          { kind: "untimed" },
        ],
      ),
    ],
    gaps: [],
    speedScale: { lowMps: 1, highMps: 3 },
  };

  it("draws each intervalle in the colour of its speed, stopped and untimed in grey", () => {
    const lines = speedLines(coloured, colors).features;
    expect(lines.map((f) => f.properties.color)).toEqual(["#000000", "#ffffff", "#888888", "#888888"]);
    expect(lines[1].geometry.coordinates).toEqual([
      [6.1, 45.0],
      [6.2, 45.0],
    ]);
  });

  it("draws whole segments in the accent colour when nothing moves", () => {
    const lines = speedLines(twoSegments, colors).features;
    expect(lines.map((f) => f.properties.color)).toEqual(["#0a7c42", "#0a7c42"]);
    expect(lines[0].geometry.coordinates).toEqual(twoSegments.segments[0].positions);
  });
});
