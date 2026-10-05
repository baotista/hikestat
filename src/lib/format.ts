// Formatting helpers. Units are metric, text is English.
// Local dates arrive from Rust as "YYYY-MM-DD" and are formatted as is: no time zone involved.

const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

const kilometres = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });

/** "14.2 km" from metres. */
export function formatDistance(metres: number): string {
  return `${kilometres.format(metres / 1000)} km`;
}

function parts(isoDate: string) {
  const [year, month, day] = isoDate.split("-").map(Number);
  const weekday = new Date(Date.UTC(year, month - 1, day)).getUTCDay();
  return { year, month: MONTHS[month - 1], day, weekday: WEEKDAYS[weekday] };
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
