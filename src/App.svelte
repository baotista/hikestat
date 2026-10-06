<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { commands, type DbStatus, type KindFilter, type View } from "./bindings";
  import Dashboard from "./lib/Dashboard.svelte";
  import ImportBanner, { type ImportBannerState } from "./lib/ImportBanner.svelte";
  import Settings from "./lib/Settings.svelte";
  import type { Metric } from "./lib/format";

  let status = $state<DbStatus | null>(null);
  let failure = $state<string | null>(null);
  let screen = $state<"dashboard" | "settings">("dashboard");
  // Bumped on every successful reload so the dashboard fetches its data again.
  let version = $state(0);
  // Window shown by the dashboard, kept here so that it survives a visit to Settings.
  let anchor = $state<string | null>(null);
  // Same for the view; the app opens on the week.
  let view = $state<View>("week");
  // And for the charted metric; distance by default.
  let metric = $state<Metric>("distance");
  // Same for the kind filter; the app opens on All.
  let kindFilter = $state<KindFilter>({ filter: "all" });

  // Only the latest request may update the screen (focus and ⌘R can overlap).
  let latest = 0;

  async function apply(request: ReturnType<typeof commands.dbStatus>) {
    const id = ++latest;
    const result = await request;
    if (id !== latest) return;
    if (result.status === "ok") {
      status = result.data;
      failure = null;
      version++;
    } else {
      failure = result.error;
    }
  }

  const refresh = () => apply(commands.dbStatus());
  const choose = (path: string | null) => apply(commands.chooseDb(path));

  function onKeydown(event: KeyboardEvent) {
    // ⌘R re-fetches the data instead of reloading the webview (which would reset the UI state).
    if (event.metaKey && event.key.toLowerCase() === "r") {
      event.preventDefault();
      refresh();
    }
  }

  // --- Import by drag and drop (ADR 0004) --------------------------------------------------------
  // Files and folders dropped anywhere on the window, on every screen, are imported through the
  // library with the CLI's rules. Tauri's drag and drop events carry the paths.

  let dragging = $state(false);
  let importing = $state(false);
  let banner = $state<ImportBannerState | null>(null);

  async function importDropped(paths: string[]) {
    if (importing) {
      banner = { notice: "An import is already running" };
      return;
    }
    importing = true;
    const result = await commands.import(paths);
    importing = false;
    banner = result.status === "ok" ? { report: result.data } : { error: result.error };
    // Reload like after an edit: status, dashboard, kinds and hike list.
    refresh();
  }
  // --- End of import by drag and drop ------------------------------------------------------------

  onMount(() => {
    refresh();
    const unlistenFocus = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) refresh();
    });
    const unlistenDrop = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "enter" || payload.type === "over") dragging = true;
      else if (payload.type === "leave") dragging = false;
      else {
        dragging = false;
        void importDropped(payload.paths);
      }
    });
    return () => {
      unlistenFocus.then((stop) => stop());
      unlistenDrop.then((stop) => stop());
    };
  });
</script>

<svelte:window onkeydown={onKeydown} />

<header>
  <h1>Hikestat</h1>
  <nav>
    <button aria-pressed={screen === "dashboard"} onclick={() => (screen = "dashboard")}>Dashboard</button>
    <button aria-pressed={screen === "settings"} onclick={() => (screen = "settings")}>Settings</button>
  </nav>
</header>

<main>
  {#if failure}
    <p class="message danger">{failure}</p>
  {/if}

  {#if screen === "settings"}
    <Settings path={status?.path ?? null} onchoose={choose} />
  {:else if status === null}
    <p class="muted">Loading…</p>
  {:else if status.state === "ready"}
    <!-- An edit reloads like focus or ⌘R: same window, filter and selection, fresh data. -->
    <Dashboard {version} bind:anchor bind:view bind:metric bind:kindFilter onchanged={refresh} />
  {:else if status.state === "missing"}
    <!-- The first drop creates the database at this path; refresh() then shows the dashboard. -->
    <p class="message warning">
      No database yet at <code>{status.path}</code>. Drop GPX files or folders anywhere on the window
      to create it and import them — or run <code>hikestat import</code>, or
      <button class="link" onclick={() => (screen = "settings")}>choose a file</button>.
    </p>
  {:else if status.state === "tooNew"}
    <p class="message warning">
      This database was written by a newer version of <code>hikestat</code>. Update Hikestat to open it.
    </p>
  {:else}
    <p class="message danger">Cannot open the database: {status.message}</p>
  {/if}
</main>

{#if banner}
  <ImportBanner state={banner} onclose={() => (banner = null)} />
{/if}

{#if dragging || importing}
  <div class="drop-overlay" aria-live="polite">
    <p>{importing ? "Importing…" : "Drop GPX files to import"}</p>
  </div>
{/if}

<footer>
  Database: <code>{status?.path ?? "…"}</code>
  {#if status?.state === "ready"}
    · {status.hikeCount} {status.hikeCount === 1 ? "hike" : "hikes"}
  {/if}
</footer>

<style>
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
  }
  h1 {
    font-size: 1.2rem;
    margin: 0;
  }
  nav {
    display: flex;
    gap: 6px;
  }
  nav button[aria-pressed="true"] {
    border-color: var(--accent);
    color: var(--accent);
  }
  main {
    padding: 20px 20px 60px;
  }
  footer {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    padding: 8px 20px;
    border-top: 1px solid var(--border);
    background: var(--surface);
    color: var(--muted);
  }
  .message {
    padding: 12px 16px;
    border-radius: 8px;
    background: var(--surface);
  }
  .warning {
    border-left: 4px solid var(--warning);
  }
  .danger {
    border-left: 4px solid var(--danger);
  }
  .muted {
    color: var(--muted);
  }
  /* Over everything, the hike panel and the expanded map included. */
  .drop-overlay {
    position: fixed;
    inset: 0;
    z-index: 30;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgb(0 0 0 / 0.35);
    pointer-events: none;
  }
  .drop-overlay p {
    margin: 0;
    padding: 24px 40px;
    border: 2px dashed var(--accent);
    border-radius: 12px;
    background: var(--bg);
    font-size: 1.1rem;
    font-weight: 600;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    text-decoration: underline;
  }
</style>
