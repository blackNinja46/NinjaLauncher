use crate::{
    discord::DiscordPresence,
    error::CommandError,
    storage::{config, config::LauncherSettings, paths::PathManager},
};
use tauri::State;

#[tauri::command]
pub fn get_settings(paths: State<'_, PathManager>) -> Result<LauncherSettings, CommandError> {
    config::load(&paths)
}

#[tauri::command]
pub fn save_settings(
    settings: LauncherSettings,
    paths: State<'_, PathManager>,
    discord: State<'_, DiscordPresence>,
) -> Result<LauncherSettings, CommandError> {
    config::save(&paths, &settings)?;
    discord.set_enabled(settings.discord_rich_presence);
    Ok(settings)
}
