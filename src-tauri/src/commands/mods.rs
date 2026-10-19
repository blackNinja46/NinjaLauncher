use crate::{
    error::CommandError,
    instances::{instance::SelectedMod, manager},
    modrinth::client::{ModrinthClient, ProjectDetails, ProjectVersionSummary, SearchResponse},
    modrinth::resolver,
    storage::paths::PathManager,
};
use std::{collections::HashMap, path::Path};
use tauri::State;

#[derive(serde::Deserialize, Default)]
struct ManagedModsManifest {
    #[serde(default)]
    files: Vec<ManagedModFile>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManagedModFile {
    project_id: String,
    version_id: String,
    #[serde(default)]
    filename: String,
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct LocalModsManifest {
    #[serde(default)]
    files: Vec<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledModFile {
    name: String,
    file_name: String,
    size_bytes: u64,
    project_id: Option<String>,
    version_id: Option<String>,
    enabled: bool,
    local: bool,
}

fn local_mods_manifest_path(mods_directory: &Path) -> std::path::PathBuf {
    mods_directory.join(".ninjalauncher-local-mods.json")
}

fn disabled_local_mods_directory(mods_directory: &Path) -> std::path::PathBuf {
    mods_directory.join(".disabled-local-mods")
}

fn load_local_mods_manifest(mods_directory: &Path) -> Result<LocalModsManifest, CommandError> {
    let path = local_mods_manifest_path(mods_directory);
    if !path.is_file() {
        return Ok(LocalModsManifest::default());
    }
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

fn save_local_mods_manifest(
    mods_directory: &Path,
    manifest: &LocalModsManifest,
) -> Result<(), CommandError> {
    std::fs::create_dir_all(mods_directory)?;
    std::fs::write(
        local_mods_manifest_path(mods_directory),
        serde_json::to_vec_pretty(manifest)?,
    )?;
    Ok(())
}

fn validate_local_mod_filename(file_name: &str) -> Result<(), CommandError> {
    let path = Path::new(file_name);
    if path.file_name().and_then(|value| value.to_str()) != Some(file_name)
        || !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"))
    {
        return Err(CommandError::new("The local mod filename is invalid."));
    }
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModConflictReport {
    compatible: bool,
    message: Option<String>,
    project_ids: Vec<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModRepairReport {
    reset_versions: usize,
    disabled_mods: Vec<String>,
}

#[tauri::command]
pub async fn search_mods(
    query: String,
    minecraft_version: String,
    loader: String,
    offset: u32,
    client: State<'_, ModrinthClient>,
) -> Result<SearchResponse, CommandError> {
    client
        .search(
            query.trim(),
            &minecraft_version,
            &loader,
            offset.min(10_000),
        )
        .await
}

#[tauri::command]
pub async fn search_launcher_content(
    query: String,
    minecraft_version: String,
    offset: u32,
    index: Option<String>,
    client: State<'_, ModrinthClient>,
) -> Result<SearchResponse, CommandError> {
    let index = index.unwrap_or_else(|| "relevance".to_string());
    if !matches!(
        index.as_str(),
        "relevance" | "downloads" | "follows" | "newest" | "updated"
    ) {
        return Err(CommandError::new(
            "The selected Modrinth sorting method is invalid.",
        ));
    }
    client
        .search_launcher_content(query.trim(), &minecraft_version, offset.min(10_000), &index)
        .await
}

#[tauri::command]
pub async fn get_mod_details(
    project_ids: Vec<String>,
    client: State<'_, ModrinthClient>,
) -> Result<Vec<ProjectDetails>, CommandError> {
    if project_ids.len() > 100
        || project_ids.iter().any(|id| {
            id.is_empty()
                || !id.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                })
        })
    {
        return Err(CommandError::new(
            "The requested Modrinth project list is invalid.",
        ));
    }
    client.projects(&project_ids).await
}

#[tauri::command]
pub async fn list_mod_versions(
    project_id: String,
    minecraft_version: String,
    loader: String,
    client: State<'_, ModrinthClient>,
) -> Result<Vec<ProjectVersionSummary>, CommandError> {
    validate_modrinth_id(&project_id)?;
    Ok(client
        .versions(&project_id, &minecraft_version, &loader)
        .await?
        .iter()
        .map(ProjectVersionSummary::from)
        .collect())
}

#[tauri::command]
pub fn get_installed_mod_versions(
    instance_id: String,
    paths: State<'_, PathManager>,
) -> Result<HashMap<String, String>, CommandError> {
    let instance = manager::load(&paths, &instance_id)?;
    let manifest_path = manager::instance_dir(&paths, &instance_id)?
        .join("mods")
        .join(".ninjalauncher-managed.json");
    let managed: ManagedModsManifest = if manifest_path.is_file() {
        serde_json::from_slice(&std::fs::read(manifest_path)?)?
    } else {
        ManagedModsManifest::default()
    };
    let mut versions: HashMap<_, _> = managed
        .files
        .into_iter()
        .map(|file| (file.project_id, file.version_id))
        .collect();
    for selected in &instance.mods {
        if let Some(version_id) = selected.version_overrides.get(&instance.minecraft_version) {
            versions.insert(selected.project_id.clone(), version_id.clone());
        }
    }
    Ok(versions)
}

#[tauri::command]
pub fn list_installed_mod_files(
    instance_id: String,
    paths: State<'_, PathManager>,
) -> Result<Vec<InstalledModFile>, CommandError> {
    let instance = manager::load(&paths, &instance_id)?;
    let mods_directory = manager::instance_dir(&paths, &instance_id)?.join("mods");
    let manifest_path = mods_directory.join(".ninjalauncher-managed.json");
    let managed: ManagedModsManifest = if manifest_path.is_file() {
        serde_json::from_slice(&std::fs::read(manifest_path)?)?
    } else {
        ManagedModsManifest::default()
    };
    let managed_by_filename: HashMap<_, _> = managed
        .files
        .into_iter()
        .filter(|file| !file.filename.is_empty())
        .map(|file| (file.filename.clone(), file))
        .collect();
    let local_manifest = load_local_mods_manifest(&mods_directory)?;

    let mut files = Vec::new();
    if !mods_directory.is_dir() {
        return Ok(files);
    }
    for entry in std::fs::read_dir(&mods_directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_file()
            || !entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"))
        {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let managed_file = managed_by_filename.get(&file_name);
        let local = local_manifest.files.contains(&file_name)
            || (instance.modpack.is_none() && managed_file.is_none());
        let name = Path::new(&file_name)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(&file_name)
            .to_string();
        files.push(InstalledModFile {
            name,
            file_name,
            size_bytes: entry.metadata()?.len(),
            project_id: managed_file.map(|file| file.project_id.clone()),
            version_id: managed_file.map(|file| file.version_id.clone()),
            enabled: true,
            local,
        });
    }
    let disabled_directory = disabled_local_mods_directory(&mods_directory);
    if disabled_directory.is_dir() {
        for entry in std::fs::read_dir(disabled_directory)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if !local_manifest.files.contains(&file_name)
                || validate_local_mod_filename(&file_name).is_err()
            {
                continue;
            }
            let name = Path::new(&file_name)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or(&file_name)
                .to_string();
            files.push(InstalledModFile {
                name,
                file_name,
                size_bytes: entry.metadata()?.len(),
                project_id: None,
                version_id: None,
                enabled: false,
                local: true,
            });
        }
    }
    files.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(files)
}

#[tauri::command]
pub fn add_local_mod_file(
    instance_id: String,
    path: String,
    paths: State<'_, PathManager>,
) -> Result<InstalledModFile, CommandError> {
    let instance = manager::load(&paths, &instance_id)?;
    if instance.loader != "fabric" {
        return Err(CommandError::new(
            "Local mod files can only be added to Fabric profiles.",
        ));
    }
    let source = std::fs::canonicalize(Path::new(&path))?;
    if !source.is_file()
        || !source
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"))
    {
        return Err(CommandError::new("Drop a valid .jar mod file."));
    }
    let metadata = std::fs::metadata(&source)?;
    if metadata.len() == 0 || metadata.len() > 1024 * 1024 * 1024 {
        return Err(CommandError::new(
            "The mod file is empty or larger than 1 GiB.",
        ));
    }
    let file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| CommandError::new("The mod filename is invalid."))?
        .to_string();
    let mods_directory = manager::instance_dir(&paths, &instance_id)?.join("mods");
    std::fs::create_dir_all(&mods_directory)?;
    let destination = mods_directory.join(&file_name);
    if destination.exists()
        || disabled_local_mods_directory(&mods_directory)
            .join(&file_name)
            .exists()
    {
        return Err(CommandError::new(format!(
            "{file_name} is already present in this profile."
        )));
    }
    let temporary = mods_directory.join(format!(".{file_name}.importing"));
    if let Err(error) = std::fs::copy(&source, &temporary) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error.into());
    }
    if let Err(error) = std::fs::rename(&temporary, &destination) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error.into());
    }
    let mut local_manifest = load_local_mods_manifest(&mods_directory)?;
    if !local_manifest.files.contains(&file_name) {
        local_manifest.files.push(file_name.clone());
        local_manifest.files.sort();
        if let Err(error) = save_local_mods_manifest(&mods_directory, &local_manifest) {
            let _ = std::fs::remove_file(&destination);
            return Err(error);
        }
    }
    Ok(InstalledModFile {
        name: source
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(&file_name)
            .to_string(),
        file_name,
        size_bytes: metadata.len(),
        project_id: None,
        version_id: None,
        enabled: true,
        local: true,
    })
}

