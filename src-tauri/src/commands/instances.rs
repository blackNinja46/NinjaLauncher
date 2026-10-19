use crate::{
    downloads::manager::{download_one, DownloadSpec},
    error::CommandError,
    instances::{
        instance::{self, InstanceConfig, SelectedMod, SupportedVersion},
        manager,
    },
    modrinth::client::{ModrinthClient, ProjectVersionSummary, SearchResponse},
    storage::paths::PathManager,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::{
    collections::{HashMap, HashSet},
    io::Read,
};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileContentFile {
    name: String,
    file_name: String,
    size_bytes: Option<u64>,
    directory: bool,
    project_id: Option<String>,
    version_id: Option<String>,
    icon_url: Option<String>,
    enabled: Option<bool>,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ManagedProfileContent {
    #[serde(default)]
    files: Vec<ManagedProfileContentFile>,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ManagedProfileContentFile {
    project_id: String,
    version_id: String,
    file_name: String,
    name: String,
    icon_url: Option<String>,
}

fn content_directory<'a>(kind: &'a str) -> Result<&'a str, CommandError> {
    match kind {
        "resourcepacks" | "shaderpacks" => Ok(kind),
        _ => Err(CommandError::new("The profile content type is invalid.")),
    }
}

fn content_project_type(kind: &str) -> Result<&'static str, CommandError> {
    match content_directory(kind)? {
        "resourcepacks" => Ok("resourcepack"),
        "shaderpacks" => Ok("shader"),
        _ => unreachable!(),
    }
}

fn managed_content_path(directory: &std::path::Path) -> std::path::PathBuf {
    directory.join(".ninjalauncher-managed.json")
}

