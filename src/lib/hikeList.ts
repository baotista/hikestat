// What the list of hikes shows, in which order. Pure functions; sorting by other columns (#39)
// will add an option here.

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

/** Lower case, accents removed: "Èze" and "eze" compare equal. */
function normalize(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

/** Whether `needle` (normalized) is in the hike's displayed name, GPX name, note or kind. */
function matches(hike: HikeDetails, needle: string): boolean {
  return [hike.title, hike.name, hike.note, hike.kind].some(
    (field) => field !== null && normalize(field).includes(needle),
  );
}

/** `search` narrows the hikes shown (blank: all of them); `total` still counts every hike given. */
export function listHikes(hikes: HikeDetails[], search = ""): HikeListing {
  const needle = normalize(search.trim());
  const found = needle === "" ? hikes : hikes.filter((h) => matches(h, needle));
  const sorted = [...found].sort(byDateDescending);
  return { hikes: sorted, shown: sorted.length, total: hikes.length };
}
