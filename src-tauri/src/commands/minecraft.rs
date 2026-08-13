use crate::{
    error::CommandError, minecraft::launcher, modrinth::client::ModrinthClient,
    storage::paths::PathManager,
};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn launch_game(
    instance_id: String,
    demo: bool,
    app: AppHandle,
    paths: State<'_, PathManager>,
    modrinth: State<'_, ModrinthClient>,
) -> Result<(), CommandError> {
    launcher::launch(app, &paths, &modrinth, &instance_id, demo).await
}
