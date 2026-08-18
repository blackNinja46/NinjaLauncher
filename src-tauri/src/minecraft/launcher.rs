use crate::{
    auth::microsoft,
    discord::DiscordPresence,
    downloads::manager::{self, DownloadSpec},
    error::CommandError,
    instances::{instance::InstanceConfig, manager as instance_manager},
    java,
    loaders::fabric,
    minecraft::metadata::{self, AssetIndex, Download, Library, VersionManifest, VersionMetadata},
    modrinth::{client::ModrinthClient, resolver},
    storage::{config, paths::PathManager},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::Stdio,
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};

const VERSION_MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchStatus {
    pub phase: String,
    pub message: String,
}

#[derive(Clone, Serialize)]
struct GameLogLine {
    stream: &'static str,
    line: String,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManagedMods {
    files: Vec<ManagedMod>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ManagedMod {
    project_id: String,
    version_id: String,
    filename: String,
    sha1: String,
}

pub async fn launch(
    app: AppHandle,
    paths: &PathManager,
    modrinth: &ModrinthClient,
    instance_id: &str,
    demo: bool,
) -> Result<(), CommandError> {
    emit(&app, "validate", "Validating account and instance");
    let instance = instance_manager::load(paths, instance_id)?;
    let mut account = if demo {
        microsoft::Account::demo()
    } else {
        microsoft::load(paths)?.ok_or_else(|| {
            CommandError::new("Sign in with your Microsoft account before playing.")
        })?
    };
    let settings = config::load(paths)?;
    if !demo && account.expires_at <= microsoft::now() + 60 {
        account = microsoft::refresh(&account.refresh_token).await?;
        microsoft::save(paths, &account)?;
    }
    let client = reqwest::Client::builder()
        .user_agent(concat!("NinjaLauncher/", env!("CARGO_PKG_VERSION")))
        .build()?;

    emit(&app, "mods", "Resolving compatible mods and dependencies");
    sync_mods(
        &app,
        paths,
        modrinth,
        &client,
        &instance,
        settings.download_concurrency as usize,
    )
    .await?;

    emit(&app, "minecraft", "Reading Minecraft version metadata");
    let base = minecraft_metadata(&client, paths, &instance.minecraft_version).await?;
    let fabric = fabric::profile(
        &client,
        &instance.minecraft_version,
        &instance.loader_version,
    )
    .await?;
    let required_java = base
        .java_version
        .as_ref()
        .map(|version| version.major_version)
        .unwrap_or(8);
    emit(
        &app,
        "java",
        &format!("Preparing Java {required_java} (automatic install if needed)"),
    );
    let java = java::manager::select_or_install(
        &client,
        paths,
        settings.java_path.as_deref(),
        required_java,
    )
    .await?;

    emit(
        &app,
        "downloads",
        "Downloading Minecraft libraries and assets",
    );
    let prepared = prepare_game(
        paths,
        &client,
        &instance,
        &base,
        &fabric,
        settings.download_concurrency as usize,
    )
    .await?;
    emit(&app, "launch", "Starting Minecraft");
    start_process(
        app,
        java,
        prepared,
        ProcessInputs {
            paths,
            instance: &instance,
            account: &account,
            base: &base,
            fabric: &fabric,
            memory_mb: settings.allocated_memory_mb,
            close_on_launch: settings.close_on_launch,
            reopen_on_exit: settings.reopen_on_exit,
            demo,
        },
    )
    .await
}

async fn minecraft_metadata(
    client: &reqwest::Client,
    paths: &PathManager,
    version: &str,
) -> Result<VersionMetadata, CommandError> {
    let manifest: VersionManifest = client
        .get(VERSION_MANIFEST)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let entry = manifest
        .versions
        .into_iter()
        .find(|entry| entry.id == version)
        .ok_or_else(|| CommandError::new(format!("Minecraft {version} does not exist.")))?;
    let metadata: VersionMetadata = client
        .get(entry.url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if metadata.id != version {
        return Err(CommandError::new(
            "Minecraft metadata returned an unexpected version identifier.",
        ));
    }
    let directory = paths.shared_dir().join("versions").join(version);
    tokio::fs::create_dir_all(&directory).await?;
    tokio::fs::write(
        directory.join(format!("{version}.json")),
        serde_json::to_vec_pretty(&metadata_to_value(&metadata))?,
    )
    .await?;
    Ok(metadata)
}

fn metadata_to_value(metadata: &VersionMetadata) -> serde_json::Value {
    serde_json::json!({"id":metadata.id,"mainClass":metadata.main_class,"assets":metadata.assets})
}

struct PreparedGame {
    classpath: Vec<PathBuf>,
    natives: PathBuf,
    assets: PathBuf,
    logging_argument: Option<String>,
}

async fn prepare_game(
    paths: &PathManager,
    client: &reqwest::Client,
    instance: &InstanceConfig,
    base: &VersionMetadata,
    fabric: &VersionMetadata,
    concurrency: usize,
) -> Result<PreparedGame, CommandError> {
    let shared = paths.shared_dir();
    let libraries_root = shared.join("libraries");
    let assets_root = shared.join("assets");
    let client_download = base
        .downloads
        .as_ref()
        .ok_or_else(|| CommandError::new("Minecraft metadata has no client download."))?
        .client
        .clone();
    let client_jar = shared
        .join("versions")
        .join(&base.id)
        .join(format!("{}.jar", base.id));
    let mut downloads = vec![spec(&client_download, client_jar.clone())];
    let mut classpath = Vec::new();
    let mut native_archives = Vec::new();
    let libraries = base.libraries.iter().chain(fabric.libraries.iter());
    for library in libraries {
        if !metadata::applies(&library.rules) {
            continue;
        }
        if let Some(artifact) = library_artifact(library)? {
            let fallback = maven_path(&library.name);
            let path = safe_join(
                &libraries_root,
                artifact.path.as_deref().unwrap_or(&fallback),
            )?;
            downloads.push(spec(&artifact, path.clone()));
            classpath.push(path);
        }
        if let Some(classifier) = library.natives.get(metadata::current_os()) {
            if let Some(details) = library.downloads.as_ref().and_then(|downloads| {
                downloads
                    .classifiers
                    .get(&classifier.replace("${arch}", "64"))
            }) {
                let fallback = maven_path(&library.name);
                let path = safe_join(
                    &libraries_root,
                    details.path.as_deref().unwrap_or(&fallback),
                )?;
                downloads.push(spec(details, path.clone()));
                native_archives.push((path, library.extract.clone()));
            }
        }
    }
    if let Some(asset_ref) = &base.asset_index {
        if asset_ref.id.is_empty()
            || !asset_ref.id.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
            })
        {
            return Err(CommandError::new(
                "Minecraft returned an invalid asset index identifier.",
            ));
        }
        let index_path = assets_root
            .join("indexes")
            .join(format!("{}.json", asset_ref.id));
        downloads.push(DownloadSpec {
            url: asset_ref.url.clone(),
            path: index_path.clone(),
            sha1: Some(asset_ref.sha1.clone()),
            sha256: None,
            sha512: None,
        });
        manager::download_all(client, downloads, concurrency).await?;
        let index: AssetIndex = serde_json::from_slice(&tokio::fs::read(index_path).await?)?;
        let mut asset_downloads = Vec::with_capacity(index.objects.len());
        for object in index.objects.into_values() {
            if object.hash.len() != 40
                || !object
                    .hash
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
            {
                return Err(CommandError::new(
                    "Minecraft returned an invalid asset hash.",
                ));
            }
            let prefix = &object.hash[..2];
            asset_downloads.push(DownloadSpec {
                url: format!(
                    "https://resources.download.minecraft.net/{prefix}/{}",
                    object.hash
                ),
                path: assets_root.join("objects").join(prefix).join(&object.hash),
                sha1: Some(object.hash),
                sha256: None,
                sha512: None,
            });
        }
        manager::download_all(client, asset_downloads, concurrency).await?;
    } else {
        manager::download_all(client, downloads, concurrency).await?;
    }
    classpath.push(client_jar);
    let natives = shared.join("natives").join(&instance.id);
    tokio::fs::create_dir_all(&natives).await?;
    for (archive, extract) in native_archives {
        extract_natives(
            &archive,
            &natives,
            extract
                .as_ref()
                .map(|item| item.exclude.as_slice())
                .unwrap_or(&[]),
        )?;
    }
    let logging_argument = if let Some(logging) = &base.logging {
        let logging_id = Path::new(&logging.client.file.id)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| *value == logging.client.file.id)
            .ok_or_else(|| CommandError::new("Minecraft returned an unsafe logging filename."))?;
        let path = shared.join("logging").join(logging_id);
        manager::download_one(
            client,
            DownloadSpec {
                url: logging.client.file.url.clone(),
                path: path.clone(),
                sha1: Some(logging.client.file.sha1.clone()),
                sha256: None,
                sha512: None,
            },
        )
        .await?;
        Some(
            logging
                .client
                .argument
                .replace("${path}", &path.to_string_lossy()),
        )
    } else {
        None
    };
    Ok(PreparedGame {
        classpath,
        natives,
        assets: assets_root,
        logging_argument,
    })
}

