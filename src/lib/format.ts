// Formatting helpers. Units are metric, text is English.
// Local dates arrive from Rust as "YYYY-MM-DD" and are formatted as is: no time zone involved.

import type { Metrics, View } from "../bindings";

const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MONTH_NAMES = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

const kilometres = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });

/** "14.2 km" from metres. */
export function formatDistance(metres: number): string {
  return `${kilometres.format(metres / 1000)} km`;
}

function parts(isoDate: string) {
  const [year, month, day] = isoDate.split("-").map(Number);
  const weekday = new Date(Date.UTC(year, month - 1, day)).getUTCDay();
  return { year, month: MONTHS[month - 1], monthName: MONTH_NAMES[month - 1], day, weekday: WEEKDAYS[weekday] };
}

/** "Mon" from "2026-09-28". */
export function formatWeekday(isoDate: string): string {
  return parts(isoDate).weekday;
}

/** "Wed 30 Sep 2026" from "2026-09-30". */
export function formatDay(isoDate: string): string {
  const d = parts(isoDate);
  return `${d.weekday} ${d.day} ${d.month} ${d.year}`;
}

/** "Mon 28 Sep – Sun 4 Oct 2026"; both years when they differ. */
export function formatRange(first: string, last: string): string {
  const a = parts(first);
  const b = parts(last);
  const start = `${a.weekday} ${a.day} ${a.month}` + (a.year === b.year ? "" : ` ${a.year}`);
  return `${start} – ${formatDay(last)}`;
}

/** "September 2026" from any date of the month. */
export function formatMonth(isoDate: string): string {
  const d = parts(isoDate);
  return `${d.monthName} ${d.year}`;
}

/** The dashboard window: "Mon 28 Sep – Sun 4 Oct 2026", "September 2026", "2026", "2019 – 2026". */
export function formatWindow(view: View, first: string, last: string): string {
  switch (view) {
    case "week":
      return formatRange(first, last);
    case "month":
      return formatMonth(first);
    case "year":
      return String(parts(first).year);
    case "all": {
      const [a, b] = [parts(first).year, parts(last).year];
      return a === b ? String(a) : `${a} – ${b}`;
    }
  }
}

/** Axis label of the bar starting on `start`; month days are labelled on the 1st and every fifth day. */
export function formatBarLabel(view: View, start: string): string {
  const d = parts(start);
  switch (view) {
    case "week":
      return d.weekday;
    case "month":
      return d.day === 1 || d.day % 5 === 0 ? String(d.day) : "";
    case "year":
      return d.monthName[0];
    case "all":
      return String(d.year);
  }
}

/** Name of the bucket starting on `start`: a day, a month ("September 2026") or a year. */
export function formatBucket(view: View, start: string): string {
  switch (view) {
    case "week":
    case "month":
      return formatDay(start);
    case "year":
      return formatMonth(start);
    case "all":
      return String(parts(start).year);
  }
}

// Hike list and panel. A missing (NULL) value is shown as a dash, never as zero.

export const MISSING = "—";

/** `format(value)`, or "—" when the value is missing. */
export function orMissing<T>(value: T | null | undefined, format: (value: T) => string): string {
  return value === null || value === undefined ? MISSING : format(value);
}

const metres = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });
const speed = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });

/** "1,234 m" from metres (elevation, ascent, descent); "—" when missing. */
export function formatElevation(m: number | null): string {
  return orMissing(m, (v) => `${metres.format(v)} m`);
}

/** "5 h 10 min", "45 min", "0 min" from seconds, rounded to the minute; hours keep counting past
 * 24 ("26 h 5 min"). "—" when missing. */
export function formatDuration(seconds: number | null): string {
  return orMissing(seconds, (s) => {
    const minutes = Math.round(s / 60);
    const h = Math.floor(minutes / 60);
    const min = minutes % 60;
    return h === 0 ? `${min} min` : `${h} h ${min} min`;
  });
}

/** "4.3 km/h" from metres per second; "—" when missing. */
export function formatSpeed(mps: number | null): string {
  return orMissing(mps, (v) => `${speed.format(v * 3.6)} km/h`);
}

