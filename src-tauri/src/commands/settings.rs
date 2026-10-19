use crate::{
    discord::DiscordPresence,
    error::CommandError,
    instances::manager,
    java,
    process::hide_std_console,
    storage::{config, config::LauncherSettings, paths::PathManager},
};
use tauri::State;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMemoryInfo {
    total_mb: u64,
    recommended_max_mb: u64,
}

#[tauri::command]
pub fn get_system_memory() -> Result<SystemMemoryInfo, CommandError> {
    let total_mb = total_memory_mb()
        .ok_or_else(|| CommandError::new("The available system memory could not be detected."))?;
    Ok(SystemMemoryInfo {
        total_mb,
        recommended_max_mb: total_mb.saturating_sub(2048).clamp(1024, 32768),
    })
}

#[tauri::command]
pub async fn inspect_java_runtime(
    java_path: Option<String>,
    paths: State<'_, PathManager>,
) -> Result<java::manager::JavaRuntimeInfo, CommandError> {
    java::manager::inspect_runtime(&paths, java_path.as_deref()).await
}

fn total_memory_mb() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        let mut command = std::process::Command::new("/usr/sbin/sysctl");
        hide_std_console(&mut command);
        let output = command.args(["-n", "hw.memsize"]).output().ok()?;
        let bytes: u64 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .ok()?;
        return Some(bytes / 1024 / 1024);
    }
    #[cfg(target_os = "linux")]
    {
        let contents = std::fs::read_to_string("/proc/meminfo").ok()?;
        let kib: u64 = contents
            .lines()
            .find_map(|line| line.strip_prefix("MemTotal:"))?
            .split_whitespace()
            .next()?
            .parse()
            .ok()?;
        return Some(kib / 1024);
    }
    #[cfg(target_os = "windows")]
    {
        let mut command = std::process::Command::new("powershell");
        hide_std_console(&mut command);
        let output = command
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory",
            ])
            .output()
            .ok()?;
        let bytes: u64 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .ok()?;
        return Some(bytes / 1024 / 1024);
    }
    #[allow(unreachable_code)]
    None
}

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
    settings.validate()?;
    paths.set_content_root(settings.installation_path.as_deref())?;
    paths.ensure_directories()?;
    manager::ensure_default(&paths)?;
    config::save(&paths, &settings)?;
    discord.set_enabled(settings.discord_rich_presence);
    Ok(settings)
}