fn library_artifact(library: &Library) -> Result<Option<Download>, CommandError> {
    if let Some(downloads) = &library.downloads {
        return Ok(downloads.artifact.clone());
    }
    if library.name.is_empty() {
        return Ok(None);
    }
    let path = maven_path(&library.name);
    let base = library
        .url
        .as_deref()
        .unwrap_or("https://maven.fabricmc.net/");
    Ok(Some(Download {
        url: format!("{}{path}", base.trim_end_matches('/').to_string() + "/"),
        sha1: None,
        path: Some(path),
    }))
}

fn maven_path(coordinate: &str) -> String {
    let parts: Vec<_> = coordinate.split(':').collect();
    if parts.len() < 3 {
        return coordinate.replace(':', "/");
    }
    let classifier = parts
        .get(3)
        .map(|value| format!("-{value}"))
        .unwrap_or_default();
    format!(
        "{}/{}/{}/{}-{}{}.jar",
        parts[0].replace('.', "/"),
        parts[1],
        parts[2],
        parts[1],
        parts[2],
        classifier
    )
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, CommandError> {
    let path = Path::new(relative);
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
        return Err(CommandError::new(
            "Minecraft returned an unsafe library path.",
        ));
    }
    Ok(root.join(path))
}
fn spec(download: &Download, path: PathBuf) -> DownloadSpec {
    DownloadSpec {
        url: download.url.clone(),
        path,
        sha1: download.sha1.clone(),
        sha256: None,
        sha512: None,
    }
}

