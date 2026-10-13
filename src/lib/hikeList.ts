// What the list of hikes shows, in which order. Pure functions.

import type { HikeDetails } from "../bindings";
import { displayName } from "./format";

export type HikeListing = {
  hikes: HikeDetails[];
  /** How many hikes are shown. */
  shown: number;
  /** How many hikes the list was given (the window's or the bar's). */
  total: number;
};

export type SortColumn = "date" | "name" | "distance" | "ascent" | "movingTime" | "avgSpeed";

export type HikeSort = { column: SortColumn; descending: boolean };

export const DEFAULT_SORT: HikeSort = { column: "date", descending: true };

/** The order a column starts in when clicked: names A to Z, the rest biggest or latest first. */
const STARTS_DESCENDING: Record<SortColumn, boolean> = {
  date: true,
  name: false,
  distance: true,
  ascent: true,
  movingTime: true,
  avgSpeed: true,
};

/** The sort after clicking `column`'s header: reversed if it is the sorted one. */
export function nextSort(current: HikeSort, column: SortColumn): HikeSort {
  if (current.column === column) return { column, descending: !current.descending };
  return { column, descending: STARTS_DESCENDING[column] };
}

const names = new Intl.Collator(undefined, { sensitivity: "base", numeric: true });

/** The value a column sorts on; `null` when the hike has none (always listed last). */
function value(column: SortColumn, hike: HikeDetails): number | string | null {
  switch (column) {
    case "date":
      return hike.startedAt;
    case "name":
      return displayName(hike);
    case "distance":
      return hike.distanceM;
    case "ascent":
      return hike.ascentM;
    case "movingTime":
      return hike.durationMovingS;
    case "avgSpeed":
      return hike.avgSpeedMps;
  }
}

function compareValues(a: number | string, b: number | string): number {
  return typeof a === "string" && typeof b === "string" ? names.compare(a, b) : Number(a) - Number(b);
}

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

/** Missing values last in both directions; ties by date (most recent first), then id. */
function comparator(sort: HikeSort): (a: HikeDetails, b: HikeDetails) => number {
  return (a, b) => {
    const [va, vb] = [value(sort.column, a), value(sort.column, b)];
    if (va === null || vb === null) {
      if (va !== vb) return va === null ? 1 : -1;
    } else {
      const order = compareValues(va, vb);
      if (order !== 0) return sort.descending ? -order : order;
    }
    return byDateDescending(a, b);
  };
}

/**
 * `search` narrows the hikes shown (blank: all of them), `sort` orders them; `total` still counts
 * every hike given.
 */
export function listHikes(
  hikes: HikeDetails[],
  { search = "", sort = DEFAULT_SORT }: { search?: string; sort?: HikeSort } = {},
): HikeListing {
  const needle = normalize(search.trim());
  const found = needle === "" ? hikes : hikes.filter((h) => matches(h, needle));
  const sorted = [...found].sort(comparator(sort));
  return { hikes: sorted, shown: sorted.length, total: hikes.length };
}
