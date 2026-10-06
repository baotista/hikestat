<script lang="ts">
  import type { ImportReport } from "../bindings";
  import { importSummary } from "./importText";

  // What the last drop did; stays until closed or replaced by the next drop.
  export type ImportBannerState = { report: ImportReport } | { error: string } | { notice: string };

  let { state, onclose }: { state: ImportBannerState; onclose: () => void } = $props();

  const text = $derived(
    "report" in state ? importSummary(state.report) : "error" in state ? `Import failed: ${state.error}` : state.notice,
  );
</script>

<div class="banner" class:danger={"error" in state} role="status">
  <span>{text}</span>
  <button class="close" aria-label="Close" onclick={onclose}>✕</button>
</div>

<style>
  /* Above the hike panel and the expanded map, below the drop overlay. */
  .banner {
    position: fixed;
    top: 56px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 25;
    display: flex;
    align-items: center;
    gap: 12px;
    max-width: calc(100% - 40px);
    box-sizing: border-box;
    padding: 8px 8px 8px 16px;
    border-radius: 8px;
    border-left: 4px solid var(--accent);
    background: var(--surface);
    box-shadow: 0 2px 12px rgb(0 0 0 / 0.2);
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