fn extract_natives(archive: &Path, target: &Path, excludes: &[String]) -> Result<(), CommandError> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|error| CommandError::new(format!("Invalid native library: {error}")))?;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| CommandError::new(error.to_string()))?;
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let relative_text = relative.to_string_lossy();
        if entry.is_dir()
            || relative_text.starts_with("META-INF/")
            || excludes
                .iter()
                .any(|exclude| relative_text.starts_with(exclude))
        {
            continue;
        }
        let destination = target.join(relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::io::copy(&mut entry, &mut std::fs::File::create(destination)?)?;
    }
    Ok(())
}

async fn sync_mods(
    app: &AppHandle,
    paths: &PathManager,
    modrinth: &ModrinthClient,
    client: &reqwest::Client,
    instance: &InstanceConfig,
    concurrency: usize,
) -> Result<(), CommandError> {
    let roots: Vec<_> = instance
        .mods
        .iter()
        .filter(|item| item.enabled)
        .map(|item| item.project_id.clone())
        .collect();
    let pinned_versions: HashMap<_, _> = instance
        .mods
        .iter()
        .filter(|item| item.enabled)
        .filter_map(|item| {
            item.version_overrides
                .get(&instance.minecraft_version)
                .map(|version_id| (item.project_id.clone(), version_id.clone()))
        })
        .collect();
    let versions = resolver::resolve(
        modrinth,
        &roots,
        &instance.minecraft_version,
        &instance.loader,
        &pinned_versions,
    )
    .await?;
    let directory = instance_manager::instance_dir(paths, &instance.id)?.join("mods");
    let manifest_path = directory.join(".ninjalauncher-managed.json");
    let previous: ManagedMods = if manifest_path.is_file() {
        serde_json::from_slice(&std::fs::read(&manifest_path)?)?
    } else {
        ManagedMods::default()
    };
    let mut desired = ManagedMods::default();
    let mut downloads = Vec::new();
    for version in versions {
        let file = version
            .files
            .iter()
            .find(|file| file.primary)
            .or_else(|| version.files.first())
            .ok_or_else(|| {
                CommandError::new(format!("Mod {} has no downloadable file.", version.name))
            })?;
        let safe_name = Path::new(&file.filename)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|name| name.ends_with(".jar") && *name == file.filename)
            .ok_or_else(|| CommandError::new("Modrinth returned an unsafe filename."))?;
        downloads.push(DownloadSpec {
            url: file.url.clone(),
            path: directory.join(safe_name),
            sha1: Some(file.hashes.sha1.clone()),
            sha256: None,
            sha512: Some(file.hashes.sha512.clone()),
        });
        desired.files.push(ManagedMod {
            project_id: version.project_id,
            version_id: version.id,
            filename: safe_name.into(),
            sha1: file.hashes.sha1.clone(),
        });
    }
    emit(
        app,
        "mods",
        &format!("Synchronizing {} managed mod files", desired.files.len()),
    );
    manager::download_all(client, downloads, concurrency).await?;
    let wanted: HashSet<_> = desired
        .files
        .iter()
        .map(|file| file.filename.as_str())
        .collect();
    for old in previous.files {
        if !wanted.contains(old.filename.as_str()) {
            let safe_name = Path::new(&old.filename)
                .file_name()
                .and_then(|value| value.to_str())
                .filter(|value| *value == old.filename);
            let Some(safe_name) = safe_name else {
                continue;
            };
            let file = directory.join(safe_name);
            if file.is_file() {
                std::fs::remove_file(file)?;
            }
        }
    }
    std::fs::write(manifest_path, serde_json::to_vec_pretty(&desired)?)?;
    Ok(())
}