#[tauri::command]
pub fn set_local_mod_enabled(
    instance_id: String,
    file_name: String,
    enabled: bool,
    paths: State<'_, PathManager>,
) -> Result<(), CommandError> {
    validate_local_mod_filename(&file_name)?;
    let instance = manager::load(&paths, &instance_id)?;
    let mods_directory = manager::instance_dir(&paths, &instance_id)?.join("mods");
    let mut local_manifest = load_local_mods_manifest(&mods_directory)?;
    let managed_manifest_path = mods_directory.join(".ninjalauncher-managed.json");
    let managed: ManagedModsManifest = if managed_manifest_path.is_file() {
        serde_json::from_slice(&std::fs::read(managed_manifest_path)?)?
    } else {
        ManagedModsManifest::default()
    };
    let registered = local_manifest.files.contains(&file_name);
    let unmanaged_legacy_file =
        instance.modpack.is_none() && !managed.files.iter().any(|file| file.filename == file_name);
    if !registered && !unmanaged_legacy_file {
        return Err(CommandError::new(
            "Only locally added mods can be enabled or disabled.",
        ));
    }

    let disabled_directory = disabled_local_mods_directory(&mods_directory);
    let (source, destination) = if enabled {
        (
            disabled_directory.join(&file_name),
            mods_directory.join(&file_name),
        )
    } else {
        std::fs::create_dir_all(&disabled_directory)?;
        (
            mods_directory.join(&file_name),
            disabled_directory.join(&file_name),
        )
    };
    if !source.is_file() {
        return Err(CommandError::new(format!(
            "{file_name} is already {} or no longer exists.",
            if enabled { "enabled" } else { "disabled" }
        )));
    }
    if destination.exists() {
        return Err(CommandError::new(format!(
            "A file named {file_name} already exists at the destination."
        )));
    }
    std::fs::rename(&source, &destination)?;
    if !registered {
        local_manifest.files.push(file_name);
        local_manifest.files.sort();
        save_local_mods_manifest(&mods_directory, &local_manifest)?;
    }
    Ok(())
}

