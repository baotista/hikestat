# Hikestat — Context

A macOS desktop app that imports `.gpx` files, computes statistics for each hike, stores them in
SQLite and shows them on a dashboard.

## Vocabulary

- **Drag and drop**: `.gpx` files and/or folders dropped on the app's window; one drop is one
  import and one summary. _Avoid_: upload.
- **Trace**: a `<trk>` element of the GPX. The unit of import.
- **Hike**: an imported trace, with its statistics. One row of `hikes`. A GPX file can give several
  hikes (the sample Gaia GPS export holds 24). Used whatever the kind (a bike ride is a hike).
  _Avoid_: outing, activity.
- **Name**: the raw `<trk><name>`, as imported. Never modified.
- **Title**: name chosen by the user in the app. The title is shown if there is one, otherwise the
  name. _Avoid_: "rename" in the sense of changing the name.
- **Note**: free text attached to a hike by the user.
- **Segment**: a `<trkseg>` element. Used so that no distance is counted across a signal gap.
  Stored as `segment_idx`.
- **Point**: a `<trkpt>` (lat, lon, optional `ele` and `time`). Stored raw in `track_points`.
- **Kind**: activity kind (walking, cycling…), nullable. Never guessed from the name.
- **Stats**: values computed from a hike's points. A pure function of the points, versioned by
  `stats_version`.
- **Start time zone**: time zone of the place of a hike's first timestamped point. Derived from the
  coordinates, never from the machine.
- **Local date**: start date in the start time zone. It ties a hike to a day, week, month, year. A
  hike crossing midnight belongs to its start day.
- **Period**: calendar span the stats are aggregated over (week, month, year). The week is the ISO
  week (Monday → Sunday, ISO numbering: week 1 of 2027 starts on Monday 4 January 2027).
- **View**: how the dashboard walks through the stats, after the Health app. A view sets a window
  (a period) and cuts it into smaller bars: W = week by day, M = month by day, Y = year by month,
  All = one bar per year. W is the default view, even when most days are empty.
- **Total duration / moving duration**: last point minus first point / sum of the intervals counted
  as moving.
- **Interval**: going from one point to the next within a segment. It is moving, stopped, or
  untimed (one of the two points has no `time`).
- **Signal gap**: the span between the end of a segment and the start of the next. Nothing is
  measured there: it adds neither distance nor moving duration. _Avoid_: break, pause (a pause is a
  stopped interval).
- **Smoothed speed**: speed of a moving interval, averaged over a short distance around it to smooth
  GPS noise. The map colours it; a stopped or untimed interval has none.
- **(Elevation) profile**: a hike's elevation against cumulated distance (counted like the stats':
  nothing across a signal gap). Outlier elevations ignored by the stats are left out; no elevation,
  no profile. _Avoid_: ascent (that's the sum of climbs or descents).
- **List of hikes**: the hikes the dashboard shows under the chart: those of the window and kind
  filter, or of the selected bar. In the All view with no bar selected, every hike. The search
  narrows the list only, never the chart.

## Decisions

1. **Stack**: a single Tauri app (see [ADR 0005](docs/adr/0005-one-repository-no-cli.md)). The
   backend's import, stats and storage code lives in `src-tauri/src/domain/`: `rusqlite`
   (`bundled`) + `rusqlite_migration`, `gpx` for parsing, `sha2`, `thiserror`, `time`,
   `directories`, `tzf-rs` (time zones, offline).
2. **Storage**: aggregates **and** raw points. The GPX file is **not** kept: the points in the
   database are the only source.
3. **One trace = one hike.** `<wpt>`, `<rte>` and `<metadata>` are ignored (with a warning for
   routes). `<metadata><time>` is not a hike date: the start date is the first point's.
4. **Idempotence per trace**: `trace_hash` = SHA-256 of the points (lat, lon, ele, time, canonical
   serialisation), `UNIQUE`. The name is not part of the hash. No file hash.
5. **Missing fields**: missing `ele` or `time` → NULL, stats computed from what exists. Trace with
   no timestamp at all → rejected (warning), the rest of the file carries on.
6. **Atomicity per trace**: one transaction per `<trk>`. Unreadable file → rejected as a whole.
7. **Kind**: `<trk><type>` > NULL. The raw trace name is stored. The kind can be edited in the app
   (`Db::set_kind`): it is edited in place, the original `<type>` is not kept aside (deliberate: no
   "imported kind" column, no going back to the GPX value). Trimmed, otherwise stored as is (no case
   normalisation); empty or blank → NULL; unknown id → error. A re-import never overwrites it (the
   trace is already there); recomputing the stats doesn't touch it.
8. **Stats computation** (named constants, `stats_version` bumped when they change):
   - distance: 2D Haversine, per segment;
   - ascent/descent: 3 m hysteresis;
   - stopped if speed < 0.3 m/s or time gap > 60 s;
   - average speed over the moving duration;
   - outliers: no filtering in v1; duplicated or zero-interval points kept, ignored for speed.
9. **Recomputing**: `Db::recompute_stats` recomputes every hike's stats and start time zone from its
   points, in one transaction; it never touches the points, name, kind, title or note.
10. **Database**: default path in the user data directory, or the file chosen in Settings.
    Migrations applied when opening; a database newer than the app → nothing is written. Dates in
    UTC epoch seconds. WAL mode and a 5 s `busy_timeout`. The dashboard opens it without ever
    creating it (`Db::open_existing`, missing database → error carrying the path); the first import
    creates it.
11. **Start time zone**: IANA name stored in `start_tz`, derived offline (`tzf-rs`) from the
    coordinates of the first timestamped point. Derived data like the stats: computed on import,
    filled in or fixed by recomputing, never entered by the user. At sea: nautical zone
    `Etc/GMT±N`; NULL only for out-of-range coordinates.
12. **Tests**: unit tests of the computation (TDD), an integration test on a reduced, anonymised
    version (constant coordinate shift, 2–3 traces) of the sample export, plus inline fixtures for
    edge cases. The original export stays out of the repository.
13. **Title and note**: nullable `title` and `note` columns of `hikes`, entered only by the user in
    the app (`Db::set_title`, `Db::set_note`). Empty or blank → NULL; the title is trimmed, so is
    the note but it keeps its inner line breaks. Unknown id → `Error::HikeNotFound`. Import (even
    replayed) and recomputing never touch them; the raw `name` is never modified.