struct ProcessInputs<'a> {
    paths: &'a PathManager,
    instance: &'a InstanceConfig,
    account: &'a microsoft::Account,
    base: &'a VersionMetadata,
    fabric: &'a VersionMetadata,
    memory_mb: u32,
    close_on_launch: bool,
    reopen_on_exit: bool,
    demo: bool,
}

async fn start_process(
    app: AppHandle,
    java: PathBuf,
    prepared: PreparedGame,
    input: ProcessInputs<'_>,
) -> Result<(), CommandError> {
    let ProcessInputs {
        paths,
        instance,
        account,
        base,
        fabric,
        memory_mb,
        close_on_launch,
        reopen_on_exit,
        demo,
    } = input;
    let separator = if cfg!(windows) { ";" } else { ":" };
    let classpath = prepared
        .classpath
        .iter()
        .map(|path| path.to_string_lossy())
        .collect::<Vec<_>>()
        .join(separator);
    let game_dir = instance_manager::instance_dir(paths, &instance.id)?;
    let library_directory = paths.shared_dir().join("libraries");
    let values = HashMap::from([
        ("${auth_player_name}", account.username.as_str()),
        ("${version_name}", instance.minecraft_version.as_str()),
        ("${game_directory}", game_dir.to_str().unwrap_or("")),
        ("${assets_root}", prepared.assets.to_str().unwrap_or("")),
        ("${assets_index_name}", base.assets.as_str()),
        ("${auth_uuid}", account.uuid.as_str()),
        ("${auth_access_token}", account.access_token.as_str()),
        ("${user_type}", if demo { "legacy" } else { "msa" }),
        ("${version_type}", "release"),
        (
            "${natives_directory}",
            prepared.natives.to_str().unwrap_or(""),
        ),
        ("${launcher_name}", "Ninja Launcher"),
        ("${launcher_version}", env!("CARGO_PKG_VERSION")),
        ("${classpath}", classpath.as_str()),
        ("${classpath_separator}", separator),
        (
            "${library_directory}",
            library_directory.to_str().unwrap_or(""),
        ),
        ("${user_properties}", "{}"),
        ("${clientid}", ""),
        ("${auth_xuid}", ""),
    ]);
    let replace = |value: String| {
        values
            .iter()
            .fold(value, |text, (from, to)| text.replace(from, to))
    };
    let mut jvm = metadata::expand_arguments(&base.arguments.jvm);
    jvm.extend(metadata::expand_arguments(&fabric.arguments.jvm));
    if jvm.is_empty() {
        jvm.extend([
            format!("-Djava.library.path={}", prepared.natives.to_string_lossy()),
            "-cp".into(),
            classpath.clone(),
        ]);
    }
    jvm.push(format!("-Xmx{memory_mb}M"));
    if let Some(argument) = prepared.logging_argument {
        jvm.push(argument);
    }
    let mut game = if let Some(legacy) = &base.minecraft_arguments {
        legacy.split_whitespace().map(String::from).collect()
    } else {
        metadata::expand_arguments(&base.arguments.game)
    };
    game.extend(metadata::expand_arguments(&fabric.arguments.game));
    if demo && !game.iter().any(|argument| argument == "--demo") {
        game.push("--demo".into());
    }
    let mut command = tokio::process::Command::new(java);
    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    command
        .args(jvm.into_iter().map(&replace))
        .arg(&fabric.main_class)
        .args(game.into_iter().map(replace))
        .current_dir(&game_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| CommandError::new(format!("Minecraft could not be started: {error}")))?;
    app.state::<DiscordPresence>()
        .show_playing(&instance.name, &instance.minecraft_version);
    app.emit("launcher://game-started", &instance.id).ok();
    if close_on_launch {
        if let Some(window) = app.get_webview_window("main") {
            window.hide().ok();
        }
    }
    if let Some(stdout) = child.stdout.take() {
        let app = app.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let Some(line) = clean_game_log_line(&line) else {
                    continue;
                };
                app.emit(
                    "launcher://log",
                    GameLogLine {
                        stream: "stdout",
                        line,
                    },
                )
                .ok();
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let app = app.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let Some(line) = clean_game_log_line(&line) else {
                    continue;
                };
                app.emit(
                    "launcher://log",
                    GameLogLine {
                        stream: "stderr",
                        line,
                    },
                )
                .ok();
            }
        });
    }
    tokio::spawn(async move {
        let result = child.wait().await;
        if reopen_on_exit {
            if let Some(window) = app.get_webview_window("main") {
                window.show().ok();
                window.set_focus().ok();
            }
        }
        app.state::<DiscordPresence>().show_launcher();
        app.emit(
            "launcher://game-exited",
            result.ok().and_then(|status| status.code()),
        )
        .ok();
    });
    Ok(())
}

