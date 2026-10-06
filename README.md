# Hikestat

Hikestat is a local, single-user macOS desktop app to look at your hikes. It opens the same SQLite
database as the [`hikestat` CLI](https://github.com/baotista/hikestats-ingester) (the ingester),
through the same `hikestat` library, and imports GPX through that library too
([ADR 0004](docs/adr/0004-the-app-imports-gpx-through-the-library.md)).

Drop GPX files or folders anywhere on the window to import them, exactly as `hikestat import` would
(same deduplication, rejections and warnings; a folder imports the `.gpx` files directly inside it;
anything else is rejected as "not a .gpx file"). An overlay shows while dragging and while
importing; a banner then sums up the import (imported, already present, rejected) and the data
reloads. A second drop during an import is refused.

For now the app shows the dashboard's total and bar chart of a chosen metric (distance, ascent,
moving time or number of hikes; with ascent, the header also says how many hikes have no elevation
data), filtered by kind (all, not set, or one kind), in four views: W (an ISO week,
one bar per day), M (a month, one bar per day), Y (a year, one bar per month) and All (one bar per
year, from the year of the first hike to the current year), with ‹ › to move between weeks, months
or years, plus the database it uses and whether it can be opened. It opens on the current week. Built with [Tauri](https://v2.tauri.app) v2 and Svelte 5 + TypeScript (see
[ADR 0001](docs/adr/0001-tauri-with-svelte-frontend.md)).

In a hike's panel, the kind can be edited (free text, suggesting the kinds already in the
database; blank sets it back to Not set). It is saved on blur or Enter, and the dashboard and the
kind filter reload right away.

Below the stats, the panel shows the hike's track on a map ([MapLibre GL](https://maplibre.org)):
one line per segment, coloured by smoothed speed (from the hike's slow to its fast parts, 5th to 95th
centile, with a legend in km/h; grey where stopped or without time; in the accent colour if the hike
never moves), a grey dashed line across each gap between segments (where nothing was recorded), and
start (filled dot) and end (ringed dot) markers, and a numbered badge at each kilometre (counted
like the stats' distance; zoomed out, only every 5th or 10th, and overlapping badges are hidden). The wheel scrolls the panel; ⌘ + wheel or a
pinch zooms the map. ⤢ expands it to the whole window (same framing, the wheel zooms); ✕ or Escape
brings back the panel, and a second Escape closes the panel. The background comes from
[OpenFreeMap](https://openfreemap.org) (no key), light or dark like macOS (the dark style's paths,
roads, water and names are lightened to stay readable), so it needs the network:
offline, the track is drawn on a plain background and the next reload (focus, ⌘R) tries again. A reload keeps the map's framing; opening another hike
frames its track.

## Prerequisites

- Rust 1.85 or newer (2024 edition)
- Node.js 24 and [pnpm](https://pnpm.io)
- The [Tauri prerequisites for macOS](https://v2.tauri.app/start/prerequisites/) (Xcode Command
  Line Tools)
- SSH access to `github.com/baotista/hikestats-ingester`: the library is a git dependency fetched
  over SSH. `src-tauri/.cargo/config.toml` sets `net.git-fetch-with-cli = true`, so Cargo uses
  your `git` and its SSH setup (keychain, `~/.ssh/config`) instead of only trying the SSH agent.

## Build and run

```sh
pnpm install          # frontend dependencies and the Tauri CLI
pnpm tauri dev        # run the app in development mode
pnpm tauri build      # build src-tauri/target/release/bundle/macos/Hikestat.app
```

Other checks:

```sh
pnpm check                            # svelte-check (TypeScript)
pnpm test                             # vitest (formatting helpers)
pnpm build                            # frontend only
cd src-tauri && cargo test            # Rust unit tests (also regenerates src/bindings.ts)
cd src-tauri && cargo clippy --all-targets
```

The TypeScript types and command wrappers in `src/bindings.ts` are generated from the Rust
commands by [`tauri-specta`](https://github.com/specta-rs/tauri-specta) in debug builds (and by
`cargo test`). The file is committed: don't edit it by hand.

## Looking at the dashboard without your data

`src-tauri/examples/demo_db.rs` creates a throwaway database with a few made-up hikes last week and
this week (one in Tokyo, one crossing midnight, one without elevation):

```sh
cd src-tauri && cargo run --example demo_db -- /tmp/hikestat-demo.db
```

Run the app, then **Settings → Choose file…** and pick `/tmp/hikestat-demo.db` (**Use default
location** switches back). Note that the app opens the default database on start, before you can
choose the demo file.

## Install the app and the CLI from the same library tag

The app depends on the `hikestat` library through a git dependency pinned to a tag (see
`src-tauri/Cargo.toml`). The library owns the database schema and the app applies its migrations
when it opens the database ([ADR 0002](docs/adr/0002-shared-database-owned-by-the-library.md)):
an app built on a newer tag would migrate the database, and an older CLI would then refuse to write
to it. **Always install the app and the `hikestat` CLI from the same library tag**, for example:

```sh
cargo install --git ssh://git@github.com/baotista/hikestats-ingester.git --tag v0.10.0
```

If this fails with `failed to authenticate when downloading repository` (`attempted ssh-agent
authentication`), Cargo's built-in SSH client only tried the SSH agent. Outside this repository,
`src-tauri/.cargo/config.toml` doesn't apply: make Cargo use your `git` for this command with
`CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install …`, or for good in `~/.cargo/config.toml`:

```toml
[net]
git-fetch-with-cli = true
```

If the database is newer than the library bundled in the app, the app says so, writes nothing, and
asks you to update Hikestat.

## Database location

The app uses, in order:

1. the file chosen in **Settings → Choose file…** (remembered in `settings.json` in the app's config
   directory, `~/Library/Application Support/fr.baotista.hikestat/`);
2. otherwise the library's default path, the same as the CLI's:
   `~/Library/Application Support/hikestat/hikestat.db`.

**`HIKESTAT_DB` is ignored by the app**: GUI apps don't inherit shell variables. If you set it in
your shell, move your database to the default path and drop the variable, so that the CLI and the
app agree with no configuration (or choose the file in the app's settings). The CLI still honours
`HIKESTAT_DB` and `--db`, which remain handy for tests and throwaway databases.

If there is no file at the path, the app says so and the first import creates it there: drop GPX
files on the window (or run `hikestat import`, or choose another file). Nothing else creates a
database. The path in use is always shown at the bottom of the window.
The data is reloaded when the window regains focus and with ⌘R, so hikes imported from the terminal
show up without restarting the app.

## Developing against a local library checkout

To work on the library and the app together, point Cargo at the sibling checkout with a local,
**uncommitted** `[patch]` in `.cargo/config.toml` at the root of this repository (the file is
git-ignored; its paths are relative to the repository root):

```toml
[patch."ssh://git@github.com/baotista/hikestats-ingester.git"]
hikestat = { path = "../hikestats-ingester" }
```

This rewrites `src-tauri/Cargo.lock`: don't commit that change. Delete the file to go back to the
tagged library.
