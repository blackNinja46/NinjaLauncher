use crate::storage::paths::PathManager;
use crate::{
    control_api,
    control_api::{ClientBootstrap, CrashReportSubmission},
};
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

#[tauri::command]
pub async fn get_launcher_bootstrap(
    minecraft_version: Option<String>,
) -> Result<ClientBootstrap, crate::error::CommandError> {
    control_api::fetch_launcher_bootstrap(minecraft_version.as_deref()).await
}

#[tauri::command]
pub async fn record_client_event(
    event_type: String,
    minecraft_version: Option<String>,
) -> Result<(), crate::error::CommandError> {
    control_api::record_event(&event_type, minecraft_version.as_deref()).await
}

#[tauri::command]
pub async fn upload_crash_report(
    report: CrashReportSubmission,
) -> Result<(), crate::error::CommandError> {
    control_api::upload_crash_report(&report).await
}
