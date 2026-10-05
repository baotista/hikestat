import { describe, expect, it } from "vitest";
import {
  MISSING,
  effectiveZone,
  formatBarLabel,
  formatBucket,
  formatDateTime,
  formatDay,
  formatDistance,
  formatDuration,
  formatElevation,
  formatLocalDate,
  formatMonth,
  formatRange,
  formatSpeed,
  formatWeekday,
  formatWindow,
  orMissing,
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

// 2026-09-12T06:14:00Z: 08:14 in Paris (CEST), 15:14 in Tokyo.
const SAT_MORNING = Date.UTC(2026, 8, 12, 6, 14) / 1000;

describe("orMissing", () => {
  it("formats a value, zero included", () => {
    expect(orMissing(0, formatDistance)).toBe("0 km");
    expect(orMissing(14_234, formatDistance)).toBe("14.2 km");
  });

  it("shows a dash for null", () => {
    expect(orMissing(null, formatDistance)).toBe(MISSING);
    expect(MISSING).toBe("—");
  });
});

describe("formatElevation", () => {
  it("shows whole metres", () => {
    expect(formatElevation(1234.6)).toBe("1,235 m");
    expect(formatElevation(-3)).toBe("-3 m");
  });

  it("shows zero as 0 m and null as a dash", () => {
    expect(formatElevation(0)).toBe("0 m");
    expect(formatElevation(null)).toBe("—");
  });
});

describe("formatDuration", () => {
  it("shows hours and minutes", () => {
    expect(formatDuration(5 * 3600 + 10 * 60)).toBe("5 h 10 min");
    expect(formatDuration(3600)).toBe("1 h 0 min");
  });

  it("shows minutes alone under an hour", () => {
    expect(formatDuration(45 * 60)).toBe("45 min");
  });

  it("rounds to the nearest minute", () => {
    expect(formatDuration(45 * 60 + 29)).toBe("45 min");
    expect(formatDuration(59 * 60 + 30)).toBe("1 h 0 min");
  });

  it("keeps counting hours past a day", () => {
    expect(formatDuration(26 * 3600 + 5 * 60)).toBe("26 h 5 min");
    expect(formatDuration(100 * 3600)).toBe("100 h 0 min");
  });

  it("shows zero as 0 min and null as a dash", () => {
    expect(formatDuration(0)).toBe("0 min");
    expect(formatDuration(null)).toBe("—");
  });
});

describe("formatSpeed", () => {
  it("converts metres per second to km/h", () => {
    expect(formatSpeed(1.2)).toBe("4.3 km/h");
    expect(formatSpeed(10)).toBe("36 km/h");
  });

  it("shows zero as 0 km/h and null as a dash", () => {
    expect(formatSpeed(0)).toBe("0 km/h");
    expect(formatSpeed(null)).toBe("—");
  });
});

describe("effectiveZone", () => {
  it("keeps a known zone and falls back to UTC otherwise", () => {
    expect(effectiveZone("Asia/Tokyo")).toBe("Asia/Tokyo");
    expect(effectiveZone(null)).toBe("UTC");
    expect(effectiveZone("Not/AZone")).toBe("UTC");
  });
});

describe("formatDateTime", () => {
  it("shows the time in the start time zone, with its name", () => {
    expect(formatDateTime(SAT_MORNING, "Europe/Paris")).toBe("Sat 12 Sep 2026, 08:14 (Europe/Paris)");
    expect(formatDateTime(SAT_MORNING, "Asia/Tokyo")).toBe("Sat 12 Sep 2026, 15:14 (Asia/Tokyo)");
  });

  it("uses a 24-hour clock, midnight included", () => {
    expect(formatDateTime(Date.UTC(2026, 0, 1, 0, 5) / 1000, "UTC")).toBe("Thu 1 Jan 2026, 00:05 (UTC)");
  });

  it("falls back to UTC when the zone is missing or unknown", () => {
    expect(formatDateTime(SAT_MORNING, null)).toBe("Sat 12 Sep 2026, 06:14 (UTC)");
    expect(formatDateTime(SAT_MORNING, "Not/AZone")).toBe("Sat 12 Sep 2026, 06:14 (UTC)");
  });

  it("shows a dash without a start", () => {
    expect(formatDateTime(null, "Europe/Paris")).toBe("—");
  });
});

describe("formatLocalDate", () => {
  it("gives the date in the start time zone, not in UTC", () => {
    // 2026-09-11T22:30Z is already Saturday morning in Tokyo.
    const lateFriday = Date.UTC(2026, 8, 11, 22, 30) / 1000;
    expect(formatLocalDate(lateFriday, "Asia/Tokyo")).toBe("Sat 12 Sep 2026");
    expect(formatLocalDate(lateFriday, null)).toBe("Fri 11 Sep 2026");
  });

  it("shows a dash without a start", () => {
    expect(formatLocalDate(null, null)).toBe("—");
  });
});
