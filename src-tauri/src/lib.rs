mod auth;
mod commands;
mod discord;
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let paths = PathManager::from_app(app.handle())?;
            paths.ensure_directories()?;
            instances::manager::ensure_default(&paths)?;
            let settings = storage::config::load(&paths)?;
            let discord_presence = discord::DiscordPresence::new(settings.discord_rich_presence);
            if settings.discord_rich_presence {
                discord_presence.show_launcher();
            }
            app.manage(paths);
            app.manage(modrinth::client::ModrinthClient::new()?);
            app.manage(discord_presence);
            app.manage(minecraft::processes::GameProcesses::new(
                app.state::<PathManager>().inner(),
            ));
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
            commands::instances::list_fabric_loader_versions,
            commands::instances::save_instance,
            commands::instances::create_instance,
            commands::instances::duplicate_instance,
            commands::instances::delete_instance,
            commands::instances::open_instance_folder,
            commands::mods::search_mods,
            commands::mods::get_mod_details,
            commands::mods::list_mod_versions,
            commands::mods::get_installed_mod_versions,
            commands::mods::analyze_mod_conflicts,
            commands::mods::repair_mod_configuration,
            commands::mods::add_mod,
            commands::mods::remove_mod,
            commands::mods::set_mod_enabled,
            commands::mods::set_mod_version,
            commands::modpacks::search_modpacks,
            commands::modpacks::list_modpack_versions,
            commands::modpacks::install_modpack,
            commands::modpacks::inspect_modpack_file,
            commands::modpacks::import_modpack_file,
            commands::modpacks::refresh_imported_modpack_mods,
            commands::modpacks::list_exportable_folders,
            commands::modpacks::export_modpack,
            commands::accounts::get_account,
            commands::accounts::get_accounts,
            commands::accounts::begin_microsoft_login,
            commands::accounts::complete_microsoft_login,
            commands::accounts::switch_microsoft_account,
            commands::accounts::remove_microsoft_account,
            commands::accounts::logout,
            commands::minecraft::launch_game,
            commands::minecraft::get_server_icon,
            commands::minecraft::list_running_instances,
            commands::minecraft::stop_game
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
