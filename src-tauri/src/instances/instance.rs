use crate::error::CommandError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DEFAULT_MINECRAFT_VERSION: &str = "26.2";
pub const DEFAULT_FABRIC_VERSION: &str = "0.19.3";
const FALLBACK_MINECRAFT_VERSIONS: [&str; 16] = [
    "26.2", "26.1.2", "26.1.1", "26.1", "1.21.11", "1.21.10", "1.21.9", "1.21.8", "1.21.7",
    "1.21.6", "1.21.5", "1.21.4", "1.21.3", "1.21.2", "1.21.1", "1.21",
];

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportedVersion {
    pub minecraft_version: String,
    pub fabric_loader_version: String,
    pub version_type: String,
}

#[derive(Deserialize)]
struct FabricGameVersion {
    version: String,
    stable: bool,
}

#[derive(Deserialize)]
struct FabricLoaderVersion {
    version: String,
    stable: bool,
}

pub async fn supported_versions() -> Vec<SupportedVersion> {
    fetch_supported_versions()
        .await
        .unwrap_or_else(|_| fallback_versions())
}

async fn fetch_supported_versions() -> Result<Vec<SupportedVersion>, CommandError> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("NinjaLauncher/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let games: Vec<FabricGameVersion> = client
        .get("https://meta.fabricmc.net/v2/versions/game")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let loaders: Vec<FabricLoaderVersion> = client
        .get("https://meta.fabricmc.net/v2/versions/loader")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let loader = loaders
        .iter()
        .find(|loader| loader.stable)
        .or_else(|| loaders.first())
        .ok_or_else(|| CommandError::new("Fabric did not return a loader version."))?
        .version
        .clone();

    relevant_versions(games, loader)
}

fn relevant_versions(
    games: Vec<FabricGameVersion>,
    loader: String,
) -> Result<Vec<SupportedVersion>, CommandError> {
    let mut versions = Vec::new();
    let mut found_latest_release = false;
    for game in games {
        if game.stable {
            found_latest_release = true;
            versions.push(SupportedVersion {
                minecraft_version: game.version.clone(),
                fabric_loader_version: loader.clone(),
                version_type: "release".into(),
            });
        } else if !found_latest_release {
            versions.push(SupportedVersion {
                minecraft_version: game.version.clone(),
                fabric_loader_version: loader.clone(),
                version_type: "snapshot".into(),
            });
        }
        if game.version == "1.21" {
            break;
        }
    }
    if !versions
        .iter()
        .any(|version| version.minecraft_version == "1.21")
    {
        return Err(CommandError::new(
            "Fabric did not return the supported Minecraft version range.",
        ));
    }
    Ok(versions)
}

fn fallback_versions() -> Vec<SupportedVersion> {
    FALLBACK_MINECRAFT_VERSIONS
        .iter()
        .map(|minecraft_version| SupportedVersion {
            minecraft_version: (*minecraft_version).into(),
            fabric_loader_version: DEFAULT_FABRIC_VERSION.into(),
            version_type: "release".into(),
        })
        .collect()
}

pub fn is_valid_version_id(version: &str) -> bool {
    !version.is_empty()
        && version.len() <= 64
        && version.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | '+')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_current_snapshots_and_releases_from_1_21() {
        let versions = relevant_versions(
            vec![
                FabricGameVersion {
                    version: "26.3-snapshot-2".into(),
                    stable: false,
                },
                FabricGameVersion {
                    version: "26.2".into(),
                    stable: true,
                },
                FabricGameVersion {
                    version: "26.2-rc-1".into(),
                    stable: false,
                },
                FabricGameVersion {
                    version: "1.21".into(),
                    stable: true,
                },
                FabricGameVersion {
                    version: "1.20.6".into(),
                    stable: true,
                },
            ],
            "0.19.3".into(),
        )
        .expect("relevant versions");

        assert_eq!(
            versions
                .iter()
                .map(|version| version.minecraft_version.as_str())
                .collect::<Vec<_>>(),
            ["26.3-snapshot-2", "26.2", "1.21"]
        );
        assert_eq!(versions[0].version_type, "snapshot");
        assert_eq!(versions[1].version_type, "release");
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceConfig {
    pub id: String,
    pub name: String,
    pub minecraft_version: String,
    pub loader: String,
    pub loader_version: String,
    #[serde(default)]
    pub mods: Vec<SelectedMod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modpack: Option<InstalledModpack>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledModpack {
    pub project_id: String,
    pub version_id: String,
    pub name: String,
    pub icon_url: Option<String>,
    #[serde(default)]
    pub file_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedMod {
    pub project_id: String,
    pub name: String,
    pub enabled: bool,
    #[serde(default)]
    pub version_overrides: HashMap<String, String>,
}

impl InstanceConfig {
    pub fn default_instance() -> Self {
        Self {
            id: "default-fabric".into(),
            name: "My Fabric World".into(),
            minecraft_version: DEFAULT_MINECRAFT_VERSION.into(),
            loader: "fabric".into(),
            loader_version: DEFAULT_FABRIC_VERSION.into(),
            mods: Vec::new(),
            modpack: None,
        }
    }
}
