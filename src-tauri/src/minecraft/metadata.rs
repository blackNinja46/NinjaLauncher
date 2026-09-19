use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize)]
pub struct VersionManifest {
    pub versions: Vec<ManifestVersion>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct ManifestVersion {
    pub id: String,
    pub url: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct VersionMetadata {
    pub id: String,
    pub main_class: String,
    pub assets: String,
    pub asset_index: Option<AssetIndexRef>,
    pub downloads: Option<VersionDownloads>,
    pub libraries: Vec<Library>,
    pub arguments: Arguments,
    pub java_version: Option<JavaVersion>,
    pub logging: Option<Logging>,
    pub minecraft_arguments: Option<String>,
}
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Conditional {
        #[serde(default)]
        rules: Vec<Rule>,
        value: ArgumentValue,
    },
}
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    One(String),
    Many(Vec<String>),
}
#[derive(Clone, Debug, Deserialize)]
pub struct JavaVersion {
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}
#[derive(Clone, Debug, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub url: String,
    pub sha1: String,
}
#[derive(Clone, Debug, Deserialize)]
pub struct VersionDownloads {
    pub client: Download,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Download {
    pub url: String,
    pub sha1: Option<String>,
    pub path: Option<String>,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub url: Option<String>,
    pub rules: Vec<Rule>,
    pub natives: HashMap<String, String>,
    pub extract: Option<Extract>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<Download>,
    #[serde(default)]
    pub classifiers: HashMap<String, Download>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<HashMap<String, bool>>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Logging {
    pub client: LoggingClient,
}
#[derive(Clone, Debug, Deserialize)]
pub struct LoggingClient {
    pub argument: String,
    pub file: LoggingFile,
}
#[derive(Clone, Debug, Deserialize)]
pub struct LoggingFile {
    pub id: String,
    pub url: String,
    pub sha1: String,
}
#[derive(Debug, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}
#[derive(Debug, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    #[serde(default)]
    pub size: u64,
}

pub fn applies(rules: &[Rule]) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in rules {
        if matches_rule(rule) {
            allowed = rule.action == "allow";
        }
    }
    allowed
}
fn matches_rule(rule: &Rule) -> bool {
    if let Some(os) = &rule.os {
        if let Some(name) = &os.name {
            if name != current_os() {
                return false;
            }
        }
        if let Some(arch) = &os.arch {
            if !current_arch().contains(arch) && !arch.contains(current_arch()) {
                return false;
            }
        }
    }
    if let Some(features) = &rule.features {
        if features.values().any(|required| *required) {
            return false;
        }
    }
    true
}
pub fn current_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}
pub fn current_arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86_64"
    }
}
pub fn expand_arguments(arguments: &[Argument]) -> Vec<String> {
    arguments
        .iter()
        .flat_map(|argument| match argument {
            Argument::Plain(value) => vec![value.clone()],
            Argument::Conditional { rules, value } if applies(rules) => match value {
                ArgumentValue::One(value) => vec![value.clone()],
                ArgumentValue::Many(values) => values.clone(),
            },
            _ => Vec::new(),
        })
        .collect()
}
