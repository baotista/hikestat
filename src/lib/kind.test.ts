import { describe, expect, it } from "vitest";
import { kindChanged, normalizeKind } from "./kind";

describe("normalizeKind", () => {
  it("keeps a kind as typed", () => {
    expect(normalizeKind("Ski touring")).toBe("Ski touring");
  });

  it("trims", () => {
    expect(normalizeKind("  walking \n")).toBe("walking");
  });

  it("reads an empty or blank value as Not set", () => {
    expect(normalizeKind("")).toBeNull();
    expect(normalizeKind(" \t ")).toBeNull();
  });
});

describe("kindChanged", () => {
  it("is false for the stored kind, even with surrounding spaces", () => {
    expect(kindChanged("walking", "walking")).toBe(false);
    expect(kindChanged(" walking ", "walking")).toBe(false);
  });

  it("is false for a blank value when no kind is stored", () => {
    expect(kindChanged("", null)).toBe(false);
    expect(kindChanged("  ", null)).toBe(false);
  });

  it("is true for another kind, case included", () => {
    expect(kindChanged("cycling", "walking")).toBe(true);
    expect(kindChanged("Walking", "walking")).toBe(true);
  });

  it("is true when setting or clearing a kind", () => {
    expect(kindChanged("walking", null)).toBe(true);
    expect(kindChanged("", "walking")).toBe(true);
  });
});
