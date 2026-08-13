mod auth;
mod commands;
mod downloads;
mod error;
mod instances;
mod java;
mod loaders;
mod minecraft;
mod modrinth;
mod storage;

use storage::paths::PathManager;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let paths = PathManager::from_app(app.handle())?;
            paths.ensure_directories()?;
            instances::manager::ensure_default(&paths)?;
            app.manage(paths);
            app.manage(modrinth::client::ModrinthClient::new()?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::launcher::get_launcher_overview,
            commands::launcher::open_launcher_folder,
            commands::launcher::open_logs_folder,
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::instances::list_instances,
            commands::instances::list_supported_versions,
            commands::instances::save_instance,
            commands::instances::create_instance,
            commands::instances::duplicate_instance,
            commands::instances::delete_instance,
            commands::instances::open_instance_folder,
            commands::mods::search_mods,
            commands::mods::get_mod_details,
            commands::mods::list_mod_versions,
            commands::mods::analyze_mod_conflicts,
            commands::mods::add_mod,
            commands::mods::remove_mod,
            commands::mods::set_mod_enabled,
            commands::mods::set_mod_version,
            commands::accounts::get_account,
            commands::accounts::begin_microsoft_login,
            commands::accounts::complete_microsoft_login,
            commands::accounts::logout,
            commands::minecraft::launch_game
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
