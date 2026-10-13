use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use crate::domain::parser::Trace;
use crate::domain::storage::{Db, Inserted};
use crate::domain::{identity, parser, stats};

#[derive(Debug, Clone, PartialEq)]
pub struct Rejection {
    pub file: PathBuf,
    /// 1-based position of the `<trk>` in the file; `None` when the whole file is rejected.
    pub trace_number: Option<usize>,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImportSummary {
    pub imported: usize,
    /// The ids of the hikes this run created, in creation order; none for a dry run (rolled back).
    pub imported_ids: Vec<i64>,
    pub already_present: usize,
    pub rejected: Vec<Rejection>,
    /// Things ignored on purpose (routes, waypoints, a file with no track).
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ImportOptions {
    /// Applied to every imported trace; wins over the file's own `<trk><type>`.
    pub kind: Option<String>,
    /// Do everything except keep the result: same summary, nothing written.
    pub dry_run: bool,
}

/// Trims, and treats a blank value as absent.
fn normalize_kind(kind: Option<&str>) -> Option<String> {
    kind.map(str::trim)
        .filter(|k| !k.is_empty())
        .map(str::to_string)
}

impl ImportSummary {
    fn merge(&mut self, other: ImportSummary) {
        self.imported += other.imported;
        self.imported_ids.extend(other.imported_ids);
        self.already_present += other.already_present;
        self.rejected.extend(other.rejected);
        self.warnings.extend(other.warnings);
    }
}

/// Imports a GPX file, or every `.gpx` file directly inside a directory.
pub fn import_path(db: &mut Db, path: &Path, opts: &ImportOptions) -> ImportSummary {
    run(db, path, opts, |db| import_path_inner(db, path, opts))
}

/// Imports what was dropped on the app's window, in one run and one summary: a directory imports
/// its direct `.gpx` files, a `.gpx` file (any case) is imported, anything else is rejected.
pub fn import_paths(db: &mut Db, paths: &[PathBuf], opts: &ImportOptions) -> ImportSummary {
    // One run for the whole drop: a dry run also sees duplicates across the dropped paths.
    run(db, Path::new(""), opts, |db| {
        let mut summary = ImportSummary::default();
        for path in paths {
            summary.merge(if path.is_dir() {
                import_path_inner(db, path, opts)
            } else if is_gpx(path) {
                import_file_inner(db, path, opts)
            } else {
                ImportSummary {
                    rejected: vec![Rejection {
                        file: path.clone(),
                        trace_number: None,
                        reason: "not a .gpx file".into(),
                    }],
                    ..ImportSummary::default()
                }
            });
        }
        summary
    })
}

/// Imports every trace of a GPX file as its own hike, skipping traces already in the database.
pub fn import_file(db: &mut Db, path: &Path, opts: &ImportOptions) -> ImportSummary {
    run(db, path, opts, |db| import_file_inner(db, path, opts))
}

/// A dry run is the real import inside a transaction that is always rolled back. Duplicate
/// detection (against the database and within the run), rejections and atomicity therefore behave
/// exactly as in a real import, without a second code path to keep in sync.
/// `path` names the run in the rejection if the dry run's transaction itself fails.
fn run(
    db: &mut Db,
    path: &Path,
    opts: &ImportOptions,
    import: impl FnOnce(&mut Db) -> ImportSummary,
) -> ImportSummary {
    if !opts.dry_run {
        return import(db);
    }
    db.rolled_back(import)
        .map(|summary| ImportSummary {
            imported_ids: Vec::new(),
            ..summary
        })
        .unwrap_or_else(|e| ImportSummary {
            rejected: vec![Rejection {
                file: path.to_path_buf(),
                trace_number: None,
                reason: e.to_string(),
            }],
            ..ImportSummary::default()
        })
}

/// Not recursive, in name order. One bad file never stops the others; the summary aggregates all
/// of them.
fn import_path_inner(db: &mut Db, path: &Path, opts: &ImportOptions) -> ImportSummary {
    if !path.is_dir() {
        return import_file_inner(db, path, opts);
    }
    let mut summary = ImportSummary::default();
    let mut files: Vec<PathBuf> = match std::fs::read_dir(path) {
        Ok(entries) => entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && is_gpx(p))
            .collect(),
        Err(e) => {
            summary.rejected.push(Rejection {
                file: path.to_path_buf(),
                trace_number: None,
                reason: e.to_string(),
            });
            return summary;
        }
    };
    files.sort();
    if files.is_empty() {
        summary
            .warnings
            .push(format!("{}: no .gpx file found", path.display()));
    }
    for file in files {
        summary.merge(import_file_inner(db, &file, opts));
    }
    summary
}

fn is_gpx(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gpx"))
}

/// Each trace is atomic. A trace that cannot be imported is rejected and the others go on; an
/// unreadable file is rejected as a whole. Nothing here aborts the run: it is all in the summary.
fn import_file_inner(db: &mut Db, path: &Path, opts: &ImportOptions) -> ImportSummary {
    let forced_kind = normalize_kind(opts.kind.as_deref());
    let mut summary = ImportSummary::default();
    let reject = |summary: &mut ImportSummary, trace_number, reason: String| {
        summary.rejected.push(Rejection {
            file: path.to_path_buf(),
            trace_number,
            reason,
        });
    };

    let parsed = match File::open(path)
        .map_err(|e| e.to_string())
        .and_then(|f| parser::parse(BufReader::new(f)).map_err(|e| e.to_string()))
    {
        Ok(parsed) => parsed,
        Err(reason) => {
            reject(&mut summary, None, reason);
            return summary;
        }
    };

    let mut ignored = |count: usize, what: &str| {
        if count > 0 {
            let warning = format!("{}: ignored {count} {what}", path.display());
            summary.warnings.push(warning);
        }
    };
    ignored(parsed.ignored_routes, "route(s)");
    ignored(parsed.ignored_waypoints, "waypoint(s)");
    if parsed.traces.is_empty() {
        summary.warnings.push(format!(
            "{}: no track found, nothing to import",
            path.display()
        ));
    }

    for (i, trace) in parsed.traces.iter().enumerate() {
        let number = Some(i + 1);
        if !has_timestamp(trace) {
            reject(&mut summary, number, "no point has a timestamp".into());
            continue;
        }
        let hash = identity::trace_hash(trace);
        let kind = forced_kind
            .clone()
            .or_else(|| normalize_kind(trace.kind.as_deref()));
        match db.insert_hike(trace, &stats::compute(trace), &hash, kind.as_deref()) {
            Ok(Inserted::Created(id)) => {
                summary.imported += 1;
                summary.imported_ids.push(id);
            }
            Ok(Inserted::AlreadyPresent) => summary.already_present += 1,
            Err(e) => reject(&mut summary, number, e.to_string()),
        }
    }
    summary
}

fn has_timestamp(trace: &Trace) -> bool {
    trace.segments.iter().flatten().any(|p| p.time.is_some())
}
