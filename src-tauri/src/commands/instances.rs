use crate::{
    error::CommandError,
    instances::{
        instance::{self, InstanceConfig, SupportedVersion},
        manager,
    },
    storage::paths::PathManager,
};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub fn list_instances(paths: State<'_, PathManager>) -> Result<Vec<InstanceConfig>, CommandError> {
    manager::list(&paths)
}

#[tauri::command]
pub fn list_supported_versions() -> Vec<SupportedVersion> {
    instance::supported_versions()
}

#[tauri::command]
pub fn save_instance(
    instance: InstanceConfig,
    paths: State<'_, PathManager>,
) -> Result<InstanceConfig, CommandError> {
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
    for child in ["config", "saves", "resourcepacks", "shaderpacks"] {
        copy_dir_contents(&source_directory.join(child), &copy_directory.join(child))?;
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
        .map_err(|error| CommandError::new(format!("Could not open the instance folder: {error}")))
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
