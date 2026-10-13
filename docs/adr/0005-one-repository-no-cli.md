# One repository, no CLI

Since ADR 0004 the app imports GPX itself, so the `hikestat` CLI and the separate `hikestat` library (repository `hikestats-ingester`) no longer earned their cost: a git dependency fetched over SSH, a library tag to bump and keep in step with the CLI, and every change split across two repositories. The library's code now lives in the app, in `src-tauri/src/domain/`, with its tests in `src-tauri/tests/`, and the CLI is gone. `hikestats-ingester` is archived.

## Consequences

- Supersedes ADR 0002 and ADR 0004: the app alone owns the schema and imports GPX. There is no "same library tag" rule anymore.
- The database keeps its path and schema: an existing `hikestat.db` opens as before.
- What only the CLI did is gone: `--kind`, `--dry-run`, `--db`, `HIKESTAT_DB` and the `recompute` command. `Db::recompute_stats` stays, but nothing calls it yet: the next `STATS_VERSION` bump must have the app run it.
