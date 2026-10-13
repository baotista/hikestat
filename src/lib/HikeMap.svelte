<script lang="ts">
  import { onMount, tick } from "svelte";
  import { Map as MapLibreMap, setWorkerUrl, type GeoJSONSource } from "maplibre-gl";
  import "maplibre-gl/dist/maplibre-gl.css";
  // MapLibre looks for its worker next to its own file, which bundling moves: Vite bundles the
  // worker (with the chunk it imports) and MapLibre is told where it is.
  import workerUrl from "maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url";
  import { commands, type MapTrack } from "../bindings";
  import { formatSpeed } from "./format";
  import {
    KM_MIN_ZOOM,
    endpoints,
    gapLines,
    isEmpty,
    kmMarkers,
    segmentLines,
    speedLines,
    trackBounds,
  } from "./mapData";
  import type { FeatureCollection, Point } from "geojson";
  import { positionAt } from "./profile";
  import { BACKGROUND_STYLE_URL, GLYPHS_URL, currentScheme, darkQuery, prepareStyle, themeColors } from "./mapStyle";

  // Map of one hike's track. `version` changes on every reload: the track is fetched again, but
  // the camera only moves when the hike changes. `expanded`: the map fills the window (bound by the
  // panel, whose Escape closes it first).
  // `track`: the hike's map track once loaded, bound by the panel for the elevation profile.
  let {
    id,
    version,
    expanded = $bindable(false),
    track = $bindable(null),
    hovered = null,
  }: {
    id: number;
    version: number;
    expanded?: boolean;
    track?: MapTrack | null;
    /** The distance (m) hovered on the elevation profile, marked on the map. */
    hovered?: number | null;
  } = $props();

  setWorkerUrl(workerUrl);

  type Style = Exclude<Parameters<MapLibreMap["setStyle"]>[0], string | null>;

  let container = $state<HTMLDivElement | null>(null);
  let failure = $state<string | null>(null);
  let backgroundUnavailable = $state(false);

  let map: MapLibreMap | null = null;
  // The hike the camera was last framed on.
  let framed: number | null = null;

  // Only the latest request may update the map (another hike or a reload can overlap).
  let latest = 0;

  $effect(() => {
    void version;
    const target = id;
    const request = ++latest;
    commands.track(target).then((result) => {
      if (request !== latest) return;
      if (result.status === "ok") {
        track = result.data;
        failure = null;
        showTrack(target);
        // A reload is also when an unavailable background is tried again.
        if (backgroundUnavailable) void applyBackground();
      } else {
        track = null;
        failure = result.error;
      }
    });
  });

  // --- Track layers ------------------------------------------------------------------------------

  const SOURCES = ["track-lines", "track-speed", "track-gaps", "track-ends", "track-km"] as const;

  function hoverData(): FeatureCollection<Point> {
    const at = track === null || hovered === null ? null : positionAt(track, hovered);
    return {
      type: "FeatureCollection",
      features: at === null ? [] : [{ type: "Feature", properties: {}, geometry: { type: "Point", coordinates: at } }],
    };
  }

  $effect(() => {
    void hovered;
    void track;
    map?.getSource<GeoJSONSource>("track-hover")?.setData(hoverData());
  });

  // Colours are read from the theme each time: a theme change reloads the style and the data.
  function sourceData(source: (typeof SOURCES)[number], data: MapTrack) {
    if (source === "track-lines") return segmentLines(data);
    if (source === "track-speed") return speedLines(data, themeColors());
    if (source === "track-gaps") return gapLines(data);
    if (source === "track-km") return kmMarkers(data);
    return endpoints(data);
  }

  // Added again after every style change (a style change drops them), with the theme's colours.
  function addTrackLayers() {
    if (map === null) return;
    const empty: MapTrack = { segments: [], gaps: [], speedScale: null, kmMarkers: [] };
    for (const source of SOURCES) {
      // The speed lines are one tiny line per intervalle: with the default tolerance, tiling drops
      // the shortest ones and the track looks dashed when zoomed out.
      const tolerance = source === "track-speed" ? 0 : undefined;
      map.addSource(source, { type: "geojson", data: sourceData(source, track ?? empty), tolerance });
    }
    const { accent, bg, stopped, muted, text } = themeColors();
    const round = { "line-cap": "round", "line-join": "round" } as const;
    map.addLayer({
      id: "track-gaps",
      type: "line",
      source: "track-gaps",
      layout: round,
      // Grey like a stretch without speed: nothing is measured across a trou de signal.
      paint: { "line-color": stopped, "line-width": 2, "line-opacity": 0.8, "line-dasharray": [1, 2] },
    });
    map.addLayer({
      id: "track-casing",
      type: "line",
      source: "track-lines",
      layout: round,
      paint: { "line-color": bg, "line-width": 7 },
    });
    // One feature per intervalle, coloured by speed (or whole segments in the accent colour).
    map.addLayer({
      id: "track-line",
      type: "line",
      source: "track-speed",
      layout: round,
      paint: { "line-color": ["get", "color"], "line-width": 4 },
    });
    // Start: filled dot. End: ringed dot, drawn after the start so it is on top on a loop.
    map.addLayer({
      id: "track-start",
      type: "circle",
      source: "track-ends",
      filter: ["==", ["get", "role"], "start"],
      paint: { "circle-radius": 6, "circle-color": accent, "circle-stroke-color": bg, "circle-stroke-width": 2 },
    });
    map.addLayer({
      id: "track-end",
      type: "circle",
      source: "track-ends",
      filter: ["==", ["get", "role"], "end"],
      paint: { "circle-radius": 5, "circle-color": bg, "circle-stroke-color": accent, "circle-stroke-width": 3 },
    });
    addKmLayers(bg, muted, text);
    // The spot hovered on the elevation profile, above everything else.
    map.addSource("track-hover", { type: "geojson", data: hoverData() });
    map.addLayer({
      id: "track-hover",
      type: "circle",
      source: "track-hover",
      paint: { "circle-radius": 6, "circle-color": text, "circle-stroke-color": bg, "circle-stroke-width": 2 },
    });
  }

  // Km markers: a round badge with the number. One layer per step, the multiples of 10 on top so
  // that they win when badges collide (overlapping ones are hidden).
  const KM_BADGE = "km-badge";
  const BADGE_PX = 22;

  function badgeImage(fill: string, stroke: string): ImageData {
    const ratio = 2;
    const size = BADGE_PX * ratio;
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = size;
    const context = canvas.getContext("2d")!;
    context.beginPath();
    context.arc(size / 2, size / 2, size / 2 - 2 * ratio, 0, 2 * Math.PI);
    context.fillStyle = fill;
    context.fill();
    context.lineWidth = 1.5 * ratio;
    context.strokeStyle = stroke;
    context.stroke();
    return context.getImageData(0, 0, size, size);
  }

  function addKmLayers(bg: string, muted: string, text: string) {
    if (map === null) return;
    map.addImage(KM_BADGE, badgeImage(bg, muted), { pixelRatio: 2 });
    for (const minZoom of [KM_MIN_ZOOM.every, KM_MIN_ZOOM.fives, KM_MIN_ZOOM.tens]) {
      map.addLayer({
        id: `track-km-${minZoom}`,
        type: "symbol",
        source: "track-km",
        minzoom: minZoom,
        filter: ["==", ["get", "minZoom"], minZoom],
        layout: {
          "icon-image": KM_BADGE,
          "text-field": ["get", "label"],
          "text-font": ["Noto Sans Bold"],
          "text-size": 9,
        },
        paint: { "text-color": text },
      });
    }
  }

  function showTrack(target: number) {
    if (map === null || track === null || !map.getSource(SOURCES[0])) return;
    for (const source of SOURCES) {
      map.getSource<GeoJSONSource>(source)?.setData(sourceData(source, track));
    }
    if (framed === target) return;
    const bounds = trackBounds(track);
    if (bounds === null) return;
    // Room at the top for the buttons and the notice, at the bottom for the legend.
    map.fitBounds(bounds, { padding: { top: 44, bottom: 40, left: 24, right: 24 }, maxZoom: 16, duration: 0 });
    framed = target;
  }

  // --- Background --------------------------------------------------------------------------------

  // Without network (or if OpenFreeMap fails), the track is drawn on a plain background.
  function plainStyle(): Style {
    return {
      version: 8,
      glyphs: GLYPHS_URL,
      sources: {},
      layers: [{ id: "background", type: "background", paint: { "background-color": themeColors().surface } }],
    };
  }

  let styleRequest = 0;

  // Loads the background matching the macOS appearance; called on mount and when it changes.
  async function applyBackground() {
    const request = ++styleRequest;
    let style: Style | null = null;
    try {
      const scheme = currentScheme();
      const response = await fetch(BACKGROUND_STYLE_URL[scheme], { signal: AbortSignal.timeout(10_000) });
      if (response.ok) style = prepareStyle(await response.json(), scheme);
    } catch {
      style = null;
    }
    if (request !== styleRequest || map === null) return;
    backgroundUnavailable = style === null;
    map.setStyle(style ?? plainStyle(), { diff: false });
  }

  onMount(() => {
    if (container === null) return;
    map = new MapLibreMap({
      container,
      style: plainStyle(),
      attributionControl: { compact: true },
      // A map inside a scrolling panel: no rotation or pitch to get lost in.
      dragRotate: false,
      pitchWithRotate: false,
      touchPitch: false,
    });
    map.touchZoomRotate.disableRotation();
    // The compact attribution starts expanded (until the first drag): start with just its ⓘ.
    map.once("load", () => {
      container?.querySelector(".maplibregl-ctrl-attrib")?.classList.remove("maplibregl-compact-show");
    });
    map.on("style.load", () => {
      addTrackLayers();
      if (track !== null) showTrack(id);
    });
    // Tiles that fail to load (offline after the style came) leave the background empty.
    map.on("error", (event) => {
      if ("sourceId" in event && typeof event.sourceId === "string" && !event.sourceId.startsWith("track-")) {
        backgroundUnavailable = true;
      }
    });
    void applyBackground();

    // The frame changes size when expanded or collapsed: the map keeps its centre and zoom.
    const resize = new ResizeObserver(() => map?.resize());
    resize.observe(container);

    const scheme = darkQuery();
    const onSchemeChange = () => void applyBackground();
    scheme.addEventListener("change", onSchemeChange);
    return () => {
      scheme.removeEventListener("change", onSchemeChange);
      resize.disconnect();
      map?.remove();
      map = null;
    };
  });

  // In the panel, the wheel scrolls the panel: it only reaches the map with ⌘ (or as a trackpad
  // pinch, which comes with ctrlKey). Expanded, it zooms.
  function onWheelCapture(event: WheelEvent) {
    if (!expanded && !event.metaKey && !event.ctrlKey) event.stopPropagation();
  }

  // --- Expanded map ------------------------------------------------------------------------------

  let expandButton = $state<HTMLButtonElement | null>(null);
  let collapseButton = $state<HTMLButtonElement | null>(null);

  // Focus follows the button that undoes the change (Escape or ✕ closes, ⤢ opens again); not on
  // mount, where the panel focuses its own close button.
  let wasExpanded = expanded;
  $effect(() => {
    const open = expanded;
    if (open === wasExpanded) return;
    wasExpanded = open;
    void tick().then(() => (open ? collapseButton : expandButton)?.focus());
  });
  // --- End of expanded map -----------------------------------------------------------------------
