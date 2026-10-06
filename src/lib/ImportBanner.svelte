<script lang="ts">
  import type { ImportReport, KindFilter } from "../bindings";
  import { hiddenByFilter, importDetails, importSummary } from "./importText";

  // What the last drop did; stays until closed or replaced by the next drop.
  // `filter`: the kind filter the dashboard had when the import ran.
  export type ImportBannerState = { report: ImportReport; filter: KindFilter } | { error: string } | { notice: string };

  let { state, onclose }: { state: ImportBannerState; onclose: () => void } = $props();

  const text = $derived(
    "report" in state
      ? [importSummary(state.report), hiddenByFilter(state.report, state.filter)].filter(Boolean).join(" · ") : "error" in state ? `Import failed: ${state.error}` : state.notice,
  );
  // Rejections and warnings, behind a "Details" disclosure (closed by default).
  const details = $derived("report" in state ? importDetails(state.report) : []);
</script>

<div class="banner" class:danger={"error" in state} role="status">
  <div class="summary">
    <span>{text}</span>
    <button class="close" aria-label="Close" onclick={onclose}>✕</button>
  </div>
  {#if details.length > 0}
    <details>
      <summary>Details</summary>
      <ul>
        {#each details as line, i (i)}
          <li>{line}</li>
        {/each}
      </ul>
    </details>
  {/if}
</div>

<style>
  /* Above the hike panel and the expanded map, below the drop overlay. */
  .banner {
    position: fixed;
    top: 56px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 25;
    max-width: calc(100% - 40px);
    box-sizing: border-box;
    padding: 8px 8px 8px 16px;
    border-radius: 8px;
    border-left: 4px solid var(--accent);
    background: var(--surface);
    box-shadow: 0 2px 12px rgb(0 0 0 / 0.2);
  }
  .summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  details {
    margin: 4px 8px 4px 0;
    color: var(--muted);
    font-size: 0.85rem;
  }
  summary {
    cursor: pointer;
  }
  /* A long list scrolls inside the banner rather than covering the window. */
  ul {
    margin: 4px 0 0;
    padding-left: 18px;
    max-height: 200px;
    overflow-y: auto;
    overflow-wrap: anywhere;
  }
  .banner.danger {
    border-left-color: var(--danger);
  }
  .close {
    flex: none;
    border: none;
    background: none;
    padding: 0 4px;
    color: var(--muted);
  }
</style>
