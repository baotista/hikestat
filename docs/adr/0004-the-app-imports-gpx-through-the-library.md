# The app imports GPX through the library

ADR 0002 kept the CLI as the only way in for GPX: the app never imported and never created the database. Having to open a terminal to add a hike turned out to be the main friction, so the app now imports the GPX files and folders dropped on its window by calling the library's import (`import_path`), the same code as `hikestat import`: deduplication by `trace_hash`, one transaction per trace, the same rejections and warnings. We chose this over a separate import path in the app (or shelling out to the CLI) because the logic already lives in the library: one implementation, one set of rules, and nothing to keep in sync.

## Consequences

- The first import creates the database at the path in use (default path or the file chosen in Settings), as the CLI does; the "no database" screen invites dropping files instead of only pointing to the CLI.
- ADR 0002 still holds for everything else: one shared file, the library owns the schema, the app applies its migrations, and **the app and the CLI must be installed from the same library tag**.
- Imported rows are the library's, whoever triggered the import: the app still never writes points, names or stats itself.
