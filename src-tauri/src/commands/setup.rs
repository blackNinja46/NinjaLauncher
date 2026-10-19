use crate::{error::CommandError, instances::manager, storage::paths::PathManager};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupPaths {
    recommended_installation_path: String,
    minecraft_directory: Option<String>,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftImportReport {
    copied_files: usize,
    skipped_files: usize,
}

#[tauri::command]
pub fn get_setup_paths(paths: State<'_, PathManager>) -> SetupPaths {
    SetupPaths {
        recommended_installation_path: paths.content_root().to_string_lossy().into_owned(),
        minecraft_directory: default_minecraft_directory()
            .filter(|path| path.is_dir())
            .map(|path| path.to_string_lossy().into_owned()),
    }
}

#[tauri::command]
pub fn import_minecraft_data(
    source_path: String,
    instance_id: String,
    paths: State<'_, PathManager>,
) -> Result<MinecraftImportReport, CommandError> {
    let source = PathBuf::from(source_path);
    if !source.is_dir() {
        return Err(CommandError::new(
            "The selected Minecraft directory does not exist.",
        ));
    }
    let destination = manager::instance_dir(&paths, &instance_id)?;
    std::fs::create_dir_all(&destination)?;
    let mut report = MinecraftImportReport::default();

    for directory in [
        "config",
        "mods",
        "resourcepacks",
        "shaderpacks",
        "saves",
        "screenshots",
    ] {
        let from = source.join(directory);
        if from.is_dir() {
            copy_directory_without_overwrite(&from, &destination.join(directory), &mut report)?;
        }
    }
    for file in ["options.txt", "optionsof.txt", "servers.dat"] {
        copy_file_without_overwrite(&source.join(file), &destination.join(file), &mut report)?;
    }
    Ok(report)
}

fn copy_directory_without_overwrite(
    source: &Path,
    destination: &Path,
    report: &mut MinecraftImportReport,
) -> Result<(), CommandError> {
    std::fs::create_dir_all(destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = destination.join(entry.file_name());
        if file_type.is_symlink() {
            report.skipped_files += 1;
        } else if file_type.is_dir() {
            copy_directory_without_overwrite(&entry.path(), &target, report)?;
        } else if file_type.is_file() {
            copy_file_without_overwrite(&entry.path(), &target, report)?;
        }
    }
    Ok(())
}

fn copy_file_without_overwrite(
    source: &Path,
    destination: &Path,
    report: &mut MinecraftImportReport,
) -> Result<(), CommandError> {
    if !source.is_file() {
        return Ok(());
    }
    if destination.exists() {
        report.skipped_files += 1;
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(source, destination)?;
    report.copied_files += 1;
    Ok(())
}

fn default_minecraft_directory() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|path| path.join(".minecraft"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join("Library/Application Support/minecraft"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join(".minecraft"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_preserves_existing_files() {
        let root = std::env::temp_dir().join(format!("ninja-setup-{}", uuid::Uuid::new_v4()));
        let source = root.join("source");
        let destination = root.join("destination");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::write(source.join("options.txt"), b"new").unwrap();
        std::fs::write(destination.join("options.txt"), b"existing").unwrap();
        let mut report = MinecraftImportReport::default();
        copy_file_without_overwrite(
            &source.join("options.txt"),
            &destination.join("options.txt"),
            &mut report,
        )
        .unwrap();
        assert_eq!(
            std::fs::read(destination.join("options.txt")).unwrap(),
            b"existing"
        );
        assert_eq!(report.skipped_files, 1);
        std::fs::remove_dir_all(root).unwrap();
    }
}
