use crate::{
    discord::DiscordPresence, error::CommandError, minecraft::launcher,
    minecraft::processes::GameProcesses, modrinth::client::ModrinthClient,
    storage::paths::PathManager,
};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub async fn get_server_icon(server_address: String) -> Result<Option<String>, CommandError> {
    launcher::fetch_server_icon(&server_address).await
}

#[tauri::command]
pub fn list_running_instances(processes: State<'_, GameProcesses>) -> Vec<String> {
    processes.list()
}

#[tauri::command]
pub async fn stop_game(
    instance_id: String,
    processes: State<'_, GameProcesses>,
) -> Result<(), CommandError> {
    processes.stop(&instance_id).await
}

#[tauri::command]
pub async fn launch_game(
    instance_id: String,
    demo: bool,
    server_address: Option<String>,
    app: AppHandle,
    paths: State<'_, PathManager>,
    modrinth: State<'_, ModrinthClient>,
    discord: State<'_, DiscordPresence>,
) -> Result<(), CommandError> {
    let processes = app.state::<GameProcesses>();
    if processes.list().iter().any(|id| id == &instance_id) {
        return Err(CommandError::new("This profile is already running."));
    }
    discord.show_launching();
    let result = launcher::launch(
        app,
        &paths,
        &modrinth,
        &instance_id,
        demo,
        server_address.as_deref(),
    )
    .await;
    if result.is_err() {
        discord.show_launcher();
    }
    result
}
