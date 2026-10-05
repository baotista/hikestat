import { describe, expect, it } from "vitest";
import {
  formatBarLabel,
  formatBucket,
  formatDay,
  formatDistance,
  formatMonth,
  formatRange,
  formatWeekday,
  formatWindow,
} from "./format";

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

describe("formatMonth", () => {
  it("gives the full month name and the year", () => {
    expect(formatMonth("2026-09-01")).toBe("September 2026");
    expect(formatMonth("2028-02-29")).toBe("February 2028");
  });
});

describe("formatWindow", () => {
  it("shows a week as its date range", () => {
    expect(formatWindow("week", "2026-09-28", "2026-10-04")).toBe("Mon 28 Sep – Sun 4 Oct 2026");
  });

  it("shows a month as its name and year", () => {
    expect(formatWindow("month", "2026-09-01", "2026-09-30")).toBe("September 2026");
  });

  it("shows a year as the year", () => {
    expect(formatWindow("year", "2026-01-01", "2026-12-31")).toBe("2026");
  });

  it("shows All as a span of years, or one year", () => {
    expect(formatWindow("all", "2019-01-01", "2026-12-31")).toBe("2019 – 2026");
    expect(formatWindow("all", "2026-01-01", "2026-12-31")).toBe("2026");
  });
});

describe("formatBarLabel", () => {
  it("labels week bars with the weekday", () => {
    expect(formatBarLabel("week", "2026-09-28")).toBe("Mon");
  });

  it("labels month bars with the day number, on the 1st and every fifth day only", () => {
    const labels = Array.from({ length: 31 }, (_, i) =>
      formatBarLabel("month", `2026-01-${String(i + 1).padStart(2, "0")}`),
    ).filter((l) => l !== "");
    expect(labels).toEqual(["1", "5", "10", "15", "20", "25", "30"]);
  });

  it("labels year bars with the month initial", () => {
    const labels = Array.from({ length: 12 }, (_, i) =>
      formatBarLabel("year", `2026-${String(i + 1).padStart(2, "0")}-01`),
    );
    expect(labels.join("")).toBe("JFMAMJJASOND");
  });

  it("labels All bars with the year", () => {
    expect(formatBarLabel("all", "2019-01-01")).toBe("2019");
  });
});

describe("formatBucket", () => {
  it("names a day in the week and month views", () => {
    expect(formatBucket("week", "2026-09-30")).toBe("Wed 30 Sep 2026");
    expect(formatBucket("month", "2026-09-30")).toBe("Wed 30 Sep 2026");
  });

  it("names a month in the year view and a year in the All view", () => {
    expect(formatBucket("year", "2026-09-01")).toBe("September 2026");
    expect(formatBucket("all", "2026-01-01")).toBe("2026");
  });
});
