use crate::storage::paths::PathManager;
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherOverview {
    app_data_directory: String,
    instance_count: usize,
    version: &'static str,
}

#[tauri::command]
pub fn open_launcher_folder(
    app: AppHandle,
    paths: State<'_, PathManager>,
) -> Result<(), crate::error::CommandError> {
    app.opener()
        .open_path(paths.root().to_string_lossy(), None::<String>)
        .map_err(|error| {
            crate::error::CommandError::new(format!("Could not open the launcher folder: {error}"))
        })
}

#[tauri::command]
pub fn open_logs_folder(
    app: AppHandle,
    paths: State<'_, PathManager>,
) -> Result<(), crate::error::CommandError> {
    app.opener()
        .open_path(paths.logs_dir().to_string_lossy(), None::<String>)
        .map_err(|error| {
            crate::error::CommandError::new(format!("Could not open the logs folder: {error}"))
        })
}

#[tauri::command]
pub fn get_launcher_overview(paths: State<'_, PathManager>) -> LauncherOverview {
    let instance_count = std::fs::read_dir(paths.instances_dir())
        .map(|entries| entries.filter_map(Result::ok).count())
        .unwrap_or(0);
    LauncherOverview {
        app_data_directory: paths.root().to_string_lossy().into_owned(),
        instance_count,
        version: env!("CARGO_PKG_VERSION"),
    }
}
