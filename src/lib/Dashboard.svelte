<script lang="ts">
  import { commands, type Dashboard, type KindFilter } from "../bindings";
  import BarChart, { type Bar } from "./BarChart.svelte";
  import { formatDay, formatDistance, formatRange, formatWeekday } from "./format";

  // `version` changes on every reload (focus, ⌘R): the dashboard is fetched again.
  let { version }: { version: number } = $props();

  const VIEWS = [
    { label: "W", title: "Week", enabled: true },
    { label: "M", title: "Month", enabled: false },
    { label: "Y", title: "Year", enabled: false },
    { label: "All", title: "All", enabled: false },
  ];
  const ALL: KindFilter = { filter: "all" };

  /** `null` = today, in the machine's time zone (resolved in Rust). */
  let anchor = $state<string | null>(null);
  let dashboard = $state<Dashboard | null>(null);
  let failure = $state<string | null>(null);

  // Only the latest request may update the view (navigation and reloads can overlap).
  let latest = 0;

  $effect(() => {
    void version;
    const id = ++latest;
    commands.dashboard("week", anchor, ALL).then((result) => {
      if (id !== latest) return;
      if (result.status === "ok") {
        dashboard = result.data;
        failure = null;
      } else {
        failure = result.error;
      }
    });
  });

  const km = (metres: number | null) => (metres ?? 0) / 1000;

  const bars = $derived<Bar[]>(
    (dashboard?.buckets ?? []).map((b) => ({
      key: b.start,
      label: formatWeekday(b.start),
      value: km(b.metrics.distanceM),
      description: `${formatDay(b.start)}: ${formatDistance(b.metrics.distanceM ?? 0)}`,
    })),
  );
</script>

<section class="dashboard">
  <div class="views" role="group" aria-label="View">
    {#each VIEWS as view (view.label)}
      <button
        aria-pressed={view.label === "W"}
        disabled={!view.enabled}
        title={view.enabled ? view.title : `${view.title} (coming soon)`}>{view.label}</button
      >
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
        <button class="nav" aria-label="Previous week" onclick={() => (anchor = dashboard!.previous)}>‹</button>
        <span>{formatRange(dashboard.first, dashboard.last)}</span>
        <button class="nav" aria-label="Next week" onclick={() => (anchor = dashboard!.next)}>›</button>
      </div>
    </header>

    <BarChart
      {bars}
      formatTick={(v) => formatDistance(v * 1000)}
      label={`Distance per day, ${formatRange(dashboard.first, dashboard.last)}`}
    />
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
