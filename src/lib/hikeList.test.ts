import { describe, expect, it } from "vitest";
import type { HikeDetails } from "../bindings";
import { listHikes } from "./hikeList";

function hike(id: number, startedAt: number | null): HikeDetails {
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
  };
}

const ids = (hikes: HikeDetails[]) => listHikes(hikes).hikes.map((h) => h.id);

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
});

describe("listHikes with a search", () => {
  const walks = [
    { ...hike(1, 400), title: "Tour de l'Èze", name: "Morning walk" },
    { ...hike(2, 300), name: "Col de la Croix" },
    { ...hike(3, 200), name: "Lunch", note: "Picnic by the LAKE" },
    { ...hike(4, 100), name: "Afternoon", kind: "Cycling" },
  ];
  const found = (search: string) => listHikes(walks, search).hikes.map((h) => h.id);

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
    const list = listHikes(walks, "de");
    expect([list.shown, list.total]).toEqual([2, 4]);
    expect(listHikes(walks, "nowhere").shown).toBe(0);
  });
});
