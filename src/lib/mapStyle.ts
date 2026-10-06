// The hike map's background, kept in one place so that another provider is a one-line change.

/** The fonts of the OpenFreeMap styles, also given to the plain background (text needs them). */
export const GLYPHS_URL = "https://tiles.openfreemap.org/fonts/{fontstack}/{range}.pbf";

/** OpenFreeMap vector styles (no key), one per macOS appearance. */
export const BACKGROUND_STYLE_URL = {
  light: "https://tiles.openfreemap.org/styles/liberty",
  dark: "https://tiles.openfreemap.org/styles/dark",
} as const;

export type Scheme = keyof typeof BACKGROUND_STYLE_URL;

type Paint = Record<string, unknown>;
type Layer = { id: string; paint?: Paint };

// OpenFreeMap's dark style draws paths, minor roads and labels barely above its black background:
// they are lightened here, keeping its neutral greys. A layer renamed upstream is just left as is.
const NAME_HALO = { "text-halo-color": "#000000" };
const DARK_OVERRIDES: Record<string, Paint> = {
  highway_path: { "line-color": "#8e8e93" },
  highway_minor: { "line-color": "#3a3a3c" },
  highway_major_inner: { "line-color": "#48484a" },
  highway_major_subtle: { "line-color": "#3a3a3c" },
  highway_motorway_subtle: { "line-color": "#3a3a3c" },
  railway: { "line-color": "#48484a" },
  railway_minor: { "line-color": "#48484a" },
  railway_transit: { "line-color": "#48484a" },
  building: { "fill-color": "#202022" },
  water: { "fill-color": "#1b2733" },
  waterway: { "line-color": "#2c3e50" },
  water_name: { "text-color": "#6e8ca8", ...NAME_HALO },
  highway_name_other: { "text-color": "#aeaeb2", ...NAME_HALO },
  highway_name_motorway: { "text-color": "#aeaeb2", ...NAME_HALO },
};
/** Every `place_…` label (villages, towns, cities…). */
const DARK_PLACE_LABEL: Paint = { "text-color": "#d1d1d6", ...NAME_HALO };

function darkOverride(id: string): Paint | undefined {
  return DARK_OVERRIDES[id] ?? (id.startsWith("place_") ? DARK_PLACE_LABEL : undefined);
}

/**
 * The fetched style, ready for the map: without its own camera (which would undo the framing on
 * the track), and with the dark style's paths and labels lightened.
 */
export function prepareStyle<S extends { layers: Layer[] }>(fetched: S, scheme: Scheme): S {
  const { center: _center, zoom: _zoom, bearing: _bearing, pitch: _pitch, ...rest } = fetched as S & Record<string, unknown>;
  // The camera keys are optional in a style: what is left is still one.
  const style = rest as unknown as S;
  if (scheme === "light") return style;
  return {
    ...style,
    layers: style.layers.map((layer) => {
      const override = darkOverride(layer.id);
      return override ? { ...layer, paint: { ...layer.paint, ...override } } : layer;
    }),
  };
}

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
    text: read("--text"),
    stopped: read("--speed-stopped"),
    speed: ["--speed-1", "--speed-2", "--speed-3", "--speed-4"].map(read),
  };
}