fn clean_game_log_line(line: &str) -> Option<String> {
    const MESSAGE_START: &str = "<log4j:Message><![CDATA[";
    const MESSAGE_END: &str = "]]></log4j:Message>";
    const THROWABLE_START: &str = "<log4j:Throwable><![CDATA[";
    const THROWABLE_END: &str = "]]></log4j:Throwable>";

    let mut content = line.trim_end_matches('\r');
    let wrapper_start = [MESSAGE_START, THROWABLE_START]
        .into_iter()
        .filter_map(|marker| content.find(marker).map(|index| (index, marker)))
        .min_by_key(|(index, _)| *index);
    let contained_log4j_cdata = if let Some((start, marker)) = wrapper_start {
        content = &content[start + marker.len()..];
        true
    } else {
        false
    };

    let wrapper_end = [MESSAGE_END, THROWABLE_END]
        .into_iter()
        .filter_map(|marker| content.find(marker))
        .min();
    if let Some(end) = wrapper_end {
        content = &content[..end];
    } else if contained_log4j_cdata {
        if let Some(end) = content.find("]]>") {
            content = &content[..end];
        }
    } else if matches!(
        content.trim(),
        value if value.starts_with("<?xml")
            || value.starts_with("<log4j:")
            || value.starts_with("</log4j:")
    ) {
        return None;
    }

    let stream_content = content
        .strip_prefix("[STDOUT]:")
        .or_else(|| content.strip_prefix("[STDERR]:"));
    let without_stream = stream_content
        .map(|value| value.strip_prefix(' ').unwrap_or(value))
        .unwrap_or(content);

    (!without_stream.is_empty()).then(|| without_stream.to_string())
}

fn emit(app: &AppHandle, phase: &str, message: &str) {
    app.emit(
        "launcher://status",
        LaunchStatus {
            phase: phase.into(),
            message: message.into(),
        },
    )
    .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_paths_cannot_escape_the_cache() {
        let root = Path::new("/cache/libraries");
        assert!(safe_join(root, "org/example/library.jar").is_ok());
        assert!(safe_join(root, "../../account.json").is_err());
        assert!(safe_join(root, "/absolute/file.jar").is_err());
    }

    #[test]
    fn removes_log4j_cdata_and_stream_wrappers() {
        assert_eq!(
            clean_game_log_line(
                r#"<log4j:Message><![CDATA[[STDOUT]: [12:00:00] [main/INFO]: Ready]]></log4j:Message>"#
            ),
            Some("[12:00:00] [main/INFO]: Ready".into())
        );
        assert_eq!(
            clean_game_log_line(
                r#"<log4j:Event logger="Minecraft"><log4j:Message><![CDATA[[STDERR]: Crash]]></log4j:Message></log4j:Event>"#
            ),
            Some("Crash".into())
        );
        assert_eq!(
            clean_game_log_line(
                r#"<log4j:Throwable><![CDATA[java.lang.IllegalStateException: Broken]]></log4j:Throwable>"#
            ),
            Some("java.lang.IllegalStateException: Broken".into())
        );
        assert_eq!(
            clean_game_log_line(
                "    at net.minecraft.Main.main(Main.java:12)]]></log4j:Throwable>"
            ),
            Some("    at net.minecraft.Main.main(Main.java:12)".into())
        );
    }

    #[test]
    fn preserves_regular_log_lines_and_hides_xml_control_lines() {
        assert_eq!(
            clean_game_log_line("    at net.minecraft.Main.main(Main.java:12)"),
            Some("    at net.minecraft.Main.main(Main.java:12)".into())
        );
        assert_eq!(clean_game_log_line("<?xml version=\"1.0\"?>"), None);
        assert_eq!(clean_game_log_line("</log4j:Event>"), None);
    }
}
