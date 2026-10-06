// What the list of hikes shows, in which order. Pure functions; sorting by other columns (#39)
// and searching (#40) will add options here.

import type { HikeDetails } from "../bindings";

export type HikeListing = {
  hikes: HikeDetails[];
  /** How many hikes are shown. */
  shown: number;
  /** How many hikes the list was given (the window's or the bar's). */
  total: number;
};

/** Most recent start first; hikes without a start last; ties by id, for a stable order. */
function byDateDescending(a: HikeDetails, b: HikeDetails): number {
  if (a.startedAt !== b.startedAt) {
    if (a.startedAt === null) return 1;
    if (b.startedAt === null) return -1;
    return b.startedAt - a.startedAt;
  }
  return a.id - b.id;
}

export function listHikes(hikes: HikeDetails[]): HikeListing {
  const sorted = [...hikes].sort(byDateDescending);
  return { hikes: sorted, shown: sorted.length, total: hikes.length };
}