fn load_managed_content(
    directory: &std::path::Path,
) -> Result<ManagedProfileContent, CommandError> {
    let path = managed_content_path(directory);
    if !path.is_file() {
        return Ok(ManagedProfileContent::default());
    }
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn save_managed_content(
    directory: &std::path::Path,
    managed: &ManagedProfileContent,
) -> Result<(), CommandError> {
    let target = managed_content_path(directory);
    let temporary = directory.join(".ninjalauncher-managed.json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(managed)?)?;
    std::fs::rename(temporary, target)?;
    Ok(())
}

fn safe_content_file_name(file_name: &str) -> Result<&str, CommandError> {
    let mut components = std::path::Path::new(file_name).components();
    match (components.next(), components.next()) {
        (Some(std::path::Component::Normal(_)), None) => Ok(file_name),
        _ => Err(CommandError::new(
            "The selected content file name is invalid.",
        )),
    }
}

fn pack_icon(path: &std::path::Path) -> Option<String> {
    const MAX_ICON_BYTES: u64 = 2 * 1024 * 1024;
    let mut bytes = Vec::new();
    if path.is_dir() {
        let icon = path.join("pack.png");
        let metadata = icon.metadata().ok()?;
        if !metadata.is_file() || metadata.len() > MAX_ICON_BYTES {
            return None;
        }
        bytes = std::fs::read(icon).ok()?;
    } else {
        let file = std::fs::File::open(path).ok()?;
        let mut archive = zip::ZipArchive::new(file).ok()?;
        let mut icon = archive.by_name("pack.png").ok()?;
        if icon.size() > MAX_ICON_BYTES {
            return None;
        }
        icon.read_to_end(&mut bytes).ok()?;
    }
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    Some(format!("data:image/png;base64,{}", BASE64.encode(bytes)))
}

fn staged_content_directory(
    instance_directory: &std::path::Path,
    kind: &str,
) -> std::path::PathBuf {
    instance_directory
        .join(".ninjalauncher-disabled-content")
        .join(kind)
}

#[tauri::command]
pub fn list_instances(paths: State<'_, PathManager>) -> Result<Vec<InstanceConfig>, CommandError> {
    manager::list(&paths)
}

#[tauri::command]
pub async fn list_supported_versions() -> Vec<SupportedVersion> {
    instance::supported_versions().await
}

#[tauri::command]
pub async fn list_vanilla_versions() -> Vec<SupportedVersion> {
    instance::vanilla_versions().await
}

#[tauri::command]
pub async fn list_fabric_loader_versions() -> Vec<instance::FabricLoaderVersionOption> {
    instance::loader_versions().await
}

#[tauri::command]
pub fn save_instance(
    instance: InstanceConfig,
    paths: State<'_, PathManager>,
) -> Result<InstanceConfig, CommandError> {
    if let Ok(existing) = manager::load(&paths, &instance.id) {
        if existing.modpack.is_some()
            && (existing.minecraft_version != instance.minecraft_version
                || existing.loader != instance.loader
                || existing.loader_version != instance.loader_version)
        {
            return Err(CommandError::new(
                "The Minecraft and loader versions of an installed modpack are controlled by the pack.",
            ));
        }
    }
    manager::save(&paths, &instance)?;
    Ok(instance)
}

#[tauri::command]
pub fn create_instance(
    name: String,
    minecraft_version: String,
    loader: String,
    loader_version: String,
    paths: State<'_, PathManager>,
) -> Result<InstanceConfig, CommandError> {
    let instance = InstanceConfig {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name,
        minecraft_version,
        loader,
        loader_version,
        mods: Vec::new(),
        modpack: None,
    };
    manager::save(&paths, &instance)?;
    Ok(instance)
}

#[tauri::command]
pub fn duplicate_instance(
    id: String,
    paths: State<'_, PathManager>,
) -> Result<InstanceConfig, CommandError> {
    let source = manager::load(&paths, &id)?;
    let mut copy = source.clone();
    copy.id = uuid::Uuid::new_v4().simple().to_string();
    copy.name = format!("{} Copy", source.name);
    manager::save(&paths, &copy)?;
    let source_directory = manager::instance_dir(&paths, &id)?;
    let copy_directory = manager::instance_dir(&paths, &copy.id)?;
    if let Err(error) = copy_instance_contents(&source_directory, &copy_directory) {
        let _ = manager::delete(&paths, &copy.id);
        return Err(error);
    }
    Ok(copy)
}

#[tauri::command]
pub fn create_editable_modpack_copy(
    id: String,
    paths: State<'_, PathManager>,
) -> Result<InstanceConfig, CommandError> {
    let source = manager::load(&paths, &id)?;
    let mut copy = source.clone();
    detach_modpack(&mut copy)?;
    copy.id = uuid::Uuid::new_v4().simple().to_string();
    copy.name = format!("{} Custom", source.name);
    manager::save(&paths, &copy)?;
    let source_directory = manager::instance_dir(&paths, &id)?;
    let copy_directory = manager::instance_dir(&paths, &copy.id)?;
    if let Err(error) = copy_instance_contents(&source_directory, &copy_directory) {
        let _ = manager::delete(&paths, &copy.id);
        return Err(error);
    }
    Ok(copy)
}

fn detach_modpack(instance: &mut InstanceConfig) -> Result<(), CommandError> {
    let modpack = instance
        .modpack
        .take()
        .ok_or_else(|| CommandError::new("This profile is not managed by a modpack."))?;
    let mut known_projects: HashSet<String> = instance
        .mods
        .iter()
        .map(|selected| selected.project_id.clone())
        .collect();
    for pack_mod in modpack.pack_mods {
        let Some(project_id) = pack_mod.project_id else {
            continue;
        };
        if let Some(selected) = instance
            .mods
            .iter_mut()
            .find(|selected| selected.project_id == project_id)
        {
            if let Some(version_id) = pack_mod.version_id {
                selected
                    .version_overrides
                    .entry(instance.minecraft_version.clone())
                    .or_insert(version_id);
            }
            continue;
        }
        if !known_projects.insert(project_id.clone()) {
            continue;
        }
        let version_overrides = pack_mod
            .version_id
            .map(|version_id| HashMap::from([(instance.minecraft_version.clone(), version_id)]))
            .unwrap_or_default();
        instance.mods.push(SelectedMod {
            project_id,
            name: pack_mod.name,
            enabled: true,
            disabled_versions: HashSet::new(),
            enabled_versions: HashSet::new(),
            version_overrides,
        });
    }
    Ok(())
}

#[tauri::command]
pub fn delete_instance(id: String, paths: State<'_, PathManager>) -> Result<(), CommandError> {
    manager::delete(&paths, &id)
}

#[tauri::command]
pub fn open_instance_folder(
    id: String,
    app: AppHandle,
    paths: State<'_, PathManager>,
) -> Result<(), CommandError> {
    let directory = manager::instance_dir(&paths, &id)?;
    std::fs::create_dir_all(&directory)?;
    app.opener()
        .open_path(directory.to_string_lossy(), None::<String>)
        .map_err(|error| CommandError::new(format!("Could not open the profile folder: {error}")))
}

#[tauri::command]
pub fn list_profile_content(
    instance_id: String,
    kind: String,
    paths: State<'_, PathManager>,
) -> Result<Vec<ProfileContentFile>, CommandError> {
    let instance_directory = manager::instance_dir(&paths, &instance_id)?;
    let content_kind = content_directory(&kind)?;
    let directory = instance_directory.join(content_kind);
    let staged_directory = staged_content_directory(&instance_directory, content_kind);
    std::fs::create_dir_all(&directory)?;
    std::fs::create_dir_all(&staged_directory)?;
    let managed = load_managed_content(&directory)?;
    let mut files = Vec::new();
    for (source_directory, enabled) in [(&directory, true), (&staged_directory, false)] {
        for entry in std::fs::read_dir(source_directory)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if file_name.starts_with('.') {
                continue;
            }
            let name = file_name
                .strip_suffix(".zip")
                .unwrap_or(&file_name)
                .replace(['_', '-'], " ");
            let installation = managed
                .files
                .iter()
                .find(|file| file.file_name == file_name);
            files.push(ProfileContentFile {
                name: installation.map(|file| file.name.clone()).unwrap_or(name),
                file_name,
                size_bytes: metadata.is_file().then_some(metadata.len()),
                directory: metadata.is_dir(),
                project_id: installation.map(|file| file.project_id.clone()),
                version_id: installation.map(|file| file.version_id.clone()),
                icon_url: (content_kind == "resourcepacks")
                    .then(|| pack_icon(&entry.path()))
                    .flatten()
                    .or_else(|| installation.and_then(|file| file.icon_url.clone())),
                enabled: Some(enabled),
            });
        }
    }
    files.sort_by_key(|file| file.name.to_lowercase());
    Ok(files)
}

#[tauri::command]
pub fn add_profile_content(
    instance_id: String,
    kind: String,
    source_path: String,
    paths: State<'_, PathManager>,
) -> Result<ProfileContentFile, CommandError> {
    let content_kind = content_directory(&kind)?;
    let source = std::path::PathBuf::from(source_path);
    if !source.is_file() {
        return Err(CommandError::new(
            "The selected pack file no longer exists.",
        ));
    }
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !extension.eq_ignore_ascii_case("zip") {
        return Err(CommandError::new(
            "Only ZIP packs can be added to a profile.",
        ));
    }
    let file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| CommandError::new("The selected pack file name is invalid."))?;
    safe_content_file_name(file_name)?;
    let instance_directory = manager::instance_dir(&paths, &instance_id)?;
    let directory = instance_directory.join(content_kind);
    std::fs::create_dir_all(&directory)?;
    let destination = directory.join(file_name);
    if destination.exists() {
        return Err(CommandError::new(
            "A pack with this file name already exists in the profile.",
        ));
    }
    let temporary = directory.join(format!(".{file_name}.importing"));
    std::fs::copy(&source, &temporary)?;
    std::fs::rename(&temporary, &destination)?;
    let metadata = destination.metadata()?;
    Ok(ProfileContentFile {
        name: file_name
            .strip_suffix(".zip")
            .unwrap_or(file_name)
            .replace(['_', '-'], " "),
        file_name: file_name.into(),
        size_bytes: Some(metadata.len()),
        directory: false,
        project_id: None,
        version_id: None,
        icon_url: (content_kind == "resourcepacks")
            .then(|| pack_icon(&destination))
            .flatten(),
        enabled: Some(true),
    })
}

