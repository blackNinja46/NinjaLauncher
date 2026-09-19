use crate::{
    downloads::manager::{self as download_manager, DownloadSpec},
    error::CommandError,
    instances::{
        instance::{is_valid_version_id, InstalledModpack, InstanceConfig, PackMod},
        manager as instance_manager,
    },
    modrinth::{
        client::{ModrinthClient, ProjectVersion, ProjectVersionSummary, SearchResponse},
        resolver,
    },
    storage::{config, paths::PathManager},
};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use sha2::Sha512;
use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tauri::State;

const MAX_INDEX_BYTES: u64 = 5 * 1024 * 1024;
const MAX_OVERRIDE_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 20_000;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModpackIndex {
    format_version: u32,
    game: String,
    version_id: String,
    name: String,
    files: Vec<ModpackFile>,
    dependencies: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModpackFile {
    path: String,
    hashes: ModpackHashes,
    downloads: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    file_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<ModpackEnvironment>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ModpackHashes {
    sha1: String,
    sha512: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ModpackEnvironment {
    client: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackFileInspection {
    path: String,
    name: String,
    version_id: String,
    minecraft_version: String,
    loader_version: String,
    file_count: usize,
    foreign_mods: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackExportReport {
    path: String,
    file_count: usize,
    foreign_mods: Vec<String>,
}

struct ModpackModAnalysis {
    foreign_mods: Vec<String>,
    versions: HashMap<String, ProjectVersion>,
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
            // Packs installed from Modrinth stay represented as one managed pack.
            pack_mods: Vec::new(),
        }),
    };
    if let Err(error) = instance_manager::save(&paths, &instance) {
        let _ = std::fs::remove_dir_all(&directory);
        return Err(error);
    }
    Ok(instance)
}

#[tauri::command]
pub async fn inspect_modpack_file(
    path: String,
    client: State<'_, ModrinthClient>,
) -> Result<ModpackFileInspection, CommandError> {
    let archive_path = validated_mrpack_path(&path, false)?;
    let index = read_index(&archive_path)?;
    validate_index(&index)?;
    let analysis = analyze_modpack_mods(&archive_path, &index, &client).await?;
    Ok(ModpackFileInspection {
        path: archive_path.to_string_lossy().into_owned(),
        name: index.name.clone(),
        version_id: index.version_id.clone(),
        minecraft_version: index.dependencies["minecraft"].clone(),
        loader_version: index.dependencies["fabric-loader"].clone(),
        file_count: index.files.len(),
        foreign_mods: analysis.foreign_mods,
    })
}

#[tauri::command]
pub async fn import_modpack_file(
    path: String,
    allow_foreign_mods: bool,
    paths: State<'_, PathManager>,
    client: State<'_, ModrinthClient>,
) -> Result<InstanceConfig, CommandError> {
    let archive_path = validated_mrpack_path(&path, false)?;
    let index = read_index(&archive_path)?;
    validate_index(&index)?;
    let analysis = analyze_modpack_mods(&archive_path, &index, &client).await?;
    if !analysis.foreign_mods.is_empty() && !allow_foreign_mods {
        return Err(CommandError::new(
            "The modpack contains mods that could not be verified with Modrinth. Confirm the warning before importing.",
        ));
    }

    let instance_id = uuid::Uuid::new_v4().simple().to_string();
    let directory = instance_manager::instance_dir(&paths, &instance_id)?;
    std::fs::create_dir_all(&directory)?;
    let settings = config::load(&paths)?;
    if let Err(error) = install_contents(
        client.http(),
        &archive_path,
        &directory,
        &index,
        settings.download_concurrency as usize,
    )
    .await
    {
        let _ = std::fs::remove_dir_all(&directory);
        return Err(error);
    }
    let mut pack_mods = pack_mods_from_versions(&client, analysis.versions).await?;
    pack_mods.extend(
        analysis
            .foreign_mods
            .into_iter()
            .map(pack_mod_from_filename),
    );
    pack_mods.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    let instance = InstanceConfig {
        id: instance_id,
        name: index.name.trim().to_string(),
        minecraft_version: index.dependencies["minecraft"].clone(),
        loader: "fabric".into(),
        loader_version: index.dependencies["fabric-loader"].clone(),
        mods: Vec::new(),
        modpack: Some(InstalledModpack {
            project_id: "local-import".into(),
            version_id: index.version_id.clone(),
            name: index.name.clone(),
            icon_url: None,
            file_count: index.files.len(),
            pack_mods,
        }),
    };
    if let Err(error) = instance_manager::save(&paths, &instance) {
        let _ = std::fs::remove_dir_all(&directory);
        return Err(error);
    }
    Ok(instance)
}

#[tauri::command]
pub async fn refresh_imported_modpack_mods(
    instance_id: String,
    paths: State<'_, PathManager>,
    client: State<'_, ModrinthClient>,
) -> Result<InstanceConfig, CommandError> {
    let mut instance = instance_manager::load(&paths, &instance_id)?;
    let is_local_import = instance
        .modpack
        .as_ref()
        .is_some_and(|modpack| modpack.project_id == "local-import");
    if !is_local_import {
        return Ok(instance);
    }
    let mods_directory = instance_manager::instance_dir(&paths, &instance_id)?.join("mods");
    let mut files_by_hash = HashMap::new();
    if mods_directory.is_dir() {
        for entry in std::fs::read_dir(&mods_directory)? {
            let entry = entry?;
            if entry.file_type()?.is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"))
            {
                let sha1 = file_hashes(&entry.path())?.0;
                files_by_hash.insert(sha1, entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    let hashes: Vec<_> = files_by_hash.keys().cloned().collect();
    let matched = versions_for_hashes(&client, &hashes).await?;
    let known_hashes: HashSet<_> = matched
        .keys()
        .map(|hash| hash.to_ascii_lowercase())
        .collect();
    let versions = matched
        .into_values()
        .map(|version| (version.project_id.clone(), version))
        .collect();
    let mut pack_mods = pack_mods_from_versions(&client, versions).await?;
    pack_mods.extend(
        files_by_hash
            .into_iter()
            .filter(|(hash, _)| !known_hashes.contains(&hash.to_ascii_lowercase()))
            .map(|(_, filename)| pack_mod_from_filename(filename)),
    );
    pack_mods.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    if let Some(modpack) = instance.modpack.as_mut() {
        modpack.pack_mods = pack_mods;
    }
    instance_manager::save(&paths, &instance)?;
    Ok(instance)
}

#[tauri::command]
pub fn list_exportable_folders(
    instance_id: String,
    paths: State<'_, PathManager>,
) -> Result<Vec<String>, CommandError> {
    let directory = instance_manager::instance_dir(&paths, &instance_id)?;
    // Loading first ensures the requested instance still exists.
    instance_manager::load(&paths, &instance_id)?;
    exportable_folders(&directory)
}

#[tauri::command]
pub async fn export_modpack(
    instance_id: String,
    destination: String,
    pack_name: String,
    version_id: String,
    included_folders: Vec<String>,
    paths: State<'_, PathManager>,
    client: State<'_, ModrinthClient>,
) -> Result<ModpackExportReport, CommandError> {
    let instance = instance_manager::load(&paths, &instance_id)?;
    let instance_directory = instance_manager::instance_dir(&paths, &instance_id)?;
    let pack_name = pack_name.trim();
    if pack_name.is_empty() || pack_name.len() > 128 {
        return Err(CommandError::new(
            "The modpack name must contain between 1 and 128 characters.",
        ));
    }
    let version_id = version_id.trim().to_string();
    if !is_valid_version_id(&version_id) {
        return Err(CommandError::new(
            "Use a version number containing only letters, numbers, dots, dashes, underscores, or plus signs.",
        ));
    }
    let available_folders: HashSet<_> = exportable_folders(&instance_directory)?
        .into_iter()
        .collect();
    let included_folders: HashSet<_> = included_folders.into_iter().collect();
    if !included_folders.is_subset(&available_folders) {
        return Err(CommandError::new(
            "The export contains an invalid or unavailable folder selection.",
        ));
    }
    let destination = validated_mrpack_path(&destination, true)?;
    if destination.starts_with(&instance_directory) {
        return Err(CommandError::new(
            "Choose an export location outside the profile folder.",
        ));
    }
    let files = collect_export_files(&instance_directory, &included_folders)?;
    let mod_files: Vec<_> = files
        .iter()
        .filter(|(relative, _)| is_mod_jar(relative))
        .cloned()
        .collect();
    let mut hashes = HashMap::new();
    for (relative, absolute) in &mod_files {
        let (sha1, sha512) = file_hashes(absolute)?;
        hashes.insert(relative.clone(), (sha1, sha512));
    }
    let sha1s: Vec<_> = hashes.values().map(|hashes| hashes.0.clone()).collect();
    let matched = versions_for_hashes(&client, &sha1s).await?;
    let matched: HashMap<_, _> = matched
        .into_iter()
        .map(|(hash, version)| (hash.to_ascii_lowercase(), version))
        .collect();

    let include_mods = included_folders.contains("mods");
    let roots: Vec<_> = if include_mods {
        instance
            .mods
            .iter()
            .filter(|item| item.enabled_for(&instance.minecraft_version))
            .map(|item| item.project_id.clone())
            .collect()
    } else {
        Vec::new()
    };
    let pinned_versions: HashMap<_, _> = if include_mods {
        instance
            .mods
            .iter()
            .filter(|item| item.enabled_for(&instance.minecraft_version))
            .filter_map(|item| {
                item.version_overrides
                    .get(&instance.minecraft_version)
                    .map(|version_id| (item.project_id.clone(), version_id.clone()))
            })
            .collect()
    } else {
        HashMap::new()
    };
    let resolved = resolver::resolve(
        &client,
        &roots,
        &instance.minecraft_version,
        &instance.loader,
        &pinned_versions,
    )
    .await?;
    let mut index_files: HashMap<String, ModpackFile> = HashMap::new();
    for version in resolved {
        let source = version
            .files
            .iter()
            .find(|file| file.primary)
            .or_else(|| version.files.first())
            .ok_or_else(|| CommandError::new(format!("Mod {} has no file.", version.name)))?;
        let filename = Path::new(&source.filename)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| *value == source.filename && value.ends_with(".jar"))
            .ok_or_else(|| CommandError::new("Modrinth returned an unsafe mod filename."))?;
        let path = format!("mods/{filename}");
        if index_files.contains_key(&path) {
            return Err(CommandError::new(format!(
                "Two configured mods use the same filename: {filename}."
            )));
        }
        index_files.insert(
            path.clone(),
            ModpackFile {
                path,
                hashes: ModpackHashes {
                    sha1: source.hashes.sha1.clone(),
                    sha512: source.hashes.sha512.clone(),
                },
                downloads: vec![source.url.clone()],
                file_size: (source.size > 0).then_some(source.size),
                env: None,
            },
        );
    }
    let mut foreign_mods = Vec::new();
    for (relative, _) in &mod_files {
        let (sha1, sha512) = &hashes[relative];
        let path = path_for_archive(relative)?;
        if index_files.contains_key(&path) {
            continue;
        }
        if let Some(version) = matched.get(&sha1.to_ascii_lowercase()) {
            let source = version
                .files
                .iter()
                .find(|file| file.hashes.sha1.eq_ignore_ascii_case(sha1))
                .ok_or_else(|| CommandError::new("Modrinth returned incomplete file metadata."))?;
            index_files.entry(path.clone()).or_insert(ModpackFile {
                path,
                hashes: ModpackHashes {
                    sha1: sha1.clone(),
                    sha512: sha512.clone(),
                },
                downloads: vec![source.url.clone()],
                file_size: (source.size > 0).then_some(source.size),
                env: None,
            });
        } else {
            foreign_mods.push(file_name(relative));
        }
    }
    foreign_mods.sort();

    let mut index_files: Vec<_> = index_files.into_values().collect();
    index_files.sort_by(|left, right| left.path.cmp(&right.path));
    let index = ModpackIndex {
        format_version: 1,
        game: "minecraft".into(),
        version_id,
        name: pack_name.to_string(),
        files: index_files,
        dependencies: HashMap::from([
            ("minecraft".into(), instance.minecraft_version.clone()),
            ("fabric-loader".into(), instance.loader_version.clone()),
        ]),
    };
    write_export_archive(&destination, &files, &hashes, &matched, &index)?;
    Ok(ModpackExportReport {
        path: destination.to_string_lossy().into_owned(),
        file_count: files.len(),
        foreign_mods,
    })
}

fn validated_mrpack_path(value: &str, may_not_exist: bool) -> Result<PathBuf, CommandError> {
    let path = PathBuf::from(value);
    if path.extension().and_then(|value| value.to_str()) != Some("mrpack") {
        return Err(CommandError::new(
            "Select a file with the .mrpack extension.",
        ));
    }
    if may_not_exist {
        let parent = path
            .parent()
            .ok_or_else(|| CommandError::new("The export destination is invalid."))?
            .canonicalize()?;
        let name = path
            .file_name()
            .ok_or_else(|| CommandError::new("The export destination is invalid."))?;
        Ok(parent.join(name))
    } else {
        let path = path.canonicalize()?;
        if !path.is_file() {
            return Err(CommandError::new(
                "The selected modpack file does not exist.",
            ));
        }
        Ok(path)
    }
}

async fn analyze_modpack_mods(
    archive_path: &Path,
    index: &ModpackIndex,
    client: &ModrinthClient,
) -> Result<ModpackModAnalysis, CommandError> {
    let mut mods_by_hash: HashMap<String, Vec<String>> = HashMap::new();
    for file in &index.files {
        if file
            .env
            .as_ref()
            .is_some_and(|environment| environment.client == "unsupported")
        {
            continue;
        }
        let relative = safe_relative_path(&file.path)?;
        if is_mod_jar(&relative) {
            mods_by_hash
                .entry(file.hashes.sha1.to_ascii_lowercase())
                .or_default()
                .push(file_name(&relative));
        }
    }

    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| CommandError::new(format!("Invalid .mrpack archive: {error}")))?;
    let mut embedded_bytes = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| CommandError::new(format!("Invalid .mrpack entry: {error}")))?;
        if entry.is_dir() {
            continue;
        }
        let Some(enclosed) = entry.enclosed_name() else {
            return Err(CommandError::new(
                "The modpack contains an unsafe archive path.",
            ));
        };
        let relative = ["overrides", "client-overrides"]
            .iter()
            .find_map(|prefix| enclosed.strip_prefix(prefix).ok());
        let Some(relative) = relative else { continue };
        let relative = safe_relative_path(&relative.to_string_lossy())?;
        if !is_mod_jar(&relative) {
            continue;
        }
        embedded_bytes = embedded_bytes
            .checked_add(entry.size())
            .ok_or_else(|| CommandError::new("The embedded mods are too large."))?;
        if embedded_bytes > MAX_OVERRIDE_BYTES {
            return Err(CommandError::new(
                "The embedded mods exceed the 1 GiB safety limit.",
            ));
        }
        let (sha1, _) = reader_hashes(&mut entry)?;
        mods_by_hash
            .entry(sha1)
            .or_default()
            .push(file_name(&relative));
    }

    let hashes: Vec<_> = mods_by_hash.keys().cloned().collect();
    let matched = versions_for_hashes(client, &hashes).await?;
    let known: HashSet<_> = matched
        .keys()
        .map(|hash| hash.to_ascii_lowercase())
        .collect();
    let mut foreign: Vec<_> = mods_by_hash
        .into_iter()
        .filter(|(hash, _)| !known.contains(hash))
        .flat_map(|(_, names)| names)
        .collect();
    foreign.sort();
    foreign.dedup();
    let versions = matched
        .into_values()
        .map(|version| (version.project_id.clone(), version))
        .collect();
    Ok(ModpackModAnalysis {
        foreign_mods: foreign,
        versions,
    })
}

async fn pack_mods_from_versions(
    client: &ModrinthClient,
    versions: HashMap<String, ProjectVersion>,
) -> Result<Vec<PackMod>, CommandError> {
    let project_ids: Vec<_> = versions.keys().cloned().collect();
    let mut project_details = HashMap::new();
    for chunk in project_ids.chunks(100) {
        for project in client.projects(chunk).await? {
            project_details.insert(project.id.clone(), project);
        }
    }
    let mut mods: Vec<_> = versions
        .into_values()
        .map(|version| {
            let project = project_details.get(&version.project_id);
            PackMod {
                project_id: Some(version.project_id),
                version_id: Some(version.id),
                name: project
                    .map(|project| project.title.clone())
                    .unwrap_or_else(|| version.name.clone()),
                version_name: version.name,
                icon_url: project.and_then(|project| project.icon_url.clone()),
            }
        })
        .collect();
    mods.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(mods)
}

fn pack_mod_from_filename(filename: String) -> PackMod {
    let name = filename
        .strip_suffix(".jar")
        .unwrap_or(&filename)
        .to_string();
    PackMod {
        project_id: None,
        version_id: None,
        name,
        version_name: filename,
        icon_url: None,
    }
}

async fn versions_for_hashes(
    client: &ModrinthClient,
    hashes: &[String],
) -> Result<HashMap<String, ProjectVersion>, CommandError> {
    let mut result = HashMap::new();
    for chunk in hashes.chunks(100) {
        result.extend(client.versions_by_hashes(chunk).await?);
    }
    Ok(result)
}

fn exportable_folders(root: &Path) -> Result<Vec<String>, CommandError> {
    let mut folders = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if !is_private_runtime_folder(&name) {
            folders.push(name);
        }
    }
    folders.sort_by_key(|name| (name != "mods", name != "config", name.to_ascii_lowercase()));
    Ok(folders)
}

fn is_private_runtime_folder(name: &str) -> bool {
    matches!(name, "saves" | "logs" | "crash-reports")
}

fn collect_export_files(
    root: &Path,
    included_folders: &HashSet<String>,
) -> Result<Vec<(PathBuf, PathBuf)>, CommandError> {
    fn visit(
        root: &Path,
        directory: &Path,
        included_folders: &HashSet<String>,
        output: &mut Vec<(PathBuf, PathBuf)>,
    ) -> Result<(), CommandError> {
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|_| CommandError::new("Could not prepare the profile export."))?
                .to_path_buf();
            let first = relative.components().next().and_then(|part| match part {
                std::path::Component::Normal(value) => value.to_str(),
                _ => None,
            });
            if first.is_some_and(is_private_runtime_folder)
                || relative == Path::new("instance.json")
                || relative == Path::new("instance.json.tmp")
                || relative == Path::new("mods/.ninjalauncher-managed.json")
                || relative == Path::new("resourcepacks/.ninjalauncher-managed.json")
                || relative == Path::new("shaderpacks/.ninjalauncher-managed.json")
            {
                continue;
            }
            if file_type.is_symlink() {
                return Err(CommandError::new(format!(
                    "Symbolic links cannot be exported: {}",
                    relative.display()
                )));
            }
            if file_type.is_dir() {
                if directory == root && !included_folders.contains(first.unwrap_or_default()) {
                    continue;
                }
                visit(root, &entry.path(), included_folders, output)?;
            } else if file_type.is_file() {
                output.push((relative, entry.path()));
            }
        }
        Ok(())
    }
    let mut output = Vec::new();
    visit(root, root, included_folders, &mut output)?;
    output.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(output)
}

