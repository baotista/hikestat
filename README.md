# Hikestat

A local, single-user macOS app to look at your hikes. It shares its SQLite database and the
`hikestat` library with the [`hikestat` CLI](https://github.com/baotista/hikestats-ingester).
Built with [Tauri](https://v2.tauri.app) v2 and Svelte 5 + TypeScript
([ADR 0001](docs/adr/0001-tauri-with-svelte-frontend.md)).

## Features

- **Import**: drop GPX files or folders on the window. The import works exactly like
  `hikestat import`, with the same deduplication and rejections
  ([ADR 0004](docs/adr/0004-the-app-imports-gpx-through-the-library.md)). A banner then sums up
  the import.
- **Dashboard**: shows the total and a bar chart of distance, ascent, moving time or number of
  hikes. Filter it by kind and pick a view: W (a week, by day), M (a month, by day), Y (a year,
  by month) or All (by year). ‹ › move between windows.
- **List of hikes**: lists the hikes of the window, or of the selected bar. Sort it by any column
  and search it by name, note or kind. The header gives the count and the total distance of the
  hikes shown.
- **Hike panel**: shows the stats and an editable kind. It also shows a map of the track
  ([MapLibre GL](https://maplibre.org), [OpenFreeMap](https://openfreemap.org) background, needs
  the network), coloured by speed with km markers. Under the map, an elevation profile is linked
  to it both ways. ⤢ expands the map to the whole window.

The data reloads when the window regains focus and with ⌘R.

## Prerequisites

- Rust 1.85 or newer (2024 edition)
- Node.js 24 and [pnpm](https://pnpm.io)
- The [Tauri prerequisites for macOS](https://v2.tauri.app/start/prerequisites/) (Xcode Command
  Line Tools)
- SSH access to `github.com/baotista/hikestats-ingester`. The library is a git dependency, and
  `src-tauri/.cargo/config.toml` makes Cargo fetch it with your `git` and its SSH setup.

## Build the app

```sh
pnpm install
pnpm tauri build
cp -R src-tauri/target/release/bundle/macos/Hikestat.app /Applications/
```

`pnpm tauri build` builds the release bundle at `src-tauri/target/release/bundle/macos/Hikestat.app`
and the bare binary at `src-tauri/target/release/hikestat-app`. Quit Hikestat and remove the old
`/Applications/Hikestat.app` before copying.

## Develop

```sh
pnpm tauri dev                        # run the app in development mode
pnpm check                            # svelte-check (TypeScript)
pnpm test                             # vitest
cd src-tauri && cargo test            # Rust tests (also regenerates src/bindings.ts)
cd src-tauri && cargo clippy --all-targets
```

`tauri-specta` generates `src/bindings.ts` from the Rust commands. The file is committed: don't
edit it by hand.

To try the app on made-up hikes, create a demo database:

```sh
cd src-tauri && cargo run --example demo_db -- /tmp/hikestat-demo.db
```

Then pick that file in **Settings → Choose file…**.

To work on the library at the same time, add an **uncommitted** `.cargo/config.toml` at the root
of this repository. The file is git-ignored. This changes `src-tauri/Cargo.lock`: don't commit
that change either.

```toml
[patch."ssh://git@github.com/baotista/hikestats-ingester.git"]
hikestat = { path = "../hikestats-ingester" }
```

## Install the CLI from the same library tag

The library owns the schema, and the app migrates the database when it opens it
([ADR 0002](docs/adr/0002-shared-database-owned-by-the-library.md)). A CLI built on an older
library would then refuse to write to the database. **Install the CLI from the tag in
`src-tauri/Cargo.toml`**:

```sh
CARGO_NET_GIT_FETCH_WITH_CLI=true cargo install --git ssh://git@github.com/baotista/hikestats-ingester.git --tag v0.12.0
```

## Database location

1. The file chosen in **Settings → Choose file…**. The choice is saved in
   `~/Library/Application Support/fr.baotista.hikestat/settings.json`.
2. Otherwise, the CLI's default path: `~/Library/Application Support/hikestat/hikestat.db`.

`HIKESTAT_DB` is ignored by the app, because GUI apps don't inherit shell variables. If the file
doesn't exist yet, the first import creates it.
