<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { commands, type DbStatus, type KindFilter, type View } from "./bindings";
  import Dashboard from "./lib/Dashboard.svelte";
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

  onMount(() => {
    refresh();
    const unlisten = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) refresh();
    });
    return () => {
      unlisten.then((stop) => stop());
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
    <p class="message warning">
      No database at <code>{status.path}</code> — run <code>hikestat import</code> or
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
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    text-decoration: underline;
  }
</style>