</script>

<div class="frame" class:expanded>
  <div class="map" bind:this={container} onwheelcapture={onWheelCapture}></div>
  {#if failure}
    <p class="cover danger-text">Cannot show the map: {failure}</p>
  {:else if track === null}
    <p class="cover">Loading…</p>
  {:else if isEmpty(track)}
    <p class="cover">No points</p>
  {:else}
    {#if backgroundUnavailable}
      <p class="notice">Map background unavailable</p>
    {/if}
    {#if track.speedScale}
      <div class="legend" aria-label="Speed colours">
        <span class="gradient"></span>
        <span>{formatSpeed(track.speedScale.lowMps)} – {formatSpeed(track.speedScale.highMps)}</span>
        <span class="swatch"></span>
        <span>Stopped</span>
      </div>
    {/if}
  {/if}
  {#if track !== null && !isEmpty(track) && !failure}
    {#if expanded}
      <button bind:this={collapseButton} class="corner" aria-label="Close map" onclick={() => (expanded = false)}>✕</button>
    {:else}
      <button bind:this={expandButton} class="corner" aria-label="Expand map" onclick={() => (expanded = true)}>⤢</button>
    {/if}
  {/if}
</div>

<style>
  .frame {
    position: relative;
    height: 220px;
    border: 1px solid var(--border);
    border-radius: 8px;
    overflow: hidden;
    background: var(--surface);
  }
  /* Expanded: the same frame over the whole window, above the panel. */
  .frame.expanded {
    position: fixed;
    inset: 0;
    height: auto;
    border: none;
    border-radius: 0;
    z-index: 20;
  }
  .map {
    position: absolute;
    inset: 0;
  }
  .corner {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 30px;
    height: 30px;
    padding: 0;
    background: var(--bg);
    box-shadow: 0 1px 4px rgb(0 0 0 / 0.15);
  }
  .cover {
    position: absolute;
    inset: 0;
    margin: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 16px;
    text-align: center;
    background: var(--surface);
    color: var(--muted);
  }
  /* Top left (the legend is bottom left): the track is framed away from both. */
  .notice {
    position: absolute;
    top: 8px;
    left: 8px;
    margin: 0;
    padding: 2px 8px;
    border-radius: 6px;
    background: var(--bg);
    color: var(--muted);
    font-size: 0.78rem;
    pointer-events: none;
  }
  /* Bottom left: the attribution is bottom right. */
  .legend {
    position: absolute;
    bottom: 8px;
    left: 8px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px;
    border-radius: 6px;
    background: var(--bg);
    color: var(--muted);
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
    pointer-events: none;
  }
  .gradient {
    width: 48px;
    height: 6px;
    border-radius: 3px;
    background: linear-gradient(to right, var(--speed-1), var(--speed-2), var(--speed-3), var(--speed-4));
  }
  .swatch {
    width: 10px;
    height: 6px;
    margin-left: 4px;
    border-radius: 3px;
    background: var(--speed-stopped);
  }
  .danger-text {
    color: var(--danger);
  }
</style>
