# One shared SQLite database, owned by the `hikestat` library

> Partly superseded by [ADR 0004](0004-the-app-imports-gpx-through-the-library.md): the app now imports GPX (through the library) and creates the database on its first import. The rest still holds.

The app and the ingester CLI read and write the same SQLite file. Neither owns the schema: the `hikestat` library (in `hikestats-ingester`) does, and the app depends on it through a git dependency pinned to a tag. The app never imports GPX; it only writes user edits (title, note, kind) into columns the importer and `recompute` never touch, and the raw track name and points stay immutable. We chose this over a separate app database or a copy of the schema in the app because one file means no synchronisation, and one schema owner means no silent drift.

## Consequences

- The app opens the database through the library, so it applies migrations. A newer app would migrate the database and the older CLI would then refuse to write to it: **the app and the CLI must be installed from the same library tag.**
- The app never creates the database file: if it is missing, it tells the user to run `hikestat import` or to pick a file.
- Both sides use WAL mode with a `busy_timeout`, so the app can read during an import and waits instead of failing with "database is locked".
- Every new query or edit the app needs is first added to the library.