#[tauri::command]
pub async fn search_profile_content(
    query: String,
    minecraft_version: String,
    kind: String,
    offset: u32,
    client: State<'_, ModrinthClient>,
) -> Result<SearchResponse, CommandError> {
    let project_type = content_project_type(&kind)?;
    client
        .search_content(
            query.trim(),
            &minecraft_version,
            project_type,
            offset.min(10_000),
        )
        .await
}

#[tauri::command]
pub async fn list_profile_content_versions(
    project_id: String,
    minecraft_version: String,
    kind: String,
    client: State<'_, ModrinthClient>,
) -> Result<Vec<ProjectVersionSummary>, CommandError> {
    content_project_type(&kind)?;
    validate_modrinth_id(&project_id)?;
    Ok(client
        .versions_for_game(&project_id, &minecraft_version)
        .await?
        .iter()
        .map(ProjectVersionSummary::from)
        .collect())
}

#[tauri::command]
pub async fn install_profile_content(
    instance_id: String,
    kind: String,
    project_id: String,
    version_id: Option<String>,
    name: String,
    icon_url: Option<String>,
    paths: State<'_, PathManager>,
    client: State<'_, ModrinthClient>,
) -> Result<ProfileContentFile, CommandError> {
    let content_kind = content_directory(&kind)?;
    let expected_project_type = content_project_type(&kind)?;
    validate_modrinth_id(&project_id)?;
    let instance = manager::load(&paths, &instance_id)?;
    let project = client
        .projects(std::slice::from_ref(&project_id))
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| CommandError::new("The selected Modrinth project no longer exists."))?;
    if project.project_type != expected_project_type {
        return Err(CommandError::new(
            "The selected Modrinth project has the wrong content type.",
        ));
    }
    let version = if let Some(version_id) = version_id {
        validate_modrinth_id(&version_id)?;
        client.version(&version_id).await?
    } else {
        let available = client
            .versions_for_game(&project_id, &instance.minecraft_version)
            .await?;
        available
            .iter()
            .find(|version| version.version_type == "release")
            .or_else(|| available.first())
            .cloned()
            .ok_or_else(|| {
                CommandError::new(format!(
                    "No compatible pack version exists for Minecraft {}.",
                    instance.minecraft_version
                ))
            })?
    };
    if version.project_id != project_id
        || !version.game_versions.contains(&instance.minecraft_version)
    {
        return Err(CommandError::new(
            "The selected Modrinth version is not compatible with this profile.",
        ));
    }
    let file = version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| CommandError::new("The selected Modrinth version has no download."))?;
    safe_content_file_name(&file.filename)?;
    if !file.filename.to_ascii_lowercase().ends_with(".zip") {
        return Err(CommandError::new(
            "The selected Modrinth version does not provide a ZIP pack.",
        ));
    }
    let download_url = reqwest::Url::parse(&file.url)
        .map_err(|_| CommandError::new("Modrinth returned an invalid download URL."))?;
    if download_url.scheme() != "https"
        || !matches!(download_url.host_str(), Some("cdn.modrinth.com"))
    {
        return Err(CommandError::new(
            "Modrinth returned an untrusted download URL.",
        ));
    }
    let instance_directory = manager::instance_dir(&paths, &instance_id)?;
    let directory = instance_directory.join(content_kind);
    let staged_directory = staged_content_directory(&instance_directory, content_kind);
    std::fs::create_dir_all(&directory)?;
    std::fs::create_dir_all(&staged_directory)?;
    let mut managed = load_managed_content(&directory)?;
    let previous = managed
        .files
        .iter()
        .find(|entry| entry.project_id == project_id)
        .cloned();
    let was_enabled = previous
        .as_ref()
        .map(|entry| directory.join(&entry.file_name).exists())
        .unwrap_or(true);
    let destination = directory.join(&file.filename);
    download_one(
        client.http(),
        DownloadSpec {
            url: download_url.into(),
            path: destination.clone(),
            sha1: Some(file.hashes.sha1.clone()),
            sha256: None,
            sha512: Some(file.hashes.sha512.clone()),
        },
    )
    .await?;

    if let Some(previous) = &previous {
        if previous.file_name != file.filename {
            for previous_path in [
                directory.join(&previous.file_name),
                staged_directory.join(&previous.file_name),
            ] {
                if previous_path.is_file() {
                    std::fs::remove_file(previous_path)?;
                }
            }
        }
    }
    managed.files.retain(|entry| entry.project_id != project_id);
    managed.files.push(ManagedProfileContentFile {
        project_id: project_id.clone(),
        version_id: version.id.clone(),
        file_name: file.filename.clone(),
        name: name.clone(),
        icon_url: icon_url.clone(),
    });
    save_managed_content(&directory, &managed)?;
    let final_path = if was_enabled {
        destination
    } else {
        let staged = staged_directory.join(&file.filename);
        if staged.exists() {
            std::fs::remove_file(&staged)?;
        }
        std::fs::rename(&destination, &staged)?;
        staged
    };

    Ok(ProfileContentFile {
        name,
        file_name: file.filename.clone(),
        size_bytes: Some(final_path.metadata()?.len()),
        directory: false,
        project_id: Some(project_id),
        version_id: Some(version.id),
        icon_url: (content_kind == "resourcepacks")
            .then(|| pack_icon(&final_path))
            .flatten()
            .or(icon_url),
        enabled: Some(was_enabled),
    })
}

