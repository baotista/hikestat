# Hikestat

Hikestat is a local, single-user macOS desktop app to look at the hikes imported by the
[`hikestat` CLI](https://github.com/baotista/hikestats-ingester) (the ingester). It opens the same
SQLite database as the CLI, through the same `hikestat` library, and never imports GPX itself.

For now the app shows the dashboard's total distance and bar chart in four views: W (an ISO week,
one bar per day), M (a month, one bar per day), Y (a year, one bar per month) and All (one bar per
year, from the year of the first hike to the current year), with ‹ › to move between weeks, months
or years, plus the database it uses and whether it can be opened. It opens on the current week. Built with [Tauri](https://v2.tauri.app) v2 and Svelte 5 + TypeScript (see
[ADR 0001](docs/adr/0001-tauri-with-svelte-frontend.md)).

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
`src-tauri/Cargo.toml`; temporarily a commit `rev`, until the library's read API is tagged
`v0.3.0`). The library owns the database schema and the app applies its migrations
when it opens the database ([ADR 0002](docs/adr/0002-shared-database-owned-by-the-library.md)):
an app built on a newer tag would migrate the database, and an older CLI would then refuse to write
to it. **Always install the app and the `hikestat` CLI from the same library tag**, for example:

```sh
cargo install --git ssh://git@github.com/baotista/hikestats-ingester.git --tag v0.3.0
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

The app never creates the database: if there is no file at the path, it tells you to run
`hikestat import` or to choose a file. The path in use is always shown at the bottom of the window.
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