#[tauri::command]
pub fn remove_local_mod_file(
    instance_id: String,
    file_name: String,
    paths: State<'_, PathManager>,
) -> Result<(), CommandError> {
    validate_local_mod_filename(&file_name)?;
    let instance = manager::load(&paths, &instance_id)?;
    let mods_directory = manager::instance_dir(&paths, &instance_id)?.join("mods");
    let mut local_manifest = load_local_mods_manifest(&mods_directory)?;
    let managed_manifest_path = mods_directory.join(".ninjalauncher-managed.json");
    let managed: ManagedModsManifest = if managed_manifest_path.is_file() {
        serde_json::from_slice(&std::fs::read(managed_manifest_path)?)?
    } else {
        ManagedModsManifest::default()
    };
    if !local_manifest.files.contains(&file_name)
        && (instance.modpack.is_some()
            || managed.files.iter().any(|file| file.filename == file_name))
    {
        return Err(CommandError::new("Only locally added mods can be removed."));
    }
    let active = mods_directory.join(&file_name);
    let disabled = disabled_local_mods_directory(&mods_directory).join(&file_name);
    if active.is_file() {
        std::fs::remove_file(active)?;
    } else if disabled.is_file() {
        std::fs::remove_file(disabled)?;
    } else {
        return Err(CommandError::new("The local mod file no longer exists."));
    }
    local_manifest.files.retain(|file| file != &file_name);
    save_local_mods_manifest(&mods_directory, &local_manifest)?;
    Ok(())
}

