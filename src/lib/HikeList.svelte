<script lang="ts">
  import { commands, type HikeDetails } from "../bindings";
  import HikePanel from "./HikePanel.svelte";
  import {
    displayName,
    formatDistance,
    formatDuration,
    formatElevation,
    formatHikeCount,
    formatLocalDate,
    formatSpeed,
    orMissing,
  } from "./format";
  import { DEFAULT_SORT, type HikeSort, type SortColumn, listHikes, nextSort } from "./hikeList";

  // The list of hikes under the chart: the window's hikes, or the selected bar's, fetched from their
  // `hikeIds`. `version` changes on every reload (focus, ⌘R, import, edit): the hikes are fetched
  // again. `onchanged` is called after the panel saved an edit. `search` narrows the list only (never
  // the chart) and `sort` orders it; both are owned by the parent so they survive a new window, bar
  // or a visit to Settings.
  let {
    title,
    ids,
    version,
    search = $bindable(""),
    onchanged,
    sort = $bindable(DEFAULT_SORT),
  }: { title: string; ids: number[]; version: number; search?: string; onchanged?: () => void; sort?: HikeSort } =
    $props();

  let hikes = $state<HikeDetails[] | null>(null);
  let failure = $state<string | null>(null);
  // Hike shown in the panel.
  let opened = $state<number | null>(null);

  // Only the latest request may update the list (a new window, bar or reload can overlap).
  let latest = 0;

  $effect(() => {
    void version;
    const id = ++latest;
    if (ids.length === 0) {
      hikes = [];
      failure = null;
      return;
    }
    commands.hikes(ids).then((result) => {
      if (id !== latest) return;
      if (result.status === "ok") {
        hikes = result.data;
        failure = null;
      } else {
        failure = result.error;
      }
    });
  });

  const listing = $derived(hikes === null ? null : listHikes(hikes, { search, sort }));

  const COLUMNS: { column: SortColumn; label: string; number: boolean }[] = [
    { column: "name", label: "Name", number: false },
    { column: "date", label: "Date", number: false },
    { column: "distance", label: "Distance", number: true },
    { column: "ascent", label: "Ascent", number: true },
    { column: "movingTime", label: "Moving time", number: true },
    { column: "avgSpeed", label: "Avg speed", number: true },
  ];

  function ariaSort(column: SortColumn): "ascending" | "descending" | "none" {
    if (sort.column !== column) return "none";
    return sort.descending ? "descending" : "ascending";
  }

  // An edit saved in the panel (title, note) replaces the listed hike, without refetching.
  function onupdated(updated: HikeDetails) {
    if (hikes) hikes = hikes.map((h) => (h.id === updated.id ? updated : h));
  }
</script>

<section class="list" aria-label={`Hikes of ${title}`}>
  <div class="heading">
    <h2>{title}</h2>
    {#if listing && listing.total > 0}
      <span class="count">
        {listing.shown < listing.total ? `${listing.shown} of ${formatHikeCount(listing.total)}` : formatHikeCount(listing.total)}
      </span>
    {/if}
  </div>

  <!-- Search: filters as you type, over the name, GPX name, note and kind. -->
  <div class="search">
    <input type="search" placeholder="Search name, note or kind" aria-label="Search hikes" bind:value={search} />
    {#if search !== ""}
      <button class="clear" aria-label="Clear search" onclick={() => (search = "")}>✕</button>
    {/if}
  </div>
  <!-- End of search. -->

  {#if failure}
    <p class="message danger">{failure}</p>
  {:else if listing === null}
    <p class="muted">Loading…</p>
  {:else if listing.total === 0}
    <p class="empty">No hikes.</p>
  {:else if listing.shown === 0}
    <p class="empty">No hikes match.</p>
  {:else}
    <div class="table">
      <!-- Header and rows share the same grid columns. A header sorts by its column; again, reverses. -->
      <div class="columns" role="row">
        {#each COLUMNS as { column, label, number } (column)}
          <span class:number role="columnheader" aria-sort={ariaSort(column)}>
            <button class="sort" class:active={sort.column === column} onclick={() => (sort = nextSort(sort, column))}>
              {label}{#if sort.column === column}<span class="arrow" aria-hidden="true"
                  >{sort.descending ? "↓" : "↑"}</span
                >{/if}
            </button>
          </span>
        {/each}
      </div>
      <ul>
        {#each listing.hikes as hike (hike.id)}
          <li>
            <button class="row" aria-pressed={opened === hike.id} onclick={() => (opened = hike.id)}>
              <span class="name">{displayName(hike)}</span>
              <span class="date">{formatLocalDate(hike.startedAt, hike.startTz)}</span>
              <span class="number" title="Distance">{orMissing(hike.distanceM, formatDistance)}</span>
              <span class="number" title="Ascent">{formatElevation(hike.ascentM)}</span>
              <span class="number" title="Moving time">{formatDuration(hike.durationMovingS)}</span>
              <span class="number" title="Average speed">{formatSpeed(hike.avgSpeedMps)}</span>
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</section>

{#if opened !== null}
  <HikePanel id={opened} {version} onclose={() => (opened = null)} {onupdated} {onchanged} />
{/if}

<style>
  .list {
    margin-top: 24px;
  }
  .heading {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    margin: 0 0 8px;
  }
  h2 {
    font-size: 1rem;
    margin: 0;
  }
  .count {
    color: var(--muted);
    font-size: 0.9em;
  }
  /* Search. */
  .search {
    position: relative;
    margin: 0 0 8px;
  }
  .search input {
    box-sizing: border-box;
    width: 100%;
    font: inherit;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 32px 6px 10px;
  }
  .search input:focus {
    outline: none;
    border-color: var(--accent);
  }
  /* Our own clear button, not the webview's. */
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  .clear {
    position: absolute;
    top: 50%;
    right: 6px;
    transform: translateY(-50%);
    border: none;
    background: none;
    padding: 0 4px;
    color: var(--muted);
  }
  /* End of search. */
  .table {
    border-radius: 8px;
    background: var(--surface);
    overflow: hidden;
  }
  /* Name takes the rest; the other columns fit their widest value. */
  .columns,
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 7.5rem 5.5rem 4.5rem 5.5rem 6rem;
    column-gap: 12px;
    padding: 8px 12px;
  }
  .columns {
    color: var(--muted);
    font-size: 0.78rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    border-bottom: 1px solid var(--border);
  }
  .sort {
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
  }
  .sort:hover,
  .sort.active {
    color: var(--text);
  }
  .arrow {
    margin-left: 4px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li + li {
    border-top: 1px solid var(--border);
  }
  .row {
    width: 100%;
    text-align: left;
    border: none;
    border-radius: 0;
    background: transparent;
    font-variant-numeric: tabular-nums;
  }
  .row:hover,
  .row[aria-pressed="true"] {
    background: var(--grid);
  }
  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .date {
    color: var(--muted);
    white-space: nowrap;
  }
  .number {
    text-align: right;
    white-space: nowrap;
  }
  .empty,
  .muted {
    color: var(--muted);
  }
  .empty {
    padding: 12px 16px;
    border-radius: 8px;
    background: var(--surface);
    margin: 0;
  }
  .message {
    padding: 12px 16px;
    border-radius: 8px;
    background: var(--surface);
  }
  .danger {
    border-left: 4px solid var(--danger);
  }
</style>