fn write_export_archive(
    destination: &Path,
    files: &[(PathBuf, PathBuf)],
    hashes: &HashMap<PathBuf, (String, String)>,
    matched: &HashMap<String, ProjectVersion>,
    index: &ModpackIndex,
) -> Result<(), CommandError> {
    let result = (|| -> Result<(), CommandError> {
        // Save dialogs grant access to the selected file itself. Writing to a sibling
        // `.part` path and renaming it can fail on sandboxed macOS/Windows setups.
        let target = std::fs::File::create(destination)?;
        let mut archive = zip::ZipWriter::new(target);
        // Stored entries are larger but have the broadest ZIP-reader compatibility.
        // Nested JARs/resource packs are already compressed, so deflating them again
        // provides little benefit and has caused imports to fail in strict readers.
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        archive
            .start_file("modrinth.index.json", options)
            .map_err(|error| CommandError::new(format!("Could not create modpack: {error}")))?;
        archive.write_all(&serde_json::to_vec_pretty(index)?)?;
        for (relative, absolute) in files {
            let archive_path = path_for_archive(relative)?;
            let recognized_mod = index.files.iter().any(|file| file.path == archive_path)
                || hashes
                    .get(relative)
                    .is_some_and(|(sha1, _)| matched.contains_key(&sha1.to_ascii_lowercase()));
            if recognized_mod {
                continue;
            }
            let name = format!("overrides/{}", path_for_archive(relative)?);
            archive
                .start_file(name, options)
                .map_err(|error| CommandError::new(format!("Could not create modpack: {error}")))?;
            let mut source = std::fs::File::open(absolute)?;
            std::io::copy(&mut source, &mut archive)?;
        }
        let target = archive
            .finish()
            .map_err(|error| CommandError::new(format!("Could not finish modpack: {error}")))?;
        target.sync_all()?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = std::fs::remove_file(destination);
        return Err(error);
    }
    Ok(())
}

fn is_mod_jar(relative: &Path) -> bool {
    relative
        .parent()
        .is_some_and(|parent| parent == Path::new("mods"))
        && relative
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("jar"))
}

