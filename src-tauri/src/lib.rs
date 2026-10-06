mod aggregate;
mod hike;
mod location;
mod status;
mod track;

use std::path::PathBuf;

use hikestat::storage::Db;
use jiff::civil::Date;
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::{Builder, collect_commands};

use aggregate::{Dashboard, KindFilter, View};
use hike::{HikeDetails, HikeId};
use location::Settings;
use status::DbStatus;
use track::MapTrack;

fn settings_file<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

fn db_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let settings = location::load_settings(&settings_file(app)?);
    Ok(location::resolve(&settings, location::default_db_path()))
}

/// Runs `work` on a blocking thread: opening the database can wait up to the busy timeout while
/// the CLI is importing, and that must not freeze the UI.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| e.to_string())?
}

/// Status of the database the app currently points at.
#[tauri::command]
#[specta::specta]
async fn db_status<R: Runtime>(app: AppHandle<R>) -> Result<DbStatus, String> {
    blocking(move || Ok(status::status_of(&db_path(&app)?))).await
}

/// The hikes of the `view` window containing `anchor` (today in the machine's time zone if
/// `null`), keeping those matching `kind_filter`. The All view runs up to today's year.
#[tauri::command]
#[specta::specta]
async fn dashboard<R: Runtime>(
    app: AppHandle<R>,
    view: View,
    anchor: Option<Date>,
    kind_filter: KindFilter,
) -> Result<Dashboard, String> {
    let today = jiff::Zoned::now().date();
    let anchor = anchor.unwrap_or(today);
    blocking(move || {
        let db = Db::open_existing(&db_path(&app)?).map_err(|e| e.to_string())?;
        let hikes = db.list_hikes().map_err(|e| e.to_string())?;
        aggregate::aggregate(&hikes, view, anchor, &kind_filter, today).map_err(|e| e.to_string())
    })
    .await
}

/// The hike with this `id`, or an error if there is none.
#[tauri::command]
#[specta::specta]
async fn hike<R: Runtime>(app: AppHandle<R>, id: HikeId) -> Result<HikeDetails, String> {
    blocking(move || {
        let db = Db::open_existing(&db_path(&app)?).map_err(|e| e.to_string())?;
        hike::hike(&db, id.0)
    })
    .await
}

/// The hikes with these `ids` (a bucket's `hikeIds`), in the same order; unknown ids are left out.
#[tauri::command]
#[specta::specta]
async fn hikes<R: Runtime>(
    app: AppHandle<R>,
    ids: Vec<HikeId>,
) -> Result<Vec<HikeDetails>, String> {
    blocking(move || {
        let db = Db::open_existing(&db_path(&app)?).map_err(|e| e.to_string())?;
        let ids: Vec<i64> = ids.into_iter().map(|id| id.0).collect();
        hike::hikes(&db, &ids)
    })
    .await
}

/// What the map of the hike `id` draws (its segments and the gaps between them), or an error if
/// there is no such hike.
#[tauri::command]
#[specta::specta]
async fn track<R: Runtime>(app: AppHandle<R>, id: HikeId) -> Result<MapTrack, String> {
    blocking(move || {
        let db = Db::open_existing(&db_path(&app)?).map_err(|e| e.to_string())?;
        track::track(&db, id.0)
    })
    .await
}

/// The distinct kinds of the hikes, as stored, sorted case-insensitively (for the kind filter).
#[tauri::command]
#[specta::specta]
async fn kinds<R: Runtime>(app: AppHandle<R>) -> Result<Vec<String>, String> {
    blocking(move || {
        let db = Db::open_existing(&db_path(&app)?).map_err(|e| e.to_string())?;
        db.kinds().map_err(|e| e.to_string())
    })
    .await
}

/// Sets the title of hike `id` to `value` (trimmed; empty clears it, falling back to the raw
/// name) and returns the updated hike.
#[tauri::command]
#[specta::specta]
async fn update_title<R: Runtime>(
    app: AppHandle<R>,
    id: HikeId,
    value: String,
) -> Result<HikeDetails, String> {
    blocking(move || {
        let db = Db::open_existing(&db_path(&app)?).map_err(|e| e.to_string())?;
        hike::update_title(&db, id.0, &value)
    })
    .await
}

/// Sets the note of hike `id` to `value` (trimmed, inner line breaks kept; empty clears it) and
/// returns the updated hike.
#[tauri::command]
#[specta::specta]
async fn update_note<R: Runtime>(
    app: AppHandle<R>,
    id: HikeId,
    value: String,
) -> Result<HikeDetails, String> {
    blocking(move || {
        let db = Db::open_existing(&db_path(&app)?).map_err(|e| e.to_string())?;
        hike::update_note(&db, id.0, &value)
    })
    .await
}

/// Sets the kind of the hike `id` to `value` (trimmed, stored as typed otherwise; empty or blank
/// clears it to Not set) and returns the updated hike.
#[tauri::command]
#[specta::specta]
async fn update_kind<R: Runtime>(
    app: AppHandle<R>,
    id: HikeId,
    value: String,
) -> Result<HikeDetails, String> {
    blocking(move || {
        let db = Db::open_existing(&db_path(&app)?).map_err(|e| e.to_string())?;
        hike::set_kind(&db, id.0, &value)
    })
    .await
}

/// Points the app at `path` (or back at the default path with `null`), remembers it, and returns
/// the new status.
#[tauri::command]
#[specta::specta]
async fn choose_db<R: Runtime>(
    app: AppHandle<R>,
    path: Option<String>,
) -> Result<DbStatus, String> {
    let settings = Settings {
        db_path: path.map(PathBuf::from),
    };
    location::save_settings(&settings_file(&app)?, &settings)
        .map_err(|e| format!("cannot save the settings: {e}"))?;
    db_status(app).await
}

fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        db_status::<tauri::Wry>,
        choose_db::<tauri::Wry>,
        dashboard::<tauri::Wry>,
        hike::<tauri::Wry>,
        hikes::<tauri::Wry>,
        track::<tauri::Wry>,
        kinds::<tauri::Wry>,
        update_title::<tauri::Wry>,
        update_note::<tauri::Wry>,
        update_kind::<tauri::Wry>
    ])
}

const BINDINGS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts");

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    builder
        .export(specta_typescript::Typescript::default(), BINDINGS)
        .expect("failed to export the TypeScript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running Hikestat");
}

#[cfg(test)]
mod tests {
    /// Keeps the committed bindings in sync without launching the app.
    #[test]
    fn export_bindings() {
        super::specta_builder()
            .export(specta_typescript::Typescript::default(), super::BINDINGS)
            .unwrap();
    }
}
