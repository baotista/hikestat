import { describe, expect, it } from "vitest";
import type { HikeDetails } from "../bindings";
import { DEFAULT_SORT, type HikeSort, listHikes, nextSort } from "./hikeList";

function hike(id: number, startedAt: number | null, fields: Partial<HikeDetails> = {}): HikeDetails {
  return {
    id,
    name: `h${id}`,
    title: null,
    note: null,
    kind: null,
    startedAt,
    startTz: null,
    distanceM: 1000,
    ascentM: null,
    descentM: null,
    minEleM: null,
    maxEleM: null,
    durationTotalS: null,
    durationMovingS: null,
    avgSpeedMps: null,
    ...fields,
  };
}

const ids = (hikes: HikeDetails[], sort?: HikeSort) => listHikes(hikes, { sort }).hikes.map((h) => h.id);

describe("listHikes", () => {
  it("puts the most recent hike first", () => {
    expect(ids([hike(1, 100), hike(2, 300), hike(3, 200)])).toEqual([2, 3, 1]);
  });

  it("breaks ties by id, for a stable order", () => {
    expect(ids([hike(5, 100), hike(2, 100), hike(9, 100)])).toEqual([2, 5, 9]);
  });

  it("puts hikes without a start last", () => {
    expect(ids([hike(1, null), hike(2, 100), hike(3, null), hike(4, 200)])).toEqual([4, 2, 1, 3]);
  });

  it("counts the hikes it lists", () => {
    const list = listHikes([hike(1, 100), hike(2, 200)]);
    expect([list.shown, list.total]).toEqual([2, 2]);
  });

  it("leaves its input untouched", () => {
    const input = [hike(1, 100), hike(2, 200)];
    listHikes(input);
    expect(input.map((h) => h.id)).toEqual([1, 2]);
  });

  it("sorts by date, most recent first, by default", () => {
    expect(DEFAULT_SORT).toEqual({ column: "date", descending: true });
  });
});

describe("listHikes with a sort", () => {
  const asc = (column: HikeSort["column"]): HikeSort => ({ column, descending: false });
  const desc = (column: HikeSort["column"]): HikeSort => ({ column, descending: true });

  it("sorts by date both ways, hikes without a start last", () => {
    const hikes = [hike(1, 200), hike(2, null), hike(3, 100), hike(4, 300)];
    expect(ids(hikes, asc("date"))).toEqual([3, 1, 4, 2]);
    expect(ids(hikes, desc("date"))).toEqual([4, 1, 3, 2]);
  });

  it("sorts by displayed name, locale-aware (accents, case)", () => {
    const hikes = [
      hike(1, 100, { name: "zinal" }),
      hike(2, 100, { name: "raw", title: "Èze" }),
      hike(3, 100, { name: "Arolla" }),
      hike(4, 100, { name: "évolène" }),
    ];
    // Arolla, évolène, Èze (the title wins over the GPX name), zinal.
    expect(ids(hikes, asc("name"))).toEqual([3, 4, 2, 1]);
    expect(ids(hikes, desc("name"))).toEqual([1, 2, 4, 3]);
  });

  it("sorts by distance both ways", () => {
    const hikes = [
      hike(1, 100, { distanceM: 5000 }),
      hike(2, 100, { distanceM: 12000 }),
      hike(3, 100, { distanceM: 800 }),
    ];
    expect(ids(hikes, asc("distance"))).toEqual([3, 1, 2]);
    expect(ids(hikes, desc("distance"))).toEqual([2, 1, 3]);
  });

  it("sorts by ascent both ways, hikes without elevation last", () => {
    const hikes = [
      hike(1, 100, { ascentM: 300 }),
      hike(2, 100),
      hike(3, 100, { ascentM: 50 }),
      hike(4, 100, { ascentM: 900 }),
    ];
    expect(ids(hikes, asc("ascent"))).toEqual([3, 1, 4, 2]);
    expect(ids(hikes, desc("ascent"))).toEqual([4, 1, 3, 2]);
  });

  it("sorts by moving time both ways, untimed hikes last", () => {
    const hikes = [hike(1, 100, { durationMovingS: 3600 }), hike(2, 100), hike(3, 100, { durationMovingS: 600 })];
    expect(ids(hikes, asc("movingTime"))).toEqual([3, 1, 2]);
    expect(ids(hikes, desc("movingTime"))).toEqual([1, 3, 2]);
  });

  it("sorts by average speed both ways, hikes without speed last", () => {
    const hikes = [hike(1, 100, { avgSpeedMps: 1.2 }), hike(2, 100), hike(3, 100, { avgSpeedMps: 1.5 })];
    expect(ids(hikes, asc("avgSpeed"))).toEqual([1, 3, 2]);
    expect(ids(hikes, desc("avgSpeed"))).toEqual([3, 1, 2]);
  });

  it("breaks ties by date (most recent first), then id", () => {
    const hikes = [hike(5, 100), hike(2, 300), hike(9, 100), hike(1, null)];
    expect(ids(hikes, asc("distance"))).toEqual([2, 5, 9, 1]);
    expect(ids(hikes, desc("distance"))).toEqual([2, 5, 9, 1]);
  });
});

describe("nextSort", () => {
  it("reverses the order when the sorted column is clicked again", () => {
    expect(nextSort({ column: "date", descending: true }, "date")).toEqual({ column: "date", descending: false });
    expect(nextSort({ column: "date", descending: false }, "date")).toEqual({ column: "date", descending: true });
  });

  it("starts a new column in its natural order: names A to Z, the rest biggest or latest first", () => {
    expect(nextSort(DEFAULT_SORT, "name")).toEqual({ column: "name", descending: false });
    expect(nextSort(DEFAULT_SORT, "distance")).toEqual({ column: "distance", descending: true });
    expect(nextSort({ column: "name", descending: false }, "date")).toEqual({ column: "date", descending: true });
  });
});

describe("listHikes with a search", () => {
  const walks = [
    { ...hike(1, 400), title: "Tour de l'Èze", name: "Morning walk" },
    { ...hike(2, 300), name: "Col de la Croix" },
    { ...hike(3, 200), name: "Lunch", note: "Picnic by the LAKE" },
    { ...hike(4, 100), name: "Afternoon", kind: "Cycling" },
  ];
  const found = (search: string) => listHikes(walks, { search }).hikes.map((h) => h.id);

  it("finds the displayed name, ignoring case and accents", () => {
    expect(found("eze")).toEqual([1]);
    expect(found("CROIX")).toEqual([2]);
  });

  it("also finds the GPX name of a hike with a title", () => {
    expect(found("morning")).toEqual([1]);
  });

  it("finds the note and the kind", () => {
    expect(found("lake")).toEqual([3]);
    expect(found("cycl")).toEqual([4]);
  });

  it("shows every hike for an empty or blank search", () => {
    expect(found("")).toEqual([1, 2, 3, 4]);
    expect(found("   ")).toEqual([1, 2, 3, 4]);
  });

  it("counts the hikes shown out of those given", () => {
    const list = listHikes(walks, { search: "de" });
    expect([list.shown, list.total]).toEqual([2, 4]);
    expect(listHikes(walks, { search: "nowhere" }).shown).toBe(0);
  });
});
