# Hikestat

A local, single-user macOS app to look at your hikes: it imports GPX files, computes their
statistics and stores them in a SQLite database. Built with [Tauri](https://v2.tauri.app) v2 and Svelte 5 + TypeScript
([ADR 0001](docs/adr/0001-tauri-with-svelte-frontend.md)).

## Features

- **Import**: drop GPX files or folders on the window. Re-importing never creates duplicates, and
  a banner sums up the import (see [Import, statistics and database](docs/data.md)).
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

## Install

Download `Hikestat-<version>-macos-arm64.zip` (Apple silicon) from the
[latest release](https://github.com/baotista/hikestat/releases/latest), unzip it and move
`Hikestat.app` to `/Applications`.

The app is not notarised by Apple, so macOS blocks the first launch of a downloaded copy. Unblock
it once, either way:

- **In Terminal**: run

  ```sh
  xattr -dr com.apple.quarantine /Applications/Hikestat.app
  ```

  The browser marks downloaded files with the `com.apple.quarantine` attribute, and macOS checks
  the apps that carry it. `xattr -d` deletes that attribute, `-r` from the app and everything inside
  it: the app then opens like any other.
- **In System Settings**: launch Hikestat once and close the warning, then open **System Settings →
  Privacy & Security**, scroll to **Security** and click **Open Anyway** next to Hikestat. Confirm
  with your password; the app then opens normally.

Right-click → **Open** no longer bypasses the check since macOS 15.

## Build prerequisites

- Rust 1.85 or newer (2024 edition)
- Node.js 24 and [pnpm](https://pnpm.io)
- The [Tauri prerequisites for macOS](https://v2.tauri.app/start/prerequisites/) (Xcode Command
  Line Tools)

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

The Tauri commands are in `src-tauri/src/`; the import, statistics and database code is in
`src-tauri/src/domain/`, tested by `src-tauri/tests/`. [`CONTEXT.md`](CONTEXT.md) holds the
vocabulary and the main decisions.

## Database location

1. The file chosen in **Settings → Choose file…**. The choice is saved in
   `~/Library/Application Support/fr.baotista.hikestat/settings.json`.
2. Otherwise, the default path: `~/Library/Application Support/hikestat/hikestat.db`.

If the file doesn't exist yet, the first import creates it.
