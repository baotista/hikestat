// What the import banner says. Pure functions.

import type { ImportReport } from "../bindings";

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
