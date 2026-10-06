<script lang="ts">
  import { onMount, tick } from "svelte";
  import { Map as MapLibreMap, setWorkerUrl, type GeoJSONSource } from "maplibre-gl";
  import "maplibre-gl/dist/maplibre-gl.css";
  // MapLibre looks for its worker next to its own file, which bundling moves: Vite bundles the
  // worker (with the chunk it imports) and MapLibre is told where it is.
  import workerUrl from "maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url";
  import { commands, type MapTrack } from "../bindings";
  import { endpoints, gapLines, isEmpty, segmentLines, trackBounds } from "./mapData";
  import { BACKGROUND_STYLE_URL, currentScheme, darkQuery, themeColors } from "./mapStyle";

  // Map of one hike's track. `version` changes on every reload: the track is fetched again, but
  // the camera only moves when the hike changes. `expanded`: the map fills the window (bound by the
  // panel, whose Escape closes it first).
  let { id, version, expanded = $bindable(false) }: { id: number; version: number; expanded?: boolean } = $props();

  setWorkerUrl(workerUrl);

  type Style = Exclude<Parameters<MapLibreMap["setStyle"]>[0], string | null>;

  let container = $state<HTMLDivElement | null>(null);
  // Raw: always replaced whole, and handed to MapLibre (which sends it to its worker) unproxied.
  let track = $state.raw<MapTrack | null>(null);
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

  const SOURCES = ["track-lines", "track-gaps", "track-ends"] as const;

  function sourceData(source: (typeof SOURCES)[number], data: MapTrack) {
    if (source === "track-lines") return segmentLines(data);
    if (source === "track-gaps") return gapLines(data);
    return endpoints(data);
  }

  // Added again after every style change (a style change drops them), with the theme's colours.
  function addTrackLayers() {
    if (map === null) return;
    const empty: MapTrack = { segments: [], gaps: [] };
    for (const source of SOURCES) {
      map.addSource(source, { type: "geojson", data: sourceData(source, track ?? empty) });
    }
    const { accent, bg } = themeColors();
    const round = { "line-cap": "round", "line-join": "round" } as const;
    map.addLayer({
      id: "track-gaps",
      type: "line",
      source: "track-gaps",
      layout: round,
      paint: { "line-color": accent, "line-width": 2, "line-opacity": 0.7, "line-dasharray": [1, 2] },
    });
    map.addLayer({
      id: "track-casing",
      type: "line",
      source: "track-lines",
      layout: round,
      paint: { "line-color": bg, "line-width": 7 },
    });
    map.addLayer({
      id: "track-line",
      type: "line",
      source: "track-lines",
      layout: round,
      paint: { "line-color": accent, "line-width": 4 },
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
  }

  function showTrack(target: number) {
    if (map === null || track === null || !map.getSource(SOURCES[0])) return;
    for (const source of SOURCES) {
      map.getSource<GeoJSONSource>(source)?.setData(sourceData(source, track));
    }
    if (framed === target) return;
    const bounds = trackBounds(track);
    if (bounds === null) return;
    map.fitBounds(bounds, { padding: 24, maxZoom: 16, duration: 0 });
    framed = target;
  }

  // --- Background --------------------------------------------------------------------------------

  // Without network (or if OpenFreeMap fails), the track is drawn on a plain background.
  function plainStyle(): Style {
    return {
      version: 8,
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
      const response = await fetch(BACKGROUND_STYLE_URL[currentScheme()], { signal: AbortSignal.timeout(10_000) });
      if (response.ok) {
        // The style's own camera would undo the framing on the track.
        const { center: _center, zoom: _zoom, bearing: _bearing, pitch: _pitch, ...rest } = await response.json();
        style = rest;
      }
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
  {:else if backgroundUnavailable}
    <p class="notice">Map background unavailable</p>
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
  /* Bottom left: the attribution is bottom right, and the track is framed away from the edges. */
  .notice {
    position: absolute;
    bottom: 8px;
    left: 8px;
    margin: 0;
    padding: 2px 8px;
    border-radius: 6px;
    background: var(--bg);
    color: var(--muted);
    font-size: 0.78rem;
    pointer-events: none;
  }
  .danger-text {
    color: var(--danger);
  }
</style>
