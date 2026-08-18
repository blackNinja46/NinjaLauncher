use crate::{
    downloads::manager::{self as download_manager, DownloadSpec},
    error::CommandError,
    instances::{
        instance::{is_valid_version_id, InstalledModpack, InstanceConfig},
        manager as instance_manager,
    },
    modrinth::client::{ModrinthClient, ProjectVersion, ProjectVersionSummary, SearchResponse},
    storage::{config, paths::PathManager},
};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    io::Read,
    path::{Path, PathBuf},
};
use tauri::State;

const MAX_INDEX_BYTES: u64 = 5 * 1024 * 1024;
const MAX_OVERRIDE_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 20_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModpackIndex {
    format_version: u32,
    game: String,
    version_id: String,
    name: String,
    files: Vec<ModpackFile>,
    dependencies: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModpackFile {
    path: String,
    hashes: ModpackHashes,
    downloads: Vec<String>,
    env: Option<ModpackEnvironment>,
}

#[derive(Debug, Deserialize)]
struct ModpackHashes {
    sha1: String,
    sha512: String,
}

#[derive(Debug, Deserialize)]
struct ModpackEnvironment {
    client: String,
}

#[tauri::command]
pub async fn search_modpacks(
    query: String,
    loader: String,
    offset: u32,
    client: State<'_, ModrinthClient>,
) -> Result<SearchResponse, CommandError> {
    if loader != "fabric" {
        return Err(CommandError::new(
            "Only Fabric modpacks are currently supported.",
        ));
    }
    client
        .search_modpacks(query.trim(), &loader, offset.min(10_000))
        .await
}

#[tauri::command]
pub async fn list_modpack_versions(
    project_id: String,
    loader: String,
    client: State<'_, ModrinthClient>,
) -> Result<Vec<ProjectVersionSummary>, CommandError> {
    validate_modrinth_id(&project_id)?;
    if loader != "fabric" {
        return Err(CommandError::new(
            "Only Fabric modpacks are currently supported.",
        ));
    }
    Ok(client
        .versions_for_loader(&project_id, &loader)
        .await?
        .iter()
        .map(ProjectVersionSummary::from)
        .collect())
}

#[tauri::command]
pub async fn install_modpack(
    project_id: String,
    version_id: Option<String>,
    name: String,
    icon_url: Option<String>,
    paths: State<'_, PathManager>,
    client: State<'_, ModrinthClient>,
) -> Result<InstanceConfig, CommandError> {
    validate_modrinth_id(&project_id)?;
    let version = select_version(&client, &project_id, version_id.as_deref()).await?;
    let pack_file = version
        .files
        .iter()
        .find(|file| file.primary && file.filename.to_ascii_lowercase().ends_with(".mrpack"))
        .or_else(|| {
            version
                .files
                .iter()
                .find(|file| file.filename.to_ascii_lowercase().ends_with(".mrpack"))
        })
        .ok_or_else(|| CommandError::new("This Modrinth version has no .mrpack download."))?;
    let archive_path = paths
        .cache_dir()
        .join("modpacks")
        .join(&project_id)
        .join(format!("{}.mrpack", version.id));
    download_manager::download_one(
        client.http(),
        DownloadSpec {
            url: pack_file.url.clone(),
            path: archive_path.clone(),
            sha1: Some(pack_file.hashes.sha1.clone()),
            sha256: None,
            sha512: Some(pack_file.hashes.sha512.clone()),
        },
    )
    .await?;

    let index = read_index(&archive_path)?;
    validate_index(&index)?;
    let minecraft_version = index
        .dependencies
        .get("minecraft")
        .cloned()
        .expect("validated Minecraft dependency");
    let instance_id = uuid::Uuid::new_v4().simple().to_string();
    let directory = instance_manager::instance_dir(&paths, &instance_id)?;
    std::fs::create_dir_all(&directory)?;
    let settings = config::load(&paths)?;
    let result = install_contents(
        client.http(),
        &archive_path,
        &directory,
        &index,
        settings.download_concurrency as usize,
    )
    .await;
    if let Err(error) = result {
        let _ = std::fs::remove_dir_all(&directory);
        return Err(error);
    }

    let display_name = if name.trim().is_empty() {
        index.name.trim()
    } else {
        name.trim()
    };
    let instance = InstanceConfig {
        id: instance_id,
        name: display_name.to_string(),
        minecraft_version,
        loader: "fabric".into(),
        loader_version: index
            .dependencies
            .get("fabric-loader")
            .cloned()
            .expect("validated Fabric dependency"),
        mods: Vec::new(),
        modpack: Some(InstalledModpack {
            project_id,
            version_id: version.id,
            name: index.name,
            icon_url: icon_url.filter(|url| url.starts_with("https://")),
            file_count: index
                .files
                .iter()
                .filter(|file| {
                    !file
                        .env
                        .as_ref()
                        .is_some_and(|env| env.client == "unsupported")
                })
                .count(),
        }),
    };
    if let Err(error) = instance_manager::save(&paths, &instance) {
        let _ = std::fs::remove_dir_all(&directory);
        return Err(error);
    }
    Ok(instance)
}

async fn select_version(
    client: &ModrinthClient,
    project_id: &str,
    version_id: Option<&str>,
) -> Result<ProjectVersion, CommandError> {
    let version = if let Some(version_id) = version_id {
        validate_modrinth_id(version_id)?;
        client.version(version_id).await?
    } else {
        let mut versions = client.versions_for_loader(project_id, "fabric").await?;
        if versions.is_empty() {
            return Err(CommandError::new("No Fabric modpack version is available."));
        }
        let preferred = versions
            .iter()
            .position(|version| version.version_type == "release")
            .unwrap_or(0);
        versions.remove(preferred)
    };
    if version.project_id != project_id
        || (!version.loaders.is_empty() && !version.loaders.iter().any(|loader| loader == "fabric"))
    {
        return Err(CommandError::new(
            "The selected modpack version is not compatible with Fabric.",
        ));
    }
    Ok(version)
}

fn read_index(archive_path: &Path) -> Result<ModpackIndex, CommandError> {
    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| CommandError::new(format!("Invalid .mrpack archive: {error}")))?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(CommandError::new(
            "The modpack archive contains too many files.",
        ));
    }
    let mut entry = archive
        .by_name("modrinth.index.json")
        .map_err(|_| CommandError::new("The modpack has no modrinth.index.json file."))?;
    if entry.size() > MAX_INDEX_BYTES {
        return Err(CommandError::new(
            "The modpack index is unexpectedly large.",
        ));
    }
    let mut contents = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut contents)?;
    Ok(serde_json::from_slice(&contents)?)
}

