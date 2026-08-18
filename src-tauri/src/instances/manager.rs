use super::instance::{is_valid_version_id, InstanceConfig};
use crate::{error::CommandError, storage::paths::PathManager};
use std::{io::Write, path::PathBuf};

pub fn instance_dir(paths: &PathManager, id: &str) -> Result<PathBuf, CommandError> {
    validate_id(id)?;
    Ok(paths.instances_dir().join(id))
}

pub fn ensure_default(paths: &PathManager) -> Result<(), CommandError> {
    let marker = paths.root().join(".instances-initialized");
    if marker.is_file() {
        return Ok(());
    }
    let instances = list(paths)?;
    if instances.is_empty() {
        save(paths, &InstanceConfig::default_instance())?;
    } else if let Some(mut generated_default) = instances.into_iter().find(|instance| {
        instance.id == "default-fabric"
            && instance.minecraft_version == "1.21.1"
            && instance.loader_version == "0.16.10"
    }) {
        generated_default.minecraft_version = super::instance::DEFAULT_MINECRAFT_VERSION.into();
        generated_default.loader_version = super::instance::DEFAULT_FABRIC_VERSION.into();
        save(paths, &generated_default)?;
    }
    std::fs::write(marker, b"1")?;
    Ok(())
}

pub fn delete(paths: &PathManager, id: &str) -> Result<(), CommandError> {
    let directory = instance_dir(paths, id)?;
    if directory.is_dir() {
        std::fs::remove_dir_all(directory)?;
    }
    Ok(())
}

pub fn list(paths: &PathManager) -> Result<Vec<InstanceConfig>, CommandError> {
    let mut instances = Vec::new();
    if !paths.instances_dir().exists() {
        return Ok(instances);
    }
    for entry in std::fs::read_dir(paths.instances_dir())? {
        let manifest = entry?.path().join("instance.json");
        if manifest.is_file() {
            instances.push(serde_json::from_slice(&std::fs::read(manifest)?)?);
        }
    }
    instances.sort_by_key(|instance| instance.name.to_lowercase());
    Ok(instances)
}

pub fn load(paths: &PathManager, id: &str) -> Result<InstanceConfig, CommandError> {
    let manifest = instance_dir(paths, id)?.join("instance.json");
    if !manifest.is_file() {
        return Err(CommandError::new("The selected instance no longer exists."));
    }
    Ok(serde_json::from_slice(&std::fs::read(manifest)?)?)
}

pub fn save(paths: &PathManager, instance: &InstanceConfig) -> Result<(), CommandError> {
    validate_id(&instance.id)?;
    if !is_valid_version_id(&instance.minecraft_version) {
        return Err(CommandError::new(
            "This Minecraft version is not supported by Ninja Launcher.",
        ));
    }
    validate_version(&instance.loader_version)?;
    if instance.name.trim().is_empty() {
        return Err(CommandError::new("Instance name cannot be empty."));
    }
    let directory = instance_dir(paths, &instance.id)?;
    for child in [
        "mods",
        "config",
        "saves",
        "resourcepacks",
        "shaderpacks",
        "logs",
        "crash-reports",
    ] {
        std::fs::create_dir_all(directory.join(child))?;
    }
    let target = directory.join("instance.json");
    let temporary = directory.join("instance.json.tmp");
    let mut file = std::fs::File::create(&temporary)?;
    file.write_all(&serde_json::to_vec_pretty(instance)?)?;
    file.sync_all()?;
    std::fs::rename(temporary, target)?;
    Ok(())
}

fn validate_version(version: &str) -> Result<(), CommandError> {
    if !is_valid_version_id(version) {
        return Err(CommandError::new(
            "The Minecraft or loader version is invalid.",
        ));
    }
    Ok(())
}

fn validate_id(id: &str) -> Result<(), CommandError> {
    if id.is_empty()
        || !id.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
    {
        return Err(CommandError::new("The instance identifier is invalid."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_path_traversal() {
        assert!(validate_id("../secret").is_err());
    }

    #[test]
    fn deleting_the_last_instance_does_not_recreate_it() {
        let root = std::env::temp_dir().join(format!(
            "ninja-launcher-delete-test-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let paths = PathManager::new(root.clone()).expect("path manager");
        paths.ensure_directories().expect("directories");
        ensure_default(&paths).expect("initial instance");
        let instance = list(&paths).expect("instances").remove(0);
        delete(&paths, &instance.id).expect("delete instance");
        ensure_default(&paths).expect("second initialization check");
        assert!(list(&paths).expect("instances after delete").is_empty());
        std::fs::remove_dir_all(root).expect("remove test directory");
    }
}