#[tauri::command]
pub async fn analyze_mod_conflicts(
    instance_id: String,
    paths: State<'_, PathManager>,
    client: State<'_, ModrinthClient>,
) -> Result<ModConflictReport, CommandError> {
    let instance = manager::load(&paths, &instance_id)?;
    let enabled: Vec<_> = instance
        .mods
        .iter()
        .filter(|item| item.enabled_for(&instance.minecraft_version))
        .collect();
    let roots: Vec<_> = enabled.iter().map(|item| item.project_id.clone()).collect();
    let pinned_versions: HashMap<_, _> = enabled
        .iter()
        .filter_map(|item| {
            item.version_overrides
                .get(&instance.minecraft_version)
                .map(|version_id| (item.project_id.clone(), version_id.clone()))
        })
        .collect();

    match resolver::resolve(
        &client,
        &roots,
        &instance.minecraft_version,
        &instance.loader,
        &pinned_versions,
    )
    .await
    {
        Ok(_) => Ok(ModConflictReport {
            compatible: true,
            message: None,
            project_ids: Vec::new(),
        }),
        Err(error) => {
            let message = error.message;
            let normalized = message.to_lowercase();
            let project_ids = enabled
                .iter()
                .filter(|item| {
                    normalized.contains(&item.project_id.to_lowercase())
                        || normalized.contains(&item.name.to_lowercase())
                })
                .map(|item| item.project_id.clone())
                .collect();
            Ok(ModConflictReport {
                compatible: false,
                message: Some(message),
                project_ids,
            })
        }
    }
}

#[tauri::command]
pub async fn repair_mod_configuration(
    instance_id: String,
    paths: State<'_, PathManager>,
    client: State<'_, ModrinthClient>,
) -> Result<ModRepairReport, CommandError> {
    let mut instance = manager::load(&paths, &instance_id)?;
    let minecraft_version = instance.minecraft_version.clone();
    let current_roots: Vec<_> = instance
        .mods
        .iter()
        .filter(|item| item.enabled_for(&minecraft_version))
        .map(|item| item.project_id.clone())
        .collect();
    let current_pins: HashMap<_, _> = instance
        .mods
        .iter()
        .filter(|item| item.enabled_for(&minecraft_version))
        .filter_map(|item| {
            item.version_overrides
                .get(&minecraft_version)
                .map(|version| (item.project_id.clone(), version.clone()))
        })
        .collect();
    if resolver::resolve(
        &client,
        &current_roots,
        &minecraft_version,
        &instance.loader,
        &current_pins,
    )
    .await
    .is_ok()
    {
        return Ok(ModRepairReport {
            reset_versions: 0,
            disabled_mods: Vec::new(),
        });
    }
    let mut reset_versions = 0;
    for selected in &mut instance.mods {
        if selected
            .version_overrides
            .remove(&minecraft_version)
            .is_some()
        {
            reset_versions += 1;
        }
    }
    let mut disabled_mods = Vec::new();
    for _ in 0..=instance.mods.len() {
        let roots: Vec<_> = instance
            .mods
            .iter()
            .filter(|item| item.enabled_for(&instance.minecraft_version))
            .map(|item| item.project_id.clone())
            .collect();
        match resolver::resolve(
            &client,
            &roots,
            &instance.minecraft_version,
            &instance.loader,
            &HashMap::new(),
        )
        .await
        {
            Ok(_) => {
                manager::save(&paths, &instance)?;
                return Ok(ModRepairReport {
                    reset_versions,
                    disabled_mods,
                });
            }
            Err(error) => {
                let normalized = error.message.to_lowercase();
                let candidates: Vec<_> = instance
                    .mods
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| {
                        let name = item.name.to_lowercase();
                        item.enabled_for(&instance.minecraft_version)
                            && (normalized.contains(&item.project_id.to_lowercase())
                                || (name.len() >= 4 && normalized.contains(&name)))
                    })
                    .map(|(index, _)| index)
                    .collect();
                if candidates.len() != 1 {
                    return Err(CommandError::new(format!(
                        "Automatic repair stopped without changing the profile because the affected personal mod could not be identified unambiguously. {error}",
                    )));
                }
                let index = candidates[0];
                instance.mods[index].set_enabled_for(&minecraft_version, false);
                disabled_mods.push(instance.mods[index].name.clone());
            }
        }
    }
    Err(CommandError::new(
        "Automatic repair reached its safety limit without finding a compatible configuration.",
    ))
}

#[tauri::command]
pub async fn add_mod(
    instance_id: String,
    project_id: String,
    name: String,
    version_id: Option<String>,
    paths: State<'_, PathManager>,
    client: State<'_, ModrinthClient>,
) -> Result<(), CommandError> {
    validate_modrinth_id(&project_id)?;
    let mut instance = manager::load(&paths, &instance_id)?;
    if let Some(version_id) = &version_id {
        validate_mod_version(
            &client,
            &project_id,
            version_id,
            &instance.minecraft_version,
            &instance.loader,
        )
        .await?;
    } else {
        validate_automatic_mod_version(
            &client,
            &project_id,
            &name,
            &instance.minecraft_version,
            &instance.loader,
        )
        .await?;
    }
    let minecraft_version = instance.minecraft_version.clone();
    if let Some(existing) = instance
        .mods
        .iter_mut()
        .find(|item| item.project_id == project_id)
    {
        existing.set_enabled_for(&minecraft_version, true);
        if let Some(version_id) = version_id {
            existing
                .version_overrides
                .insert(minecraft_version, version_id);
        }
    } else {
        let mut version_overrides = HashMap::new();
        if let Some(version_id) = version_id {
            version_overrides.insert(minecraft_version, version_id);
        }
        instance.mods.push(SelectedMod {
            project_id,
            name,
            enabled: true,
            disabled_versions: Default::default(),
            enabled_versions: Default::default(),
            version_overrides,
        });
    }
    manager::save(&paths, &instance)
}

