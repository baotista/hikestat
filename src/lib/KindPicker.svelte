<script lang="ts">
  import { commands, type KindFilter } from "../bindings";

  // The dashboard's kind filter: All, Not set, then each kind of the database.
  // `version` changes on every reload (focus, ⌘R): the kinds are fetched again, and a kind that
  // is no longer in the database (re-import, edit) falls back to All.
  let { value = $bindable({ filter: "all" }), version }: { value?: KindFilter; version: number } = $props();

  let kinds = $state<string[]>([]);
  let failure = $state<string | null>(null);

  // Only the latest request may update the list (reloads can overlap).
  let latest = 0;

  $effect(() => {
    void version;
    const id = ++latest;
    commands.kinds().then((result) => {
      if (id !== latest) return;
      if (result.status === "ok") {
        kinds = result.data;
        failure = null;
        if (value.filter === "kind" && !kinds.includes(value.kind)) value = { filter: "all" };
      } else {
        failure = result.error;
      }
    });
  });

  // A <select> holds strings: "all", "notSet" or "kind:<name>".
  const keyOf = (f: KindFilter) => (f.filter === "kind" ? `kind:${f.kind}` : f.filter);

  function onchange(event: Event) {
    const key = (event.currentTarget as HTMLSelectElement).value;
    value = key.startsWith("kind:")
      ? { filter: "kind", kind: key.slice("kind:".length) }
      : { filter: key === "notSet" ? "notSet" : "all" };
  }
</script>

<label class="kind" title={failure ? `Cannot list the kinds: ${failure}` : undefined}>
  <span>Kind</span>
  <select value={keyOf(value)} {onchange}>
    <option value="all">All</option>
    <option value="notSet">Not set</option>
    {#each kinds as kind (kind)}
      <option value={`kind:${kind}`}>{kind}</option>
    {/each}
    {#if value.filter === "kind" && !kinds.includes(value.kind)}
      <!-- Until the kinds are listed (or if listing them failed), keep the current choice visible. -->
      <option value={keyOf(value)}>{value.kind}</option>
    {/if}
  </select>
</label>

<style>
  .kind {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 10px;
    color: var(--muted);
  }
  select {
    font: inherit;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 2px 8px;
    max-width: 50%;
  }
</style>
