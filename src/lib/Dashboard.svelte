<script lang="ts">
  import { commands, type Dashboard, type KindFilter, type View } from "../bindings";
  import BarChart, { type Bar } from "./BarChart.svelte";
  import HikeList from "./HikeList.svelte";
  import KindPicker from "./KindPicker.svelte";
  import {
    formatBarLabel,
    formatBucket,
    formatMetric,
    formatMetricTick,
    formatWindow,
    formatWithoutElevation,
    metricMinStep,
    metricName,
    metricValue,
    type Metric,
  } from "./format";

  // `version` changes on every reload (focus, ⌘R): the dashboard is fetched again.
  // `anchor`, `view`, `metric` and `kindFilter` are owned by the parent so they survive a visit to
  // Settings.
  // `anchor` is a date (`null` = today, in the machine's time zone, resolved in Rust): every view
  // but All shows the window containing it, so switching views keeps it in sight.
  let {
    version,
    anchor = $bindable(null),
    view = $bindable("week"),
    metric = $bindable("distance"),
    kindFilter = $bindable({ filter: "all" }),
    onchanged,
  }: {
    version: number;
    anchor?: string | null;
    view?: View;
    metric?: Metric;
    kindFilter?: KindFilter;
    // Called after a hike edit is saved: the parent reloads (bumps `version`).
    onchanged?: () => void;
  } = $props();

  const VIEWS: { label: string; title: string; value: View }[] = [
    { label: "W", title: "Week", value: "week" },
    { label: "M", title: "Month", value: "month" },
    { label: "Y", title: "Year", value: "year" },
    { label: "All", title: "All years", value: "all" },
  ];
  const UNIT: Record<View, string> = { week: "week", month: "month", year: "year", all: "year" };
  const BUCKET: Record<View, string> = { week: "day", month: "day", year: "month", all: "year" };
  const METRICS: { label: string; value: Metric }[] = [
    { label: "Distance", value: "distance" },
    { label: "Ascent", value: "ascent" },
    { label: "Moving time", value: "moving" },
    { label: "Hikes", value: "count" },
  ];

  let dashboard = $state<Dashboard | null>(null);
  let failure = $state<string | null>(null);
  // The kind filter of the dashboard shown, part of the window a selection belongs to.
  let shownFilter = $state("");

  // Only the latest request may update the view (navigation and reloads can overlap).
  let latest = 0;

  $effect(() => {
    void version;
    const id = ++latest;
    const filter = $state.snapshot(kindFilter);
    commands.dashboard(view, anchor, filter).then((result) => {
      if (id !== latest) return;
      if (result.status === "ok") {
        dashboard = result.data;
        shownFilter = JSON.stringify(filter);
        failure = null;
      } else {
        failure = result.error;
      }
    });
  });

  // The clicked bar, tied to the window it was clicked in: moving to another window (or view, or
  // kind filter) clears it, a reload of the same window keeps it and refreshes its hikes.
  let selection = $state<{ window: string; key: string } | null>(null);
  const windowOf = (d: Dashboard) => `${d.view}:${d.first}:${shownFilter}`;
  const selected = $derived(
    dashboard && selection?.window === windowOf(dashboard)
      ? (dashboard.buckets.find((b) => b.start === selection!.key) ?? null)
      : null,
  );
  function select(key: string) {
    if (!dashboard) return;
    selection = selected?.start === key ? null : { window: windowOf(dashboard), key };
  }

  // Labels follow the view of the data shown, not the one being fetched.
  const bars = $derived<Bar[]>(
    (dashboard?.buckets ?? []).map((b) => ({
      key: b.start,
      label: formatBarLabel(dashboard!.view, b.start),
      value: metricValue(metric, b.metrics),
      description: `${formatBucket(dashboard!.view, b.start)}: ${formatMetric(metric, metricValue(metric, b.metrics))}`,
    })),
  );

  // Missing elevation only lowers the ascent, so the hint is only shown with that metric.
  const hint = $derived(dashboard && metric === "ascent" ? formatWithoutElevation(dashboard.withoutElevation) : null);
</script>

<section class="dashboard">
  <div class="views" role="group" aria-label="View">
    {#each VIEWS as v (v.value)}
      <button aria-pressed={v.value === view} title={v.title} onclick={() => (view = v.value)}>{v.label}</button>
    {/each}
  </div>

  <KindPicker bind:value={kindFilter} {version} />

  {#if failure}
    <p class="message danger">{failure}</p>
  {/if}

  {#if dashboard}
    <header>
      <div class="total">
        <span class="label">Total {metricName(metric)}</span>
        <span class="value">{formatMetric(metric, metricValue(metric, dashboard.totals))}</span>
        {#if hint}
          <span class="hint">{hint}</span>
        {/if}
      </div>
      <div class="range">
        <!-- No navigation in All (`previous` and `next` are null): the anchor is kept for the other views. -->
        {#if dashboard.previous}
          <button class="nav" aria-label={`Previous ${UNIT[dashboard.view]}`} onclick={() => (anchor = dashboard!.previous)}
            >‹</button
          >
        {/if}
        <span>{formatWindow(dashboard.view, dashboard.first, dashboard.last)}</span>
        {#if dashboard.next}
          <button class="nav" aria-label={`Next ${UNIT[dashboard.view]}`} onclick={() => (anchor = dashboard!.next)}>›</button>
        {/if}
      </div>
    </header>

    <select class="metric" aria-label="Metric" bind:value={metric}>
      {#each METRICS as m (m.value)}
        <option value={m.value}>{m.label}</option>
      {/each}
    </select>

    <BarChart
      {bars}
      formatTick={(v) => formatMetricTick(metric, v)}
      formatValue={(v) => formatMetric(metric, v)}
      minStep={metricMinStep(metric)}
      label={`${METRICS.find((m) => m.value === metric)?.label} per ${BUCKET[dashboard.view]}, ${formatWindow(dashboard.view, dashboard.first, dashboard.last)}`}
      selected={selected?.start ?? null}
      onselect={select}
    />

    {#if selected}
      {#key selected.start}
        <HikeList title={formatBucket(dashboard.view, selected.start)} ids={selected.hikeIds} {version} {onchanged} />
      {/key}
    {/if}
  {:else if !failure}
    <p class="muted">Loading…</p>
  {/if}
</section>

<style>
  .dashboard {
    max-width: 760px;
    margin: 0 auto;
  }
  .views {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 2px;
    padding: 2px;
    border-radius: 8px;
    background: var(--surface);
  }
  .views button {
    border: none;
    background: transparent;
    border-radius: 6px;
    padding: 3px 0;
    font-weight: 500;
  }
  .views button[aria-pressed="true"] {
    background: var(--bg);
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.15);
  }
  header {
    margin: 20px 0 12px;
  }
  .total {
    display: flex;
    flex-direction: column;
  }
  .label {
    color: var(--muted);
    font-size: 0.78rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .value {
    font-size: 2.2rem;
    font-weight: 600;
    line-height: 1.2;
    font-variant-numeric: tabular-nums;
  }
  .hint {
    color: var(--warning);
    font-size: 0.85rem;
  }
  .metric {
    display: block;
    margin: 0 0 8px auto;
    font: inherit;
  }
  .range {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
  }
  .nav {
    border: none;
    background: none;
    padding: 0 6px;
    font-size: 1.3rem;
    line-height: 1;
    color: var(--accent);
  }
  .message {
    padding: 12px 16px;
    border-radius: 8px;
    background: var(--surface);
  }
  .danger {
    border-left: 4px solid var(--danger);
  }
  .muted {
    color: var(--muted);
  }
</style>
