use crate::{
    downloads::manager::{download_one, DownloadSpec},
    error::CommandError,
    instances::{
        instance::{self, InstanceConfig, SupportedVersion},
        manager,
    },
    modrinth::client::{ModrinthClient, ProjectVersionSummary, SearchResponse},
    storage::paths::PathManager,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::{collections::HashSet, io::Read};
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

fn resource_pack_key(file_name: &str) -> String {
    format!("file/{file_name}")
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DisabledResourcePacks {
    #[serde(default)]
    files: HashSet<String>,
}

fn disabled_resource_packs_path(instance_directory: &std::path::Path) -> std::path::PathBuf {
    instance_directory.join(".ninjalauncher-disabled-resourcepacks.json")
}

fn load_disabled_resource_packs(
    instance_directory: &std::path::Path,
) -> Result<DisabledResourcePacks, CommandError> {
    let path = disabled_resource_packs_path(instance_directory);
    if !path.is_file() {
        return Ok(DisabledResourcePacks::default());
    }
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn save_disabled_resource_packs(
    instance_directory: &std::path::Path,
    disabled: &DisabledResourcePacks,
) -> Result<(), CommandError> {
    let target = disabled_resource_packs_path(instance_directory);
    let temporary = instance_directory.join(".ninjalauncher-disabled-resourcepacks.json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(disabled)?)?;
    std::fs::rename(temporary, target)?;
    Ok(())
}

fn selected_resource_packs(instance_directory: &std::path::Path) -> Vec<String> {
    let Ok(options) = std::fs::read_to_string(instance_directory.join("options.txt")) else {
        return Vec::new();
    };
    options
        .lines()
        .find_map(|line| line.strip_prefix("resourcePacks:"))
        .and_then(|value| serde_json::from_str(value).ok())
        .unwrap_or_default()
}

fn write_resource_pack_selection(
    instance_directory: &std::path::Path,
    selected: &[String],
) -> Result<(), CommandError> {
    let target = instance_directory.join("options.txt");
    let existing = std::fs::read_to_string(&target).unwrap_or_default();
    let replacement = format!("resourcePacks:{}", serde_json::to_string(selected)?);
    let mut found = false;
    let mut lines: Vec<String> = existing
        .lines()
        .map(|line| {
            if line.starts_with("resourcePacks:") {
                found = true;
                replacement.clone()
            } else {
                line.to_string()
            }
        })
        .collect();
    if !found {
        lines.push(replacement);
    }
    let mut contents = lines.join("\n");
    contents.push('\n');
    let temporary = instance_directory.join("options.txt.tmp");
    std::fs::write(&temporary, contents)?;
    std::fs::rename(temporary, target)?;
    Ok(())
}

pub(crate) fn sync_resource_pack_selection(
    instance_directory: &std::path::Path,
) -> Result<(), CommandError> {
    let directory = instance_directory.join("resourcepacks");
    std::fs::create_dir_all(&directory)?;
    let disabled = load_disabled_resource_packs(instance_directory)?;
    let current = selected_resource_packs(instance_directory);
    let mut selected: Vec<String> = current
        .iter()
        .filter(|entry| !entry.starts_with("file/"))
        .cloned()
        .collect();
    let mut pack_names = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let is_zip = entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
        if !file_name.starts_with('.') && (metadata.is_dir() || is_zip) {
            pack_names.push(file_name);
        }
    }
    pack_names.sort_by_key(|name| name.to_lowercase());
    selected.extend(
        pack_names
            .into_iter()
            .filter(|name| !disabled.files.contains(name))
            .map(|name| resource_pack_key(&name)),
    );
    if selected != current {
        write_resource_pack_selection(instance_directory, &selected)?;
    }
    Ok(())
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
    loader_version: String,
    paths: State<'_, PathManager>,
) -> Result<InstanceConfig, CommandError> {
    let instance = InstanceConfig {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name,
        minecraft_version,
        loader: "fabric".into(),
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
    std::fs::create_dir_all(&directory)?;
    let managed = load_managed_content(&directory)?;
    let disabled = (content_kind == "resourcepacks")
        .then(|| load_disabled_resource_packs(&instance_directory))
        .transpose()?;
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if file_name.starts_with('.') || file_name.ends_with(".disabled") {
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
        let enabled = disabled
            .as_ref()
            .map(|disabled| !disabled.files.contains(&file_name));
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
            enabled,
        });
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
    if content_kind == "resourcepacks" {
        sync_resource_pack_selection(&instance_directory)?;
    }
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
        enabled: (content_kind == "resourcepacks").then_some(true),
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
    std::fs::create_dir_all(&directory)?;
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

    let mut managed = load_managed_content(&directory)?;
    let previous = managed
        .files
        .iter()
        .find(|entry| entry.project_id == project_id)
        .cloned();
    let resource_pack_was_enabled = if content_kind == "resourcepacks" {
        let disabled = load_disabled_resource_packs(&instance_directory)?;
        !previous
            .as_ref()
            .map(|entry| disabled.files.contains(&entry.file_name))
            .unwrap_or_else(|| disabled.files.contains(&file.filename))
    } else {
        false
    };
    if let Some(previous) = &previous {
        if previous.file_name != file.filename {
            let previous_path = directory.join(&previous.file_name);
            if previous_path.is_file() {
                std::fs::remove_file(previous_path)?;
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

    if content_kind == "resourcepacks" {
        let mut disabled = load_disabled_resource_packs(&instance_directory)?;
        if let Some(previous) = &previous {
            disabled.files.remove(&previous.file_name);
        }
        if !resource_pack_was_enabled {
            disabled.files.insert(file.filename.clone());
        }
        save_disabled_resource_packs(&instance_directory, &disabled)?;
        sync_resource_pack_selection(&instance_directory)?;
    }

    Ok(ProfileContentFile {
        name,
        file_name: file.filename.clone(),
        size_bytes: Some(destination.metadata()?.len()),
        directory: false,
        project_id: Some(project_id),
        version_id: Some(version.id),
        icon_url: (content_kind == "resourcepacks")
            .then(|| pack_icon(&destination))
            .flatten()
            .or(icon_url),
        enabled: (content_kind == "resourcepacks").then_some(resource_pack_was_enabled),
    })
}

#[tauri::command]
pub fn set_resource_pack_enabled(
    instance_id: String,
    file_name: String,
    enabled: bool,
    paths: State<'_, PathManager>,
) -> Result<(), CommandError> {
    safe_content_file_name(&file_name)?;
    let instance_directory = manager::instance_dir(&paths, &instance_id)?;
    let pack = instance_directory.join("resourcepacks").join(&file_name);
    if !pack.exists() {
        return Err(CommandError::new(
            "The selected resource pack no longer exists.",
        ));
    }
    let mut disabled = load_disabled_resource_packs(&instance_directory)?;
    if enabled {
        disabled.files.remove(&file_name);
    } else {
        disabled.files.insert(file_name);
    }
    save_disabled_resource_packs(&instance_directory, &disabled)?;
    sync_resource_pack_selection(&instance_directory)
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
    let directory = manager::instance_dir(&paths, &instance_id)?.join(content_kind);
    let target = directory.join(&file_name);
    if target.is_dir() {
        std::fs::remove_dir_all(target)?;
    } else if target.is_file() {
        std::fs::remove_file(target)?;
    }
    let mut managed = load_managed_content(&directory)?;
    let original_len = managed.files.len();
    managed.files.retain(|entry| entry.file_name != file_name);
    if managed.files.len() != original_len {
        save_managed_content(&directory, &managed)?;
    }
    if content_kind == "resourcepacks" {
        let instance_directory = manager::instance_dir(&paths, &instance_id)?;
        let mut disabled = load_disabled_resource_packs(&instance_directory)?;
        if disabled.files.remove(&file_name) {
            save_disabled_resource_packs(&instance_directory, &disabled)?;
        }
        sync_resource_pack_selection(&instance_directory)?;
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
    use super::{
        load_disabled_resource_packs, save_disabled_resource_packs, selected_resource_packs,
        sync_resource_pack_selection, DisabledResourcePacks,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_instance() -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should move forward")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "ninjalauncher-resourcepacks-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(directory.join("resourcepacks")).expect("resourcepacks directory");
        directory
    }

    #[test]
    fn resource_packs_load_by_default_and_use_a_version_independent_opt_out() {
        let instance = temporary_instance();
        std::fs::write(
            instance.join("options.txt"),
            "resourcePacks:[\"vanilla\"]\nguiScale:3\n",
        )
        .expect("options file");
        std::fs::write(instance.join("resourcepacks").join("A.zip"), [])
            .expect("first resource pack");
        std::fs::create_dir(instance.join("resourcepacks").join("B"))
            .expect("folder resource pack");

        sync_resource_pack_selection(&instance).expect("default selection");
        assert_eq!(
            selected_resource_packs(&instance),
            ["vanilla", "file/A.zip", "file/B"]
        );

        let mut disabled = load_disabled_resource_packs(&instance).expect("disabled packs");
        disabled.files.insert("A.zip".to_string());
        save_disabled_resource_packs(&instance, &disabled).expect("save opt-out");
        std::fs::write(
            instance.join("options.txt"),
            "resourcePacks:[\"vanilla\",\"file/A.zip\",\"file/B\"]\nguiScale:3\n",
        )
        .expect("options after a simulated version switch");

        sync_resource_pack_selection(&instance).expect("selection after version switch");
        assert_eq!(selected_resource_packs(&instance), ["vanilla", "file/B"]);
        assert_eq!(
            load_disabled_resource_packs(&instance)
                .expect("persistent opt-out")
                .files,
            DisabledResourcePacks {
                files: ["A.zip".to_string()].into()
            }
            .files
        );

        std::fs::remove_dir_all(instance).expect("remove test instance");
    }
}
