import { describe, expect, it } from "vitest";
import { formatDay, formatDistance, formatRange, formatWeekday } from "./format";

describe("formatDistance", () => {
  it("shows kilometres with at most one decimal", () => {
    expect(formatDistance(14_234)).toBe("14.2 km");
    expect(formatDistance(14_000)).toBe("14 km");
    expect(formatDistance(1_234_567)).toBe("1,234.6 km");
  });

  it("shows zero as 0 km", () => {
    expect(formatDistance(0)).toBe("0 km");
  });
});

describe("formatWeekday", () => {
  it("gives the short English day name of a local date", () => {
    expect(formatWeekday("2026-09-28")).toBe("Mon");
    expect(formatWeekday("2026-10-04")).toBe("Sun");
  });
});

describe("formatDay", () => {
  it("gives the weekday, day, month and year", () => {
    expect(formatDay("2026-09-30")).toBe("Wed 30 Sep 2026");
  });
});

describe("formatRange", () => {
  it("shows the year once when both dates share it", () => {
    expect(formatRange("2026-09-28", "2026-10-04")).toBe("Mon 28 Sep – Sun 4 Oct 2026");
  });

  it("shows both years across a year boundary", () => {
    expect(formatRange("2026-12-28", "2027-01-03")).toBe("Mon 28 Dec 2026 – Sun 3 Jan 2027");
  });
});
