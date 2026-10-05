<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";

  let { path, onchoose }: { path: string | null; onchoose: (path: string | null) => void } = $props();

  async function chooseFile() {
    const picked = await open({
      title: "Choose a Hikestat database",
      multiple: false,
      directory: false,
      filters: [{ name: "SQLite database", extensions: ["db", "sqlite", "sqlite3"] }],
    });
    if (picked) onchoose(picked);
  }
</script>

<section>
  <h2>Database</h2>
  <p><code>{path ?? "…"}</code></p>
  <div class="actions">
    <button onclick={chooseFile}>Choose file…</button>
    <button onclick={() => onchoose(null)}>Use default location</button>
  </div>
  <p class="hint">
    The default location is the one the <code>hikestat</code> CLI uses. The <code>HIKESTAT_DB</code>
    variable is ignored by the app.
  </p>
</section>

<style>
  h2 {
    font-size: 1rem;
    margin: 0 0 8px;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .hint {
    color: var(--muted);
    font-size: 0.9em;
  }
</style>