fn validate_index(index: &ModpackIndex) -> Result<(), CommandError> {
    if index.format_version != 1 || index.game != "minecraft" || index.version_id.trim().is_empty()
    {
        return Err(CommandError::new(
            "This file does not use the supported Modrinth modpack format.",
        ));
    }
    if !index
        .dependencies
        .get("minecraft")
        .is_some_and(|version| is_valid_version_id(version))
    {
        return Err(CommandError::new(
            "The modpack contains an invalid Minecraft version.",
        ));
    }
    let loader = index
        .dependencies
        .get("fabric-loader")
        .ok_or_else(|| CommandError::new("Only Fabric modpacks are currently supported."))?;
    if loader.is_empty()
        || !loader.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | '+')
        })
    {
        return Err(CommandError::new(
            "The modpack contains an invalid Fabric loader version.",
        ));
    }
    Ok(())
}

async fn install_contents(
    client: &reqwest::Client,
    archive_path: &Path,
    instance_directory: &Path,
    index: &ModpackIndex,
    concurrency: usize,
) -> Result<(), CommandError> {
    let mut destinations = HashSet::new();
    let mut downloads = Vec::new();
    for file in &index.files {
        if file
            .env
            .as_ref()
            .is_some_and(|env| env.client == "unsupported")
        {
            continue;
        }
        let relative = safe_relative_path(&file.path)?;
        reject_reserved_path(&relative)?;
        if !destinations.insert(relative.clone()) {
            return Err(CommandError::new(format!(
                "The modpack contains the path {} more than once.",
                relative.display()
            )));
        }
        let url = file
            .downloads
            .iter()
            .find(|url| allowed_download_url(url))
            .ok_or_else(|| {
                CommandError::new(format!(
                    "The modpack has no trusted download URL for {}.",
                    relative.display()
                ))
            })?;
        downloads.push(DownloadSpec {
            url: url.clone(),
            path: instance_directory.join(relative),
            sha1: Some(file.hashes.sha1.clone()),
            sha256: None,
            sha512: Some(file.hashes.sha512.clone()),
        });
    }
    download_manager::download_all(client, downloads, concurrency).await?;
    extract_overrides(archive_path, instance_directory)?;
    Ok(())
}

