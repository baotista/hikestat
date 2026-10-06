import { describe, expect, it } from "vitest";
import { prepareStyle } from "./mapStyle";

const fetched = () => ({
  version: 8,
  center: [0, 0],
  zoom: 1,
  bearing: 0,
  pitch: 0,
  sources: {},
  layers: [
    { id: "background", type: "background", paint: { "background-color": "rgb(12,12,12)" } },
    { id: "highway_path", type: "line", paint: { "line-color": "rgb(27,27,29)", "line-dasharray": [1, 1] } },
    { id: "highway_name_other", type: "symbol", paint: { "text-color": "rgba(80,78,78,1)" } },
    { id: "place_village", type: "symbol", paint: { "text-color": "rgb(101,101,101)" } },
    { id: "unknown_layer", type: "line", paint: { "line-color": "#123456" } },
  ],
});

type Layer = { id: string; paint?: Record<string, unknown> };
const paint = (style: { layers: Layer[] }, id: string) => style.layers.find((l) => l.id === id)?.paint;

describe("prepareStyle", () => {
  it("drops the style's own camera, which would undo the framing", () => {
    for (const scheme of ["light", "dark"] as const) {
      const style = prepareStyle(fetched(), scheme);
      expect(Object.keys(style)).not.toContain("center");
      expect(Object.keys(style)).not.toContain("zoom");
      expect(Object.keys(style)).not.toContain("bearing");
      expect(Object.keys(style)).not.toContain("pitch");
    }
  });

  it("leaves the light style's layers as they are", () => {
    expect(prepareStyle(fetched(), "light").layers).toEqual(fetched().layers);
  });

  it("lightens paths, street and place names in the dark style, keeping their other paint", () => {
    const style = prepareStyle(fetched(), "dark") as { layers: Layer[] };
    expect(paint(style, "highway_path")).toEqual({ "line-color": "#8e8e93", "line-dasharray": [1, 1] });
    expect(paint(style, "highway_name_other")?.["text-color"]).toBe("#aeaeb2");
    expect(paint(style, "place_village")?.["text-color"]).toBe("#d1d1d6");
  });

  it("leaves a layer it doesn't know untouched", () => {
    const style = prepareStyle(fetched(), "dark") as { layers: Layer[] };
    expect(paint(style, "unknown_layer")).toEqual({ "line-color": "#123456" });
    expect(paint(style, "background")).toEqual({ "background-color": "rgb(12,12,12)" });
  });
});
