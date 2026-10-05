<script lang="ts">
  import { commands, type HikeDetails } from "../bindings";
  import {
    MISSING,
    displayName,
    formatDateTime,
    formatDistance,
    formatDuration,
    formatElevation,
    formatSpeed,
    orMissing,
  } from "./format";

  // Panel of one hike. `version` changes on every reload: the hike is fetched again.
  // `onupdated` receives the hike after each saved edit.
  let {
    id,
    version,
    onclose,
    onupdated,
  }: { id: number; version: number; onclose: () => void; onupdated: (hike: HikeDetails) => void } = $props();

  let hike = $state<HikeDetails | null>(null);
  let failure = $state<string | null>(null);
  let closeButton = $state<HTMLButtonElement | null>(null);

  // Only the latest request may update the panel (another hike or a reload can overlap).
  let latest = 0;

  $effect(() => {
    void version;
    const request = ++latest;
    commands.hike(id).then((result) => {
      if (request !== latest) return;
      if (result.status === "ok") {
        if (hike?.id !== result.data.id) titleSave = noteSave = "idle";
        hike = result.data;
        titleDraft = hike.title ?? "";
        noteDraft = hike.note ?? "";
        failure = null;
      } else {
        hike = null;
        failure = result.error;
      }
    });
  });

  $effect(() => {
    closeButton?.focus();
  });

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      // Blurring a title or note field saves it before the panel goes away.
      (document.activeElement as HTMLElement | null)?.blur();
      onclose();
    }
  }

  // --- Title and note: edited in place, saved on blur (and the title on Enter). ---

  type SaveState = "idle" | "saving" | "saved" | { error: string };

  let titleDraft = $state("");
  let noteDraft = $state("");
  let titleSave = $state<SaveState>("idle");
  let noteSave = $state<SaveState>("idle");

  async function save(field: "title" | "note") {
    if (hike === null) return;
    const saved = hike;
    const draft = field === "title" ? titleDraft : noteDraft;
    // Stored values are already trimmed: only a real change is written.
    if (draft.trim() === ((field === "title" ? saved.title : saved.note) ?? "")) return;
    const setState = (state: SaveState) => (field === "title" ? (titleSave = state) : (noteSave = state));
    setState("saving");
    const result =
      field === "title" ? await commands.updateTitle(saved.id, draft) : await commands.updateNote(saved.id, draft);
    // The panel may show another hike by now: the list is still told.
    if (result.status === "ok") onupdated(result.data);
    if (hike?.id !== saved.id) return;
    if (result.status === "ok") {
      hike = result.data;
      if (field === "title") titleDraft = result.data.title ?? "";
      else noteDraft = result.data.note ?? "";
      setState("saved");
    } else {
      setState({ error: result.error });
    }
  }

  function onTitleKeydown(event: KeyboardEvent & { currentTarget: HTMLInputElement }) {
    if (event.key === "Enter") {
      event.preventDefault();
      event.currentTarget.blur();
    }
  }

  function saveStateText(state: SaveState): string {
    if (state === "idle") return "";
    if (state === "saving") return "Saving…";
    if (state === "saved") return "Saved";
    return `Not saved: ${state.error}`;
  }

  // --- End of title and note. ---

  const stats = $derived(
    hike === null
      ? []
      : [
          ["Distance", orMissing(hike.distanceM, formatDistance)],
          ["Ascent", formatElevation(hike.ascentM)],
          ["Descent", formatElevation(hike.descentM)],
          ["Min elevation", formatElevation(hike.minEleM)],
          ["Max elevation", formatElevation(hike.maxEleM)],
          ["Moving time", formatDuration(hike.durationMovingS)],
          ["Total time", formatDuration(hike.durationTotalS)],
          ["Average speed", formatSpeed(hike.avgSpeedMps)],
        ],
  );
</script>

<svelte:window onkeydown={onKeydown} />

<!-- Clicking outside the panel closes it, like Escape. -->
<div class="backdrop" aria-hidden="true" onclick={onclose}></div>

<div class="panel" role="dialog" aria-modal="true" aria-labelledby="hike-panel-title">
  <header>
    <h2 id="hike-panel-title">{hike ? displayName(hike) : "Hike"}</h2>
    <button bind:this={closeButton} class="close" aria-label="Close" onclick={onclose}>✕</button>
  </header>

  {#if failure}
    <p class="message danger">{failure}</p>
  {:else if hike === null}
    <p class="muted">Loading…</p>
  {:else}
    <p class="start">{formatDateTime(hike.startedAt, hike.startTz)}</p>

    <!-- Title and note: saved on blur. -->
    <div class="edit">
      <div class="field">
        <div class="label">
          <label for="hike-title">Title</label>
          <span class="save-state" class:danger-text={typeof titleSave === "object"} aria-live="polite">
            {saveStateText(titleSave)}
          </span>
        </div>
        <input
          id="hike-title"
          type="text"
          placeholder={hike.name ?? "Untitled"}
          bind:value={titleDraft}
          oninput={() => (titleSave = "idle")}
          onblur={() => save("title")}
          onkeydown={onTitleKeydown}
        />
        <p class="raw-name">GPX name: {hike.name ?? MISSING}</p>
      </div>
      <div class="field">
        <div class="label">
          <label for="hike-note">Note</label>
          <span class="save-state" class:danger-text={typeof noteSave === "object"} aria-live="polite">
            {saveStateText(noteSave)}
          </span>
        </div>
        <textarea
          id="hike-note"
          rows="4"
          bind:value={noteDraft}
          oninput={() => (noteSave = "idle")}
          onblur={() => save("note")}
        ></textarea>
      </div>
    </div>
    <!-- End of title and note. -->

    <dl>
      {#each stats as [label, value] (label)}
        <div>
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      {/each}
    </dl>

    <div class="map" aria-label="Map (coming soon)">Map — coming soon</div>
  {/if}
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.25);
    z-index: 10;
  }
  .panel {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(400px, 100%);
    box-sizing: border-box;
    overflow-y: auto;
    padding: 16px 20px;
    background: var(--bg);
    border-left: 1px solid var(--border);
    box-shadow: -4px 0 16px rgb(0 0 0 / 0.15);
    z-index: 11;
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  h2 {
    font-size: 1.2rem;
    margin: 0;
    overflow-wrap: anywhere;
  }
  .close {
    flex: none;
    border: none;
    background: none;
    padding: 0 4px;
    font-size: 1.1rem;
    color: var(--muted);
  }
  .start {
    color: var(--muted);
    margin: 4px 0 16px;
  }
  /* Title and note. */
  .edit {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 0 0 20px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .label {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  .label label {
    color: var(--muted);
    font-size: 0.78rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .save-state {
    color: var(--muted);
    font-size: 0.78rem;
    text-align: right;
  }
  .danger-text {
    color: var(--danger);
  }
  .field input,
  .field textarea {
    box-sizing: border-box;
    width: 100%;
    font: inherit;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 8px;
  }
  .field textarea {
    resize: vertical;
  }
  .raw-name {
    margin: 0;
    color: var(--muted);
    font-size: 0.85rem;
    overflow-wrap: anywhere;
  }
  /* End of title and note. */
  dl {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px 16px;
    margin: 0 0 20px;
  }
  dt {
    color: var(--muted);
    font-size: 0.78rem;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  dd {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .map {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 220px;
    border: 1px dashed var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--muted);
  }
  .muted {
    color: var(--muted);
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