fn file_name(relative: &Path) -> String {
    relative
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

fn path_for_archive(relative: &Path) -> Result<String, CommandError> {
    let value = relative
        .components()
        .map(|component| match component {
            std::path::Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| CommandError::new("A profile file has an unsupported name."))?
        .join("/");
    if value.is_empty() {
        return Err(CommandError::new("A profile file has an unsupported name."));
    }
    Ok(value)
}

fn file_hashes(path: &Path) -> Result<(String, String), CommandError> {
    reader_hashes(&mut std::fs::File::open(path)?)
}

fn reader_hashes(reader: &mut impl Read) -> Result<(String, String), CommandError> {
    let mut sha1 = Sha1::new();
    let mut sha512 = Sha512::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        sha1.update(&buffer[..read]);
        sha512.update(&buffer[..read]);
    }
    Ok((
        format!("{:x}", sha1.finalize()),
        format!("{:x}", sha512.finalize()),
    ))
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
    if index.format_version != 1
        || index.game != "minecraft"
        || index.version_id.trim().is_empty()
        || index.name.trim().is_empty()
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
    let mut paths = HashSet::new();
    for file in &index.files {
        let relative = safe_relative_path(&file.path)?;
        reject_reserved_path(&relative)?;
        if !paths.insert(relative) {
            return Err(CommandError::new(format!(
                "The modpack contains the path {} more than once.",
                file.path
            )));
        }
        if !valid_hex_hash(&file.hashes.sha1, 40) || !valid_hex_hash(&file.hashes.sha512, 128) {
            return Err(CommandError::new(format!(
                "The modpack contains invalid checksums for {}.",
                file.path
            )));
        }
    }
    Ok(())
}

fn valid_hex_hash(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
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
        || relative == Path::new("resourcepacks/.ninjalauncher-managed.json")
        || relative == Path::new("shaderpacks/.ninjalauncher-managed.json")
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

    #[test]
    fn exports_unknown_mods_as_overrides() {
        let root = std::env::temp_dir().join(format!(
            "ninja-launcher-export-test-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let instance = root.join("instance");
        std::fs::create_dir_all(instance.join("mods")).expect("mods directory");
        std::fs::create_dir_all(instance.join("config")).expect("config directory");
        std::fs::write(instance.join("mods/private.jar"), b"private mod").expect("mod file");
        std::fs::write(instance.join("config/example.toml"), b"enabled = true")
            .expect("config file");
        let config_only = collect_export_files(&instance, &HashSet::from(["config".into()]))
            .expect("collect selected folder");
        assert!(config_only
            .iter()
            .any(|(path, _)| path == Path::new("config/example.toml")));
        assert!(!config_only
            .iter()
            .any(|(path, _)| path == Path::new("mods/private.jar")));
        let files =
            collect_export_files(&instance, &HashSet::from(["mods".into(), "config".into()]))
                .expect("collect files");
        let mod_path = PathBuf::from("mods/private.jar");
        let hashes = HashMap::from([(
            mod_path.clone(),
            file_hashes(&instance.join(&mod_path)).expect("hashes"),
        )]);
        let index = ModpackIndex {
            format_version: 1,
            game: "minecraft".into(),
            version_id: "test-export".into(),
            name: "Test Export".into(),
            files: Vec::new(),
            dependencies: HashMap::from([
                ("minecraft".into(), "1.21.1".into()),
                ("fabric-loader".into(), "0.16.10".into()),
            ]),
        };
        let output = root.join("test.mrpack");
        write_export_archive(&output, &files, &hashes, &HashMap::new(), &index)
            .expect("write archive");
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&output).expect("archive file"))
            .expect("archive");
        let index_entry = archive
            .by_name("modrinth.index.json")
            .expect("modrinth index");
        assert_eq!(index_entry.compression(), zip::CompressionMethod::Stored);
        drop(index_entry);
        assert!(archive.by_name("overrides/mods/private.jar").is_ok());
        assert!(archive.by_name("overrides/config/example.toml").is_ok());
        assert!(!output.with_extension("mrpack.part").exists());
        drop(archive);
        std::fs::remove_dir_all(root).expect("remove test directory");
    }
}
