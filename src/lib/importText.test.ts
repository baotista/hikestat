import { describe, expect, it } from "vitest";
import type { ImportReport } from "../bindings";
import { hiddenByFilter, importDetails, importSummary } from "./importText";

const report = (imported: number, alreadyPresent: number, rejected: number): ImportReport => ({
  imported,
  alreadyPresent,
  rejections: Array.from({ length: rejected }, () => ({ file: "a.gpx", traceNumber: null, reason: "bad" })),
  warnings: [],
  anchor: null,
  hiddenByFilter: 0,
});

describe("importSummary", () => {
  it("lists the three counts", () => {
    expect(importSummary(report(3, 1, 1))).toBe("3 imported · 1 already present · 1 rejected");
  });

  it("leaves out counts of zero", () => {
    expect(importSummary(report(2, 0, 0))).toBe("2 imported");
    expect(importSummary(report(0, 4, 0))).toBe("4 already present");
    expect(importSummary(report(0, 0, 2))).toBe("2 rejected");
  });

  it("says when there was nothing to import", () => {
    expect(importSummary(report(0, 0, 0))).toBe("Nothing to import");
  });
});

describe("importDetails", () => {
  it("names the file and the trace of a rejected trace", () => {
    const details = importDetails({
      ...report(0, 0, 0),
      rejections: [{ file: "/Users/me/hikes/walk.gpx", traceNumber: 2, reason: "no point has a timestamp" }],
    });
    expect(details).toEqual(["walk.gpx, trace 2: no point has a timestamp"]);
  });

  it("names only the file when the whole file was rejected", () => {
    const details = importDetails({
      ...report(0, 0, 0),
      rejections: [
        { file: "/Users/me/hikes/broken.gpx", traceNumber: null, reason: "unexpected end of file" },
        { file: "/Users/me/photo.jpg", traceNumber: null, reason: "not a .gpx file" },
      ],
    });
    expect(details).toEqual(["broken.gpx: unexpected end of file", "photo.jpg: not a .gpx file"]);
  });

  it("lists the warnings after the rejections, as they are", () => {
    const details = importDetails({
      ...report(1, 0, 0),
      rejections: [{ file: "a.gpx", traceNumber: null, reason: "bad" }],
      warnings: ["/Users/me/hikes/walk.gpx: ignored 2 waypoint(s)"],
    });
    expect(details).toEqual(["a.gpx: bad", "/Users/me/hikes/walk.gpx: ignored 2 waypoint(s)"]);
  });

  it("is empty without rejections or warnings", () => {
    expect(importDetails(report(3, 1, 0))).toEqual([]);
  });
});

describe("hiddenByFilter", () => {
  const hidden = (n: number) => ({ ...report(n, 0, 0), hiddenByFilter: n });

  it("names the kind filter that hides imported hikes", () => {
    expect(hiddenByFilter(hidden(1), { filter: "kind", kind: "Cycling" })).toBe("1 hidden by the Cycling filter");
    expect(hiddenByFilter(hidden(2), { filter: "notSet" })).toBe("2 hidden by the Not set filter");
  });

  it("is empty when the filter hides nothing", () => {
    expect(hiddenByFilter(report(3, 0, 0), { filter: "kind", kind: "Cycling" })).toBe("");
  });
});
