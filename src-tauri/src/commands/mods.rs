use crate::{
    error::CommandError,
    instances::{instance::SelectedMod, manager},
    modrinth::client::{ModrinthClient, ProjectDetails, ProjectVersionSummary, SearchResponse},
    modrinth::resolver,
    storage::paths::PathManager,
};
use std::collections::HashMap;
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
pub async fn analyze_mod_conflicts(
    instance_id: String,
    paths: State<'_, PathManager>,
    client: State<'_, ModrinthClient>,
) -> Result<ModConflictReport, CommandError> {
    let instance = manager::load(&paths, &instance_id)?;
    let enabled: Vec<_> = instance.mods.iter().filter(|item| item.enabled).collect();
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
    loop {
        let roots: Vec<_> = instance
            .mods
            .iter()
            .filter(|item| item.enabled)
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
                let candidate = instance.mods.iter().rposition(|item| {
                    let name = item.name.to_lowercase();
                    item.enabled
                        && (normalized.contains(&item.project_id.to_lowercase())
                            || (name.len() >= 4 && normalized.contains(&name)))
                });
                let Some(index) = candidate else {
                    return Err(CommandError::new(format!(
                        "Automatic Fabric repair could not safely identify the affected personal mod. {error}",
                    )));
                };
                instance.mods[index].enabled = false;
                disabled_mods.push(instance.mods[index].name.clone());
            }
        }
    }
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
    }
    let minecraft_version = instance.minecraft_version.clone();
    if let Some(existing) = instance
        .mods
        .iter_mut()
        .find(|item| item.project_id == project_id)
    {
        existing.enabled = true;
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
    let selected = instance
        .mods
        .iter_mut()
        .find(|item| item.project_id == project_id)
        .ok_or_else(|| CommandError::new("The selected mod is not installed in this profile."))?;
    selected.enabled = enabled;
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

    if let Some(version_id) = &version_id {
        validate_mod_version(
            &client,
            &project_id,
            version_id,
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
