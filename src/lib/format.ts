// Formatting helpers. Units are metric, text is English.
// Local dates arrive from Rust as "YYYY-MM-DD" and are formatted as is: no time zone involved.

import type { View } from "../bindings";

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