/** The zone to show times in: `zone` if it is a known IANA name, otherwise UTC. */
export function effectiveZone(zone: string | null): string {
  if (zone === null) return "UTC";
  try {
    new Intl.DateTimeFormat("en-US", { timeZone: zone });
    return zone;
  } catch {
    return "UTC";
  }
}

/** Wall-clock date ("YYYY-MM-DD") and time ("HH:MM") of an instant in an IANA zone. */
function wallClock(epochSeconds: number, zone: string) {
  const fields = new Intl.DateTimeFormat("en-US", {
    timeZone: zone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hourCycle: "h23",
  }).formatToParts(new Date(epochSeconds * 1000));
  const get = (type: string) => fields.find((p) => p.type === type)?.value ?? "";
  return { date: `${get("year")}-${get("month")}-${get("day")}`, time: `${get("hour")}:${get("minute")}` };
}

/** "Sat 12 Sep 2026": the local date of an instant in `zone` (UTC if missing or unknown). */
export function formatLocalDate(epochSeconds: number | null, zone: string | null): string {
  return orMissing(epochSeconds, (s) => formatDay(wallClock(s, effectiveZone(zone)).date));
}

/** "Sat 12 Sep 2026, 08:14 (Europe/Paris)": an instant in `zone`, named; UTC if the zone is
 * missing or unknown. */
export function formatDateTime(epochSeconds: number | null, zone: string | null): string {
  return orMissing(epochSeconds, (s) => {
    const z = effectiveZone(zone);
    const { date, time } = wallClock(s, z);
    return `${formatDay(date)}, ${time} (${z})`;
  });
}

/** The name a hike is shown under: its title if set, otherwise its raw GPX name, otherwise
 * "Untitled". A blank value counts as unset. */
export function displayName(hike: { title: string | null; name: string | null }): string {
  return [hike.title, hike.name].map((v) => v?.trim()).find((v) => v) ?? "Untitled";
}

// Dashboard metrics. Bars and ticks are in the metric's chart unit: km, m, hours, hikes.

const plain = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });

/** The metric charted by the dashboard and summed in its header. */
export type Metric = "distance" | "ascent" | "moving" | "count";

/** Lower-case name of a metric: "distance", "ascent", "moving time", "hikes". */
export function metricName(metric: Metric): string {
  return { distance: "distance", ascent: "ascent", moving: "moving time", count: "hikes" }[metric];
}

/** The metric's value in its chart unit (km, m, hours or a number of hikes); a missing value is 0. */
export function metricValue(metric: Metric, m: Metrics): number {
  switch (metric) {
    case "distance":
      return (m.distanceM ?? 0) / 1000;
    case "ascent":
      return m.ascentM ?? 0;
    case "moving":
      return m.movingS / 3600;
    case "count":
      return m.hikeCount;
  }
}

/** "1 hike", "3 hikes". */
export function formatHikeCount(n: number): string {
  return `${n} ${n === 1 ? "hike" : "hikes"}`;
}

/** A value in the metric's chart unit, with its unit: "14.2 km", "1,235 m", "5 h 10 min", "3 hikes". */
export function formatMetric(metric: Metric, value: number): string {
  switch (metric) {
    case "distance":
      return formatDistance(value * 1000);
    case "ascent":
      return formatElevation(value);
    case "moving":
      return formatDuration(value * 3600);
    case "count":
      return formatHikeCount(value);
  }
}

/** Smallest grid step in the metric's chart unit: whole hours and whole hikes. */
export function metricMinStep(metric: Metric): number {
  return metric === "moving" || metric === "count" ? 1 : 0;
}

/** A compact grid label in the metric's chart unit: "10 km", "500 m", "2 h", "3". */
export function formatMetricTick(metric: Metric, value: number): string {
  switch (metric) {
    case "distance":
      return formatDistance(value * 1000);
    case "ascent":
      return formatElevation(value);
    case "moving":
      return `${plain.format(value)} h`;
    case "count":
      return plain.format(value);
  }
}

/** "1 hike without elevation", "3 hikes without elevation"; `null` when there is none. */
export function formatWithoutElevation(n: number): string | null {
  return n === 0 ? null : `${formatHikeCount(n)} without elevation`;
}