fn extract_overrides(archive_path: &Path, instance_directory: &Path) -> Result<(), CommandError> {
    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| CommandError::new(format!("Invalid .mrpack archive: {error}")))?;
    let mut extracted_bytes = 0_u64;
    for prefix in ["overrides", "client-overrides"] {
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| CommandError::new(format!("Invalid .mrpack entry: {error}")))?;
            let Some(enclosed) = entry.enclosed_name() else {
                return Err(CommandError::new(
                    "The modpack contains an unsafe archive path.",
                ));
            };
            let Ok(relative) = enclosed.strip_prefix(prefix) else {
                continue;
            };
            if relative.as_os_str().is_empty() || entry.is_dir() {
                continue;
            }
            if entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                return Err(CommandError::new(
                    "Symbolic links in modpack overrides are not supported.",
                ));
            }
            let relative = safe_relative_path(&relative.to_string_lossy())?;
            reject_reserved_path(&relative)?;
            extracted_bytes = extracted_bytes
                .checked_add(entry.size())
                .ok_or_else(|| CommandError::new("The modpack overrides are too large."))?;
            if extracted_bytes > MAX_OVERRIDE_BYTES {
                return Err(CommandError::new(
                    "The modpack overrides exceed the 1 GiB safety limit.",
                ));
            }
            let destination = instance_directory.join(relative);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::io::copy(&mut entry, &mut std::fs::File::create(destination)?)?;
        }
    }
    Ok(())
}

fn safe_relative_path(value: &str) -> Result<PathBuf, CommandError> {
    if value.is_empty()
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || value.contains(':')
        || value
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(CommandError::new(format!(
            "The modpack contains an unsafe path: {value}"
        )));
    }
    let path = PathBuf::from(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(CommandError::new(format!(
            "The modpack contains an unsafe path: {value}"
        )));
    }
    Ok(path)
}

fn reject_reserved_path(relative: &Path) -> Result<(), CommandError> {
    if relative == Path::new("instance.json")
        || relative == Path::new("mods/.ninjalauncher-managed.json")
    {
        return Err(CommandError::new(format!(
            "The modpack attempted to replace a launcher file: {}",
            relative.display()
        )));
    }
    Ok(())
}

fn allowed_download_url(value: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(value) else {
        return false;
    };
    url.scheme() == "https"
        && matches!(
            url.host_str(),
            Some("cdn.modrinth.com" | "github.com" | "raw.githubusercontent.com" | "gitlab.com")
        )
}

fn validate_modrinth_id(value: &str) -> Result<(), CommandError> {
    if value.is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(CommandError::new("The Modrinth identifier is invalid."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_paths_that_can_escape_an_instance() {
        for path in ["../secret", "/secret", "C:/secret", "mods\\bad.jar", "a//b"] {
            assert!(
                safe_relative_path(path).is_err(),
                "{path} should be rejected"
            );
        }
        assert_eq!(
            safe_relative_path("config/example.toml").expect("safe path"),
            PathBuf::from("config/example.toml")
        );
    }

    #[test]
    fn only_allows_documented_modpack_download_hosts() {
        assert!(allowed_download_url(
            "https://cdn.modrinth.com/data/project/version/mod.jar"
        ));
        assert!(allowed_download_url(
            "https://raw.githubusercontent.com/example/project/main/file.jar"
        ));
        assert!(!allowed_download_url("http://cdn.modrinth.com/file.jar"));
        assert!(!allowed_download_url("https://example.com/file.jar"));
    }

    #[test]
    fn reads_index_and_layers_client_overrides_last() {
        let root = std::env::temp_dir().join(format!(
            "ninja-launcher-mrpack-test-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&root).expect("test directory");
        let archive_path = root.join("example.mrpack");
        let archive_file = std::fs::File::create(&archive_path).expect("archive file");
        let mut archive = zip::ZipWriter::new(archive_file);
        let options = zip::write::SimpleFileOptions::default();
        archive
            .start_file("modrinth.index.json", options)
            .expect("index entry");
        archive
            .write_all(
                br#"{
                    "formatVersion": 1,
                    "game": "minecraft",
                    "versionId": "example-1",
                    "name": "Example Pack",
                    "files": [],
                    "dependencies": {
                        "minecraft": "26.2",
                        "fabric-loader": "0.19.3"
                    }
                }"#,
            )
            .expect("index data");
        archive
            .start_file("overrides/config/example.txt", options)
            .expect("override entry");
        archive.write_all(b"base").expect("override data");
        archive
            .start_file("client-overrides/config/example.txt", options)
            .expect("client override entry");
        archive.write_all(b"client").expect("client override data");
        archive.finish().expect("finish archive");

        let index = read_index(&archive_path).expect("read index");
        validate_index(&index).expect("valid index");
        let instance = root.join("instance");
        std::fs::create_dir_all(&instance).expect("instance directory");
        extract_overrides(&archive_path, &instance).expect("extract overrides");
        assert_eq!(
            std::fs::read_to_string(instance.join("config/example.txt"))
                .expect("installed override"),
            "client"
        );
        std::fs::remove_dir_all(root).expect("remove test directory");
    }
}
