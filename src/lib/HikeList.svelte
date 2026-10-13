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
  import { listHikes } from "./hikeList";

  // The list of hikes under the chart: the window's hikes, or the selected bar's, fetched from their
  // `hikeIds`. `version` changes on every reload (focus, ⌘R, import, edit): the hikes are fetched
  // again. `onchanged` is called after the panel saved an edit. `search` narrows the list only (never
  // the chart); it is owned by the parent so it survives a new window, bar or a visit to Settings.
  let {
    title,
    ids,
    version,
    search = $bindable(""),
    onchanged,
  }: { title: string; ids: number[]; version: number; search?: string; onchanged?: () => void } = $props();

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

  const listing = $derived(hikes === null ? null : listHikes(hikes, search));

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
      <!-- Header and rows share the same grid columns. -->
      <div class="columns" aria-hidden="true">
        <span>Name</span>
        <span>Date</span>
        <span class="number">Distance</span>
        <span class="number">Ascent</span>
        <span class="number">Moving time</span>
        <span class="number">Avg speed</span>
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
