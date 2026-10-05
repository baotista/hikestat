<script lang="ts">
  import { commands, type Dashboard, type KindFilter, type View } from "../bindings";
  import BarChart, { type Bar } from "./BarChart.svelte";
  import HikeList from "./HikeList.svelte";
  import { formatBarLabel, formatBucket, formatDistance, formatWindow } from "./format";

  // `version` changes on every reload (focus, ⌘R): the dashboard is fetched again.
  // `anchor` and `view` are owned by the parent so the window shown survives a visit to Settings.
  // `anchor` is a date (`null` = today, in the machine's time zone, resolved in Rust): every view
  // but All shows the window containing it, so switching views keeps it in sight.
  let {
    version,
    anchor = $bindable(null),
    view = $bindable("week"),
  }: { version: number; anchor?: string | null; view?: View } = $props();

  const VIEWS: { label: string; title: string; value: View }[] = [
    { label: "W", title: "Week", value: "week" },
    { label: "M", title: "Month", value: "month" },
    { label: "Y", title: "Year", value: "year" },
    { label: "All", title: "All years", value: "all" },
  ];
  const UNIT: Record<View, string> = { week: "week", month: "month", year: "year", all: "year" };
  const BUCKET: Record<View, string> = { week: "day", month: "day", year: "month", all: "year" };
  const ALL: KindFilter = { filter: "all" };

  let dashboard = $state<Dashboard | null>(null);
  let failure = $state<string | null>(null);

  // Only the latest request may update the view (navigation and reloads can overlap).
  let latest = 0;

  $effect(() => {
    void version;
    const id = ++latest;
    commands.dashboard(view, anchor, ALL).then((result) => {
      if (id !== latest) return;
      if (result.status === "ok") {
        dashboard = result.data;
        failure = null;
      } else {
        failure = result.error;
      }
    });
  });

  // The clicked bar, tied to the window it was clicked in: moving to another window (or view)
  // clears it, a reload of the same window keeps it and refreshes its hikes.
  let selection = $state<{ window: string; key: string } | null>(null);
  const windowOf = (d: Dashboard) => `${d.view}:${d.first}`;
  const selected = $derived(
    dashboard && selection?.window === windowOf(dashboard)
      ? (dashboard.buckets.find((b) => b.start === selection!.key) ?? null)
      : null,
  );
  function select(key: string) {
    if (!dashboard) return;
    selection = selected?.start === key ? null : { window: windowOf(dashboard), key };
  }

  const km = (metres: number | null) => (metres ?? 0) / 1000;

  // Labels follow the view of the data shown, not the one being fetched.
  const bars = $derived<Bar[]>(
    (dashboard?.buckets ?? []).map((b) => ({
      key: b.start,
      label: formatBarLabel(dashboard!.view, b.start),
      value: km(b.metrics.distanceM),
      description: `${formatBucket(dashboard!.view, b.start)}: ${formatDistance(b.metrics.distanceM ?? 0)}`,
    })),
  );
</script>

<section class="dashboard">
  <div class="views" role="group" aria-label="View">
    {#each VIEWS as v (v.value)}
      <button aria-pressed={v.value === view} title={v.title} onclick={() => (view = v.value)}>{v.label}</button>
    {/each}
  </div>

  {#if failure}
    <p class="message danger">{failure}</p>
  {/if}

  {#if dashboard}
    <header>
      <div class="total">
        <span class="label">Total distance</span>
        <span class="value">{formatDistance(dashboard.totals.distanceM ?? 0)}</span>
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

    <BarChart
      {bars}
      formatTick={(v) => formatDistance(v * 1000)}
      label={`Distance per ${BUCKET[dashboard.view]}, ${formatWindow(dashboard.view, dashboard.first, dashboard.last)}`}
      selected={selected?.start ?? null}
      onselect={select}
    />

    {#if selected}
      {#key selected.start}
        <HikeList title={formatBucket(dashboard.view, selected.start)} ids={selected.hikeIds} {version} />
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