#[tauri::command]
pub fn set_profile_content_enabled(
    instance_id: String,
    kind: String,
    file_name: String,
    enabled: bool,
    paths: State<'_, PathManager>,
) -> Result<(), CommandError> {
    let content_kind = content_directory(&kind)?;
    safe_content_file_name(&file_name)?;
    let instance_directory = manager::instance_dir(&paths, &instance_id)?;
    let active_directory = instance_directory.join(content_kind);
    let staged_directory = staged_content_directory(&instance_directory, content_kind);
    std::fs::create_dir_all(&active_directory)?;
    std::fs::create_dir_all(&staged_directory)?;
    let source = if enabled {
        staged_directory.join(&file_name)
    } else {
        active_directory.join(&file_name)
    };
    let destination = if enabled {
        active_directory.join(&file_name)
    } else {
        staged_directory.join(&file_name)
    };
    if !source.exists() {
        return Err(CommandError::new(
            "The selected pack is already in the requested state or no longer exists.",
        ));
    }
    if destination.exists() {
        return Err(CommandError::new(
            "A pack with this name already exists in the destination folder.",
        ));
    }
    std::fs::rename(source, destination)?;
    Ok(())
}

#[tauri::command]
pub fn remove_profile_content(
    instance_id: String,
    kind: String,
    file_name: String,
    paths: State<'_, PathManager>,
) -> Result<(), CommandError> {
    let content_kind = content_directory(&kind)?;
    safe_content_file_name(&file_name)?;
    let instance_directory = manager::instance_dir(&paths, &instance_id)?;
    let directory = instance_directory.join(content_kind);
    for target in [
        directory.join(&file_name),
        staged_content_directory(&instance_directory, content_kind).join(&file_name),
    ] {
        if target.is_dir() {
            std::fs::remove_dir_all(target)?;
        } else if target.is_file() {
            std::fs::remove_file(target)?;
        }
    }
    let mut managed = load_managed_content(&directory)?;
    let original_len = managed.files.len();
    managed.files.retain(|entry| entry.file_name != file_name);
    if managed.files.len() != original_len {
        save_managed_content(&directory, &managed)?;
    }
    Ok(())
}

