use crate::{
    discord::DiscordPresence, error::CommandError, minecraft::launcher,
    modrinth::client::ModrinthClient, storage::paths::PathManager,
};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn launch_game(
    instance_id: String,
    demo: bool,
    app: AppHandle,
    paths: State<'_, PathManager>,
    modrinth: State<'_, ModrinthClient>,
    discord: State<'_, DiscordPresence>,
) -> Result<(), CommandError> {
    discord.show_launching();
    let result = launcher::launch(app, &paths, &modrinth, &instance_id, demo).await;
    if result.is_err() {
        discord.show_launcher();
    }
    result
}
