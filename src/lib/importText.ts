// What the import banner says. Pure functions.

import type { ImportReport, KindFilter } from "../bindings";

/** "3 imported · 1 already present · 1 rejected", without the counts of zero. */
export function importSummary(report: ImportReport): string {
  const parts = [
    [report.imported, "imported"],
    [report.alreadyPresent, "already present"],
    [report.rejections.length, "rejected"],
  ] as const;
  const shown = parts.filter(([count]) => count > 0).map(([count, what]) => `${count} ${what}`);
  return shown.length > 0 ? shown.join(" · ") : "Nothing to import";
}

const baseName = (path: string) => path.split(/[\\/]/).pop() || path;

/**
 * The "Details" lines: each rejection ("walk.gpx, trace 2: reason", or "walk.gpx: reason" for a
 * whole file), then each warning as the import wrote it.
 */
export function importDetails(report: ImportReport): string[] {
  const rejections = report.rejections.map((r) => {
    const where = r.traceNumber === null ? baseName(r.file) : `${baseName(r.file)}, trace ${r.traceNumber}`;
    return `${where}: ${r.reason}`;
  });
  return [...rejections, ...report.warnings];
}

/** "1 hidden by the Cycling filter" when the dashboard's kind filter hides imported hikes, else "". */
export function hiddenByFilter(report: ImportReport, filter: KindFilter): string {
  if (report.hiddenByFilter === 0) return "";
  const name = filter.filter === "kind" ? filter.kind : filter.filter === "notSet" ? "Not set" : "All";
  return `${report.hiddenByFilter} hidden by the ${name} filter`;
}
