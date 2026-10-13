# Import, statistics and database

How Hikestat imports GPX, computes the statistics and stores them. The code is in
`src-tauri/src/domain/`.

## Import

Dropping files or folders on the window imports each `.gpx` file, and every `.gpx` file located
**directly** in a dropped folder (subfolders are not entered, the extension is case-insensitive,
other files are ignored). Files are processed in name order.

- **One track (`<trk>`) = one hike.** A file containing 24 tracks creates 24 hikes.
- **Replaying an import never creates duplicates.** Each track has an identity computed from its
  points (SHA-256 of the latitudes, longitudes, elevations and times), not from its name.
  Re-importing an export that has grown only adds the new hikes, and renaming a track in the source
  application does not make it a new one.
- **Each track is atomic**: a hike is inserted entirely (statistics and points) or not at all.
- The kind is the track's `<type>` if there is one, otherwise nothing. It is **never** guessed from
  the name. A re-import doesn't change a hike already present: its kind, title and note are kept.

| Case | Result |
| --- | --- |
| `<rte>` (planned route) and `<wpt>` (waypoint) | ignored, with a warning |
| File with no track at all | nothing is imported, warning |
| Track in which **no** point has a timestamp | **rejected**, the other tracks of the file carry on |
| Points without `<ele>` or `<time>` | stored with an empty value (NULL), stats use what exists |
| Unreadable file or invalid XML | **rejected** as a whole, the other files carry on |

## Statistics

Algorithm of `stats_version` **2**. The constants are named in `src-tauri/src/domain/stats.rs`.

- **Distance**: Haversine formula in 2D, summed per segment. A gap between two `<trkseg>` adds no
  distance.
- **Elevation gain**: hysteresis of **3 m**. A climb or descent only counts once it exceeds 3 m
  cumulated since the last retained point, which absorbs GPS elevation noise.
- **Outlier elevations**: elevations outside **-500 to 9,000 m** are ignored for the elevation gain
  and the minimum and maximum elevations. The points are left untouched in the database.
- **Moving time**: an interval between two points counts as stopped if the speed is below
  **0.3 m/s** or if the time gap exceeds **60 s** (signal loss). An interval with no timestamp or no
  positive time gap is ignored.
- **Average speed**: distance covered while moving divided by the moving time, `NULL` if never
  moving.

When the algorithm or a constant changes, bump `STATS_VERSION`. `Db::recompute_stats` brings the
stored rows up to date from their points (the GPX files are not kept).

## Database

SQLite, a single file, in [WAL](https://sqlite.org/wal.html) mode: the `-wal` and `-shm` files next
to it are part of the database, don't delete them. Migrations are applied when the app opens it; a
database whose schema is **newer** than the app's is refused, without any write.

Dates are seconds since the Unix epoch (UTC), distances and elevations are in metres, durations in
seconds, speeds in metres per second.

**`hikes`**, one row per hike:

| Column | Content |
| --- | --- |
| `id` | primary key |
| `name` | track name (`<trk><name>`), as is |
| `kind` | activity kind, free text, `NULL` if unknown |
| `started_at` | time of the first timestamped point |
| `start_tz` | IANA time zone (e.g. `Europe/Paris`) of the first timestamped point, derived offline from its coordinates; `Etc/GMT±N` at sea |
| `distance_m` | total distance |
| `ascent_m`, `descent_m` | elevation gain and loss |
| `min_ele_m`, `max_ele_m` | minimum and maximum elevation |
| `duration_total_s` | from the first to the last timestamped point |
| `duration_moving_s` | time spent moving |
| `avg_speed_mps` | average speed while moving |
| `stats_version` | version of the algorithm that produced the stats |
| `trace_hash` | track identity (`UNIQUE`) |
| `title` | title chosen by the user, shown instead of `name`; `NULL` if none |
| `note` | free text written by the user; `NULL` if none |

**`track_points`**, the raw points, never modified:

| Column | Content |
| --- | --- |
| `hike_id` | hike (cascading delete) |
| `segment_idx`, `idx` | segment (`<trkseg>`), then position within the segment |
| `lat`, `lon` | coordinates |
| `ele`, `time` | elevation and time, `NULL` if absent |

Statistics can be `NULL` when data is missing (for example the elevation gain of a track with no
elevation).
