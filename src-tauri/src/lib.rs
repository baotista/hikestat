mod aggregate;
mod location;
mod status;

use std::path::PathBuf;

use hikestat::storage::Db;
use jiff::civil::Date;
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::{Builder, collect_commands};

use aggregate::{Dashboard, KindFilter, View};
use location::Settings;
use status::DbStatus;

fn settings_file<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

fn db_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    let settings = location::load_settings(&settings_file(app)?);
    Ok(location::resolve(&settings, location::default_db_path()))
}

/// Status of the database the app currently points at.
#[tauri::command]
#[specta::specta]
fn db_status<R: Runtime>(app: AppHandle<R>) -> Result<DbStatus, String> {
    Ok(status::status_of(&db_path(&app)?))
}

/// The hikes of the `view` window containing `anchor` (today in the machine's time zone if
/// `null`), keeping those matching `kind_filter`.
#[tauri::command]
#[specta::specta]
fn dashboard<R: Runtime>(
    app: AppHandle<R>,
    view: View,
    anchor: Option<Date>,
    kind_filter: KindFilter,
) -> Result<Dashboard, String> {
    let anchor = anchor.unwrap_or_else(|| jiff::Zoned::now().date());
    let db = Db::open_existing(&db_path(&app)?).map_err(|e| e.to_string())?;
    let hikes = db.list_hikes().map_err(|e| e.to_string())?;
    aggregate::aggregate(&hikes, view, anchor, &kind_filter).map_err(|e| e.to_string())
}

/// Points the app at `path` (or back at the default path with `null`), remembers it, and returns
/// the new status.
#[tauri::command]
#[specta::specta]
fn choose_db<R: Runtime>(app: AppHandle<R>, path: Option<String>) -> Result<DbStatus, String> {
    let settings = Settings {
        db_path: path.map(PathBuf::from),
    };
    location::save_settings(&settings_file(&app)?, &settings)
        .map_err(|e| format!("cannot save the settings: {e}"))?;
    db_status(app)
}

fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        db_status::<tauri::Wry>,
        choose_db::<tauri::Wry>,
        dashboard::<tauri::Wry>
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
