import { describe, expect, it } from "vitest";
import type { ImportReport } from "../bindings";
import { importSummary } from "./importText";

const report = (imported: number, alreadyPresent: number, rejected: number): ImportReport => ({
  imported,
  alreadyPresent,
  rejections: Array.from({ length: rejected }, () => ({ file: "a.gpx", traceNumber: null, reason: "bad" })),
  warnings: [],
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
