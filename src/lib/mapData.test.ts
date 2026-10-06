import { describe, expect, it } from "vitest";
import type { MapTrack } from "../bindings";
import { endpoints, gapLines, isEmpty, segmentLines, trackBounds } from "./mapData";

const twoSegments: MapTrack = {
  segments: [
    [
      [6.0, 45.0],
      [6.1, 45.1],
    ],
    [
      [6.3, 45.2],
      [6.2, 44.9],
    ],
  ],
  gaps: [{ from: [6.1, 45.1], to: [6.3, 45.2] }],
};

const empty: MapTrack = { segments: [], gaps: [] };

describe("segmentLines", () => {
  it("draws each segment as its own line", () => {
    expect(segmentLines(twoSegments).features.map((f) => f.geometry.coordinates)).toEqual(twoSegments.segments);
  });

  it("leaves out a segment of a single point, which is not a line", () => {
    const track: MapTrack = { segments: [[[6.0, 45.0]], ...twoSegments.segments], gaps: [] };
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
    const track: MapTrack = { segments: [[[6.0, 45.0]]], gaps: [] };
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
