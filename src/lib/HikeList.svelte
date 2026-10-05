<script lang="ts">
  import { commands, type HikeDetails } from "../bindings";
  import HikePanel from "./HikePanel.svelte";
  import {
    displayName,
    formatDistance,
    formatDuration,
    formatElevation,
    formatLocalDate,
    orMissing,
  } from "./format";

  // The hikes of one bucket (a bar of the chart), fetched from its `hikeIds`.
  // `version` changes on every reload (focus, ⌘R): the hikes are fetched again.
  let { title, ids, version }: { title: string; ids: number[]; version: number } = $props();

  let hikes = $state<HikeDetails[] | null>(null);
  let failure = $state<string | null>(null);
  // Hike shown in the panel.
  let opened = $state<number | null>(null);

  // Only the latest request may update the list (a new bar or a reload can overlap).
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

  // An edit saved in the panel (title, note) replaces the listed hike, without refetching.
  function onupdated(updated: HikeDetails) {
    if (hikes) hikes = hikes.map((h) => (h.id === updated.id ? updated : h));
  }
</script>

<section class="list" aria-label={`Hikes of ${title}`}>
  <h2>{title}</h2>

  {#if failure}
    <p class="message danger">{failure}</p>
  {:else if hikes === null}
    <p class="muted">Loading…</p>
  {:else if hikes.length === 0}
    <p class="empty">No hikes.</p>
  {:else}
    <ul>
      {#each hikes as hike (hike.id)}
        <li>
          <button class="row" aria-pressed={opened === hike.id} onclick={() => (opened = hike.id)}>
            <span class="name">{displayName(hike)}</span>
            <span class="date">{formatLocalDate(hike.startedAt, hike.startTz)}</span>
            <span class="stats">
              <span title="Distance">{orMissing(hike.distanceM, formatDistance)}</span>
              <span title="Ascent">↑ {formatElevation(hike.ascentM)}</span>
              <span title="Moving time">{formatDuration(hike.durationMovingS)}</span>
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

{#if opened !== null}
  <HikePanel id={opened} {version} onclose={() => (opened = null)} {onupdated} />
{/if}

<style>
  .list {
    margin-top: 24px;
  }
  h2 {
    font-size: 1rem;
    margin: 0 0 8px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    border-radius: 8px;
    background: var(--surface);
    overflow: hidden;
  }
  li + li {
    border-top: 1px solid var(--border);
  }
  .row {
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-areas: "name stats" "date stats";
    column-gap: 16px;
    width: 100%;
    text-align: left;
    border: none;
    border-radius: 0;
    background: transparent;
    padding: 8px 12px;
  }
  .row:hover,
  .row[aria-pressed="true"] {
    background: var(--grid);
  }
  .name {
    grid-area: name;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .date {
    grid-area: date;
    color: var(--muted);
    font-size: 0.9em;
  }
  .stats {
    grid-area: stats;
    align-self: center;
    display: flex;
    gap: 14px;
    font-variant-numeric: tabular-nums;
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
