<script lang="ts" module>
  export type Bar = {
    key: string;
    /** Axis label under the bar. */
    label: string;
    /** In the unit of the axis; 0 draws an empty slot. */
    value: number;
    /** Shown on hover, e.g. "Wed 30 Sep: 12.5 km". */
    description: string;
  };
</script>

<script lang="ts">
  // Hand-written SVG bar chart: one series, one accent colour, Apple Health–like.
  let {
    bars,
    formatTick,
    label,
    selected = null,
    onselect,
  }: {
    bars: Bar[];
    formatTick: (value: number) => string;
    label: string;
    /** Key of the highlighted bar. */
    selected?: string | null;
    /** Called with the key of a clicked bar (also Enter or Space when focused). */
    onselect?: (key: string) => void;
  } = $props();

  function onKeydown(event: KeyboardEvent, key: string) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onselect?.(key);
    }
  }

  const HEIGHT = 240;
  const TOP = 24; // room for the hover value above the tallest bar
  const BOTTOM = 28; // day labels
  const RIGHT = 56; // tick labels
  const RADIUS = 4;

  let width = $state(600);
  let hovered = $state<number | null>(null);

  /** A round step (1, 2 or 5 × 10ⁿ) giving about three grid lines. */
  function niceStep(max: number): number {
    if (max <= 0) return 1;
    const raw = max / 3;
    const pow = 10 ** Math.floor(Math.log10(raw));
    return ([1, 2, 5, 10].find((m) => m * pow >= raw) ?? 10) * pow;
  }

  const step = $derived(niceStep(Math.max(0, ...bars.map((b) => b.value))));
  const top = $derived(Math.max(1, Math.ceil(Math.max(0, ...bars.map((b) => b.value)) / step)) * step);
  const ticks = $derived(Array.from({ length: Math.round(top / step) + 1 }, (_, i) => i * step));

  const plotWidth = $derived(Math.max(0, width - RIGHT));
  const plotHeight = HEIGHT - TOP - BOTTOM;
  const band = $derived(plotWidth / Math.max(1, bars.length));
  const barWidth = $derived(Math.min(40, band * 0.56));

  const y = (value: number) => TOP + plotHeight - (value / top) * plotHeight;

  /** A bar with rounded top corners, anchored to the baseline. */
  function barPath(i: number, value: number): string {
    const x = i * band + (band - barWidth) / 2;
    const h = Math.max(2, (value / top) * plotHeight); // a tiny value stays visible
    const r = Math.min(RADIUS, h, barWidth / 2);
    const base = TOP + plotHeight;
    const t = base - h;
    return `M${x},${base} V${t + r} Q${x},${t} ${x + r},${t} H${x + barWidth - r} Q${x + barWidth},${t} ${x + barWidth},${t + r} V${base} Z`;
  }
</script>

<figure bind:clientWidth={width}>
  <svg {width} height={HEIGHT} role="group" aria-label={label}>
    {#each ticks as tick (tick)}
      <line class="grid" class:baseline={tick === 0} x1="0" x2={plotWidth} y1={y(tick)} y2={y(tick)} />
      <text class="tick" x={plotWidth + 8} y={y(tick)} dy="0.35em">{formatTick(tick)}</text>
    {/each}

    {#each bars as bar, i (bar.key)}
      <g
        role="button"
        tabindex="0"
        aria-label={bar.description}
        aria-pressed={selected === bar.key}
        class:clickable={onselect !== undefined}
        onmouseenter={() => (hovered = i)}
        onmouseleave={() => (hovered = null)}
        onclick={() => onselect?.(bar.key)}
        onkeydown={(e) => onKeydown(e, bar.key)}
      >
        <!-- The whole column is the hover and click target, larger than the bar. -->
        <rect
          class="column"
          class:active={hovered === i}
          class:selected={selected === bar.key}
          x={i * band + 2}
          y={TOP}
          width={band - 4}
          height={plotHeight}
          rx="6"
        />
        {#if bar.value > 0}
          <path class="bar" class:dimmed={selected !== null && selected !== bar.key} d={barPath(i, bar.value)} />
        {/if}
        <text class="day" class:selected={selected === bar.key} x={i * band + band / 2} y={HEIGHT - 8} text-anchor="middle"
          >{bar.label}</text
        >
        {#if hovered === i || selected === bar.key}
          <text class="value" x={i * band + band / 2} y={y(bar.value) - 8} text-anchor="middle">
            {formatTick(bar.value)}
          </text>
        {/if}
        <title>{bar.description}</title>
      </g>
    {/each}
  </svg>
</figure>

<style>
  figure {
    margin: 0;
    width: 100%;
  }
  svg {
    display: block;
    overflow: visible;
  }
  .grid {
    stroke: var(--grid);
    stroke-width: 1;
    stroke-dasharray: 2 3;
  }
  .grid.baseline {
    stroke: var(--border);
    stroke-dasharray: none;
  }
  .tick {
    fill: var(--muted);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  .day {
    fill: var(--muted);
    font-size: 12px;
  }
  .value {
    fill: var(--text);
    font-size: 12px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .column {
    fill: transparent;
  }
  .column.active,
  .column.selected {
    fill: var(--surface);
  }
  .column.selected {
    stroke: var(--accent);
    stroke-width: 1.5;
  }
  .clickable {
    cursor: pointer;
  }
  g:focus {
    outline: none;
  }
  g:focus-visible .column {
    stroke: var(--accent);
    stroke-width: 2;
  }
  .day.selected {
    fill: var(--accent);
    font-weight: 600;
  }
  .bar {
    fill: var(--chart-bar);
  }
  .bar.dimmed {
    opacity: 0.45;
  }
</style>
