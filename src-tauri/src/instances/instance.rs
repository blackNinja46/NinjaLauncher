use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DEFAULT_MINECRAFT_VERSION: &str = "26.2";
pub const DEFAULT_FABRIC_VERSION: &str = "0.19.3";
pub const SUPPORTED_MINECRAFT_VERSIONS: [&str; 5] = ["1.21.11", "26.1", "26.1.1", "26.1.2", "26.2"];

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportedVersion {
    pub minecraft_version: &'static str,
    pub fabric_loader_version: &'static str,
}

pub fn supported_versions() -> Vec<SupportedVersion> {
    SUPPORTED_MINECRAFT_VERSIONS
        .iter()
        .map(|minecraft_version| SupportedVersion {
            minecraft_version,
            fabric_loader_version: DEFAULT_FABRIC_VERSION,
        })
        .collect()
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