#[tauri::command]
pub fn open_profile_content_folder(
    instance_id: String,
    kind: String,
    app: AppHandle,
    paths: State<'_, PathManager>,
) -> Result<(), CommandError> {
    let directory = manager::instance_dir(&paths, &instance_id)?.join(content_directory(&kind)?);
    std::fs::create_dir_all(&directory)?;
    app.opener()
        .open_path(directory.to_string_lossy(), None::<String>)
        .map_err(|error| CommandError::new(format!("Could not open the content folder: {error}")))
}

fn copy_dir_contents(
    source: &std::path::Path,
    target: &std::path::Path,
) -> Result<(), CommandError> {
    if !source.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let destination = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_contents(&entry.path(), &destination)?;
        } else {
            std::fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

fn copy_instance_contents(
    source: &std::path::Path,
    target: &std::path::Path,
) -> Result<(), CommandError> {
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if matches!(
            name.as_ref(),
            "instance.json" | "instance.json.tmp" | "logs" | "crash-reports"
        ) {
            continue;
        }
        let destination = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_contents(&entry.path(), &destination)?;
        } else {
            std::fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

fn validate_modrinth_id(id: &str) -> Result<(), CommandError> {
    if id.is_empty()
        || !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(CommandError::new("The Modrinth identifier is invalid."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{detach_modpack, staged_content_directory, InstanceConfig};

    #[test]
    fn disabled_packs_are_staged_outside_the_minecraft_content_folder() {
        let instance = std::path::Path::new("/instance");
        assert_eq!(
            staged_content_directory(instance, "resourcepacks"),
            instance
                .join(".ninjalauncher-disabled-content")
                .join("resourcepacks")
        );
    }

    #[test]
    fn editable_copy_converts_pack_mods_to_profile_mods() {
        let mut instance: InstanceConfig = serde_json::from_str(
            r#"{
                "id":"pack",
                "name":"Pack",
                "minecraftVersion":"1.21.1",
                "loader":"fabric",
                "loaderVersion":"0.16.10",
                "mods":[],
                "modpack":{
                    "projectId":"pack-project",
                    "versionId":"pack-version",
                    "name":"Pack",
                    "iconUrl":null,
                    "fileCount":1,
                    "packMods":[{
                        "projectId":"sodium",
                        "versionId":"sodium-version",
                        "name":"Sodium",
                        "versionName":"Sodium 1.0",
                        "iconUrl":null
                    }]
                }
            }"#,
        )
        .expect("modpack profile");

        detach_modpack(&mut instance).expect("editable copy");

        assert!(instance.modpack.is_none());
        assert_eq!(instance.mods.len(), 1);
        assert_eq!(instance.mods[0].project_id, "sodium");
        assert_eq!(
            instance.mods[0].version_overrides.get("1.21.1"),
            Some(&"sodium-version".to_string())
        );
    }
}
