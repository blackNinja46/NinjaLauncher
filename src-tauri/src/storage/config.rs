use crate::{error::CommandError, storage::paths::PathManager};
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
#[serde(rename_all = "camelCase")]
pub struct LauncherSettings {
    pub setup_complete: bool,
    pub installation_path: Option<String>,
    pub terms_accepted_at: Option<String>,
    pub allocated_memory_mb: u32,
    pub java_path: Option<String>,
    pub download_concurrency: u8,
    pub close_on_launch: bool,
    pub reopen_on_exit: bool,
    pub show_logs_on_launch: bool,
    pub show_snapshots: bool,
    pub language: String,
    pub theme: Theme,
    pub animations_enabled: bool,
    pub parallax_enabled: bool,
    pub halloween_theme_enabled: bool,
    pub halloween_sounds_enabled: bool,
    pub check_for_updates: bool,
    pub discord_rich_presence: bool,
    pub share_anonymous_usage_data: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    System,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            setup_complete: false,
            installation_path: None,
            terms_accepted_at: None,
            allocated_memory_mb: 4096,
            java_path: None,
            download_concurrency: 4,
            close_on_launch: false,
            reopen_on_exit: true,
            show_logs_on_launch: true,
            show_snapshots: false,
            language: "en".into(),
            theme: Theme::Dark,
            animations_enabled: true,
            parallax_enabled: true,
            halloween_theme_enabled: true,
            halloween_sounds_enabled: true,
            check_for_updates: true,
            discord_rich_presence: true,
            share_anonymous_usage_data: false,
        }
    }
}

impl LauncherSettings {
    pub fn validate(&self) -> Result<(), CommandError> {
        if !(1024..=32768).contains(&self.allocated_memory_mb) {
            return Err(CommandError::new(
                "Allocated memory must be between 1 GB and 32 GB.",
            ));
        }
        if !(1..=12).contains(&self.download_concurrency) {
            return Err(CommandError::new(
                "Download concurrency must be between 1 and 12.",
            ));
        }
        Ok(())
    }
}

pub fn load(paths: &PathManager) -> Result<LauncherSettings, CommandError> {
    let path = paths.config_file();
    if !path.exists() {
        let settings = LauncherSettings::default();
        save(paths, &settings)?;
        return Ok(settings);
    }
    let settings = serde_json::from_slice(&std::fs::read(path)?)?;
    Ok(settings)
}

pub fn save(paths: &PathManager, settings: &LauncherSettings) -> Result<(), CommandError> {
    settings.validate()?;
    let target = paths.config_file();
    let temporary = target.with_extension("json.tmp");
    let mut file = std::fs::File::create(&temporary)?;
    file.write_all(&serde_json::to_vec_pretty(settings)?)?;
    file.sync_all()?;
    std::fs::rename(temporary, target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_download_concurrency() {
        let settings = LauncherSettings {
            download_concurrency: 0,
            ..LauncherSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn old_settings_enable_game_logs_by_default() {
        let settings: LauncherSettings =
            serde_json::from_str(r#"{"allocatedMemoryMb":4096,"downloadConcurrency":4}"#)
                .expect("old settings should remain compatible");
        assert!(settings.show_logs_on_launch);
        assert!(!settings.show_snapshots);
        assert!(settings.discord_rich_presence);
        assert!(!settings.share_anonymous_usage_data);
        assert!(settings.halloween_theme_enabled);
        assert!(settings.halloween_sounds_enabled);
    }
}
