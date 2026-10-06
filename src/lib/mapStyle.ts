// The hike map's background, kept in one place so that another provider is a one-line change.

/** OpenFreeMap vector styles (no key), one per macOS appearance. */
export const BACKGROUND_STYLE_URL = {
  light: "https://tiles.openfreemap.org/styles/liberty",
  dark: "https://tiles.openfreemap.org/styles/dark",
} as const;

export type Scheme = keyof typeof BACKGROUND_STYLE_URL;

export const darkQuery = () => window.matchMedia("(prefers-color-scheme: dark)");

export const currentScheme = (): Scheme => (darkQuery().matches ? "dark" : "light");

/** The theme colours of `app.css` the map draws with, read when the style (re)loads. */
export function themeColors() {
  const css = getComputedStyle(document.documentElement);
  const read = (name: string) => css.getPropertyValue(name).trim();
  return {
    accent: read("--accent"),
    bg: read("--bg"),
    surface: read("--surface"),
    muted: read("--muted"),
    stopped: read("--speed-stopped"),
    speed: ["--speed-1", "--speed-2", "--speed-3", "--speed-4"].map(read),
  };
}
