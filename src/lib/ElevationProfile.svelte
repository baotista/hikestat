<script lang="ts">
  import type { MapTrack } from "../bindings";
  import { formatDistance, formatElevation } from "./format";
  import { type ProfilePoint, kmTicks, profileExtent, profileSeries } from "./profile";

  // Elevation against distance, hand-drawn like the bar chart (ADR 0003). Hovering shows a line
  // with the km and the elevation there; `hovered` (that distance, in metres) is bound by the panel
  // so the map can mark the same spot.
  let {
    track,
    height = 120,
    hovered = $bindable(null),
  }: { track: MapTrack; height?: number; hovered?: number | null } = $props();

  const PAD = { top: 8, right: 8, bottom: 20, left: 44 };

  let width = $state(0);

  const series = $derived(profileSeries(track));
  const extent = $derived(profileExtent(track, series));

  // A little room above and below the elevations drawn (at least 10 m, so a flat walk isn't a cliff).
  const range = $derived.by(() => {
    if (extent === null) return null;
    const margin = Math.max(10, (extent.maxElevationM - extent.minElevationM) * 0.1);
    return { low: extent.minElevationM - margin, high: extent.maxElevationM + margin };
  });

  const plotWidth = $derived(Math.max(0, width - PAD.left - PAD.right));
  const plotHeight = $derived(height - PAD.top - PAD.bottom);

  function x(distanceM: number): number {
    return PAD.left + (extent && extent.distanceM > 0 ? (distanceM / extent.distanceM) * plotWidth : 0);
  }
  function y(elevationM: number): number {
    if (range === null) return PAD.top;
    return PAD.top + ((range.high - elevationM) / (range.high - range.low)) * plotHeight;
  }

  function line(points: ProfilePoint[]): string {
    return "M " + points.map((p) => `${x(p.distanceM)},${y(p.elevationM)}`).join(" L ");
  }
  function area(points: ProfilePoint[]): string {
    const bottom = PAD.top + plotHeight;
    return `${line(points)} L ${x(points.at(-1)!.distanceM)},${bottom} L ${x(points[0].distanceM)},${bottom} Z`;
  }

  // A trou de signal adds no distance, so it has no width here: a dashed line marks where it is.
  const gaps = $derived(track.segments.slice(1).map((s) => s.distancesM[0]));

  const ticks = $derived(extent ? kmTicks(extent.distanceM, plotWidth, 50) : []);
  const tickLabel = (m: number) => {
    const step = ticks.length > 1 ? ticks[1] - ticks[0] : 1000;
    return (m / 1000).toFixed(step < 1000 ? 1 : 0);
  };

  /** The elevation at `distanceM`, interpolated within the series covering it, if any. */
  function elevationAt(distanceM: number): number | null {
    for (const points of series) {
      if (distanceM < points[0].distanceM || distanceM > points.at(-1)!.distanceM) continue;
      const i = Math.max(1, points.findIndex((p) => p.distanceM >= distanceM));
      const [a, b] = [points[i - 1], points[i]];
      const t = b.distanceM === a.distanceM ? 0 : (distanceM - a.distanceM) / (b.distanceM - a.distanceM);
      return a.elevationM + t * (b.elevationM - a.elevationM);
    }
    return null;
  }

  const hoveredElevation = $derived(hovered === null ? null : elevationAt(hovered));

  function onPointerMove(event: PointerEvent & { currentTarget: SVGSVGElement }) {
    if (extent === null || plotWidth === 0) return;
    const left = event.currentTarget.getBoundingClientRect().left;
    const fraction = (event.clientX - left - PAD.left) / plotWidth;
    hovered = Math.min(1, Math.max(0, fraction)) * extent.distanceM;
  }
</script>

<div class="profile" bind:clientWidth={width} style:height={`${height}px`}>
  {#if extent === null || range === null}
    <p class="empty">No elevation data</p>
  {:else if width > 0}
    <svg
      {width}
      {height}
      role="img"
      aria-label={`Elevation profile, from ${formatElevation(extent.minElevationM)} to ${formatElevation(extent.maxElevationM)} over ${formatDistance(extent.distanceM)}`}
      onpointermove={onPointerMove}
      onpointerleave={() => (hovered = null)}
    >
      <line class="baseline" x1={PAD.left} x2={PAD.left + plotWidth} y1={PAD.top + plotHeight} y2={PAD.top + plotHeight} />
      {#each series as points, i (i)}
        <path class="area" d={area(points)} />
        <path class="line" d={line(points)} />
      {/each}
      {#each gaps as gap, i (i)}
        <line class="gap" x1={x(gap)} x2={x(gap)} y1={PAD.top} y2={PAD.top + plotHeight} />
      {/each}
      <text class="label" x={PAD.left - 6} y={y(extent.maxElevationM)} text-anchor="end" dominant-baseline="middle">
        {formatElevation(extent.maxElevationM)}
      </text>
      <text class="label" x={PAD.left - 6} y={y(extent.minElevationM)} text-anchor="end" dominant-baseline="middle">
        {formatElevation(extent.minElevationM)}
      </text>
      {#each ticks as tick (tick)}
        <text class="label" x={x(tick)} y={height - 4} text-anchor="middle">{tickLabel(tick)}</text>
      {/each}
      <text class="label" x={PAD.left + plotWidth} y={height - 4} text-anchor="end">km</text>
      {#if hovered !== null}
        <line class="hover" x1={x(hovered)} x2={x(hovered)} y1={PAD.top} y2={PAD.top + plotHeight} />
        {#if hoveredElevation !== null}
          <circle class="dot" cx={x(hovered)} cy={y(hoveredElevation)} r="3.5" />
        {/if}
        <text
          class="value"
          x={x(hovered) > PAD.left + plotWidth / 2 ? x(hovered) - 6 : x(hovered) + 6}
          y={PAD.top + 10}
          text-anchor={x(hovered) > PAD.left + plotWidth / 2 ? "end" : "start"}
        >
          {formatDistance(hovered)}{hoveredElevation === null ? "" : ` · ${formatElevation(hoveredElevation)}`}
        </text>
      {/if}
    </svg>
  {/if}
</div>

<style>
  .profile {
    position: relative;
    width: 100%;
  }
  svg {
    display: block;
    touch-action: none;
  }
  .area {
    fill: var(--accent);
    fill-opacity: 0.22;
  }
  .line {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.5;
    stroke-linejoin: round;
  }
  .gap {
    stroke: var(--speed-stopped);
    stroke-dasharray: 2 3;
  }
  .baseline {
    stroke: var(--border);
  }
  .label {
    fill: var(--muted);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
  }
  .hover {
    stroke: var(--text);
    stroke-opacity: 0.5;
  }
  .dot {
    fill: var(--accent);
    stroke: var(--bg);
    stroke-width: 1.5;
  }
  .value {
    fill: var(--text);
    font-size: 0.75rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    paint-order: stroke;
    stroke: var(--bg);
    stroke-width: 3px;
  }
  .empty {
    margin: 0;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    font-size: 0.85rem;
  }
</style>
