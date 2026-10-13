// The kind field of the hike panel. The backend trims and stores a blank kind as NULL; the panel
// applies the same rule to know whether there is anything to save.

/** The kind a typed value stands for: trimmed, `null` (Not set) if blank. */
export function normalizeKind(value: string): string | null {
  const kind = value.trim();
  return kind === "" ? null : kind;
}

/** Whether saving `typed` would change the stored kind `stored` (compared exactly, case included). */
export function kindChanged(typed: string, stored: string | null): boolean {
  return normalizeKind(typed) !== stored;
}