#[tauri::command]
pub fn remove_mod(
    instance_id: String,
    project_id: String,
    paths: State<'_, PathManager>,
) -> Result<(), CommandError> {
    let mut instance = manager::load(&paths, &instance_id)?;
    instance.mods.retain(|item| item.project_id != project_id);
    manager::save(&paths, &instance)
}

#[tauri::command]
pub fn set_mod_enabled(
    instance_id: String,
    project_id: String,
    enabled: bool,
    paths: State<'_, PathManager>,
) -> Result<(), CommandError> {
    let mut instance = manager::load(&paths, &instance_id)?;
    let minecraft_version = instance.minecraft_version.clone();
    let selected = instance
        .mods
        .iter_mut()
        .find(|item| item.project_id == project_id)
        .ok_or_else(|| CommandError::new("The selected mod is not installed in this profile."))?;
    selected.set_enabled_for(&minecraft_version, enabled);
    manager::save(&paths, &instance)
}

#[tauri::command]
pub async fn set_mod_version(
    instance_id: String,
    project_id: String,
    minecraft_version: String,
    version_id: Option<String>,
    paths: State<'_, PathManager>,
    client: State<'_, ModrinthClient>,
) -> Result<(), CommandError> {
    validate_modrinth_id(&project_id)?;
    let mut instance = manager::load(&paths, &instance_id)?;
    if minecraft_version != instance.minecraft_version {
        return Err(CommandError::new(
            "The profile's Minecraft version changed. Reload the mod list and try again.",
        ));
    }

    let selected_name = instance
        .mods
        .iter()
        .find(|item| item.project_id == project_id)
        .map(|item| item.name.clone())
        .ok_or_else(|| CommandError::new("The selected mod is not installed in this profile."))?;

    if let Some(version_id) = &version_id {
        validate_mod_version(
            &client,
            &project_id,
            version_id,
            &minecraft_version,
            &instance.loader,
        )
        .await?;
    } else {
        validate_automatic_mod_version(
            &client,
            &project_id,
            &selected_name,
            &minecraft_version,
            &instance.loader,
        )
        .await?;
    }

    let selected = instance
        .mods
        .iter_mut()
        .find(|item| item.project_id == project_id)
        .ok_or_else(|| CommandError::new("The selected mod is not installed in this profile."))?;
    if let Some(version_id) = version_id {
        selected
            .version_overrides
            .insert(minecraft_version, version_id);
    } else {
        selected.version_overrides.remove(&minecraft_version);
    }
    manager::save(&paths, &instance)
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

async fn validate_mod_version(
    client: &ModrinthClient,
    project_id: &str,
    version_id: &str,
    minecraft_version: &str,
    loader: &str,
) -> Result<(), CommandError> {
    validate_modrinth_id(version_id)?;
    let version = client.version(version_id).await?;
    if version.project_id != project_id
        || (!version.game_versions.is_empty()
            && !version
                .game_versions
                .iter()
                .any(|game| game == minecraft_version))
        || (!version.loaders.is_empty() && !version.loaders.iter().any(|item| item == loader))
    {
        return Err(CommandError::new(format!(
            "The selected mod version is not compatible with Minecraft {minecraft_version} and {loader}."
        )));
    }
    Ok(())
}

async fn validate_automatic_mod_version(
    client: &ModrinthClient,
    project_id: &str,
    name: &str,
    minecraft_version: &str,
    loader: &str,
) -> Result<(), CommandError> {
    if client
        .versions(project_id, minecraft_version, loader)
        .await?
        .is_empty()
    {
        let loader_name = if loader.eq_ignore_ascii_case("fabric") {
            "Fabric"
        } else {
            loader
        };
        return Err(CommandError::new(format!(
            "{name} is not available for Minecraft {minecraft_version} with {loader_name}. Choose a compatible project or change the profile version or loader."
        )));
    }
    Ok(())
}
