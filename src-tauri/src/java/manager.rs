use crate::{
    downloads::manager::{self, DownloadSpec},
    error::CommandError,
    storage::paths::PathManager,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
struct Platform {
    os: &'static str,
    arch: &'static str,
    archive_extension: &'static str,
}

#[derive(Deserialize)]
struct AdoptiumRelease {
    binary: AdoptiumBinary,
}

#[derive(Deserialize)]
struct AdoptiumBinary {
    package: AdoptiumPackage,
}

#[derive(Deserialize)]
struct AdoptiumPackage {
    checksum: String,
    link: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaRuntimeInfo {
    pub path: String,
    pub version: String,
    pub major: u32,
    pub source: String,
}

pub async fn inspect_runtime(
    paths: &PathManager,
    configured: Option<&str>,
) -> Result<JavaRuntimeInfo, CommandError> {
    let explicitly_configured = configured.filter(|value| !value.trim().is_empty());
    let mut candidates = Vec::new();
    if let Some(value) = explicitly_configured {
        candidates.push((java_from_setting(value), "Custom runtime"));
    } else {
        if let Ok(home) = std::env::var("JAVA_HOME") {
            candidates.push((
                PathBuf::from(home).join("bin").join(executable()),
                "JAVA_HOME",
            ));
        }
        candidates.push((PathBuf::from(executable()), "System PATH"));
        if let Some(managed) = find_java(&paths.shared_dir().join("java"))? {
            candidates.push((managed, "Ninja Launcher managed runtime"));
        }
    }
    for (candidate, source) in candidates {
        if let Some((version, major, _)) = runtime_details(&candidate).await {
            return Ok(JavaRuntimeInfo {
                path: candidate.to_string_lossy().into_owned(),
                version,
                major,
                source: source.into(),
            });
        }
    }
    Err(CommandError::new(if explicitly_configured.is_some() {
        "The selected Java executable could not be started or did not report a valid version."
    } else {
        "No working Java runtime was found. Ninja Launcher will install the required runtime automatically when the game starts."
    }))
}

pub async fn select_or_install(
    client: &reqwest::Client,
    paths: &PathManager,
    configured: Option<&str>,
    required_major: u32,
    required_arch: Option<&str>,
) -> Result<PathBuf, CommandError> {
    let platform = platform(required_arch)?;
    let managed_dir = managed_dir(paths, required_major, platform);
    let mut candidates = Vec::new();
    if let Some(path) = configured.filter(|path| !path.trim().is_empty()) {
        candidates.push(java_from_setting(path));
    }
    if let Ok(home) = std::env::var("JAVA_HOME") {
        candidates.push(PathBuf::from(home).join("bin").join(executable()));
    }
    candidates.push(PathBuf::from(executable()));
    if let Some(managed) = find_java(&managed_dir)? {
        candidates.push(managed);
    }
    for candidate in candidates {
        if supports(&candidate, required_major, platform.arch).await {
            return Ok(candidate);
        }
    }

    install(client, paths, required_major, platform).await
}

async fn install(
    client: &reqwest::Client,
    paths: &PathManager,
    required_major: u32,
    platform: Platform,
) -> Result<PathBuf, CommandError> {
    let package = match latest_package(client, required_major, platform, "jre").await? {
        Some(package) => package,
        None => latest_package(client, required_major, platform, "jdk")
            .await?
            .ok_or_else(|| {
                CommandError::new(format!(
                    "No automatic Java {required_major} runtime is available for {} {}.",
                    platform.os, platform.arch
                ))
            })?,
    };
    if package.checksum.len() != 64
        || !package
            .checksum
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(CommandError::new(
            "The Java provider returned an invalid package checksum.",
        ));
    }

    let runtime_key = runtime_key(required_major, platform);
    let archive = paths
        .cache_dir()
        .join("java")
        .join(format!("{runtime_key}.{}", platform.archive_extension));
    manager::download_one(
        client,
        DownloadSpec {
            url: package.link,
            path: archive.clone(),
            sha1: None,
            sha256: Some(package.checksum),
            sha512: None,
        },
    )
    .await?;

    let java_root = paths.shared_dir().join("java");
    tokio::fs::create_dir_all(&java_root).await?;
    let staging = java_root.join(format!(".{runtime_key}.installing"));
    if staging.exists() {
        tokio::fs::remove_dir_all(&staging).await?;
    }
    tokio::fs::create_dir_all(&staging).await?;
    let archive_for_extract = archive.clone();
    let staging_for_extract = staging.clone();
    tokio::task::spawn_blocking(move || {
        extract_archive(
            &archive_for_extract,
            &staging_for_extract,
            platform.archive_extension,
        )
    })
    .await
    .map_err(|error| CommandError::new(format!("Java extraction failed: {error}")))??;

    let extracted_java = find_java(&staging)?.ok_or_else(|| {
        CommandError::new("The downloaded Java archive did not contain a Java executable.")
    })?;
    if !supports(&extracted_java, required_major, platform.arch).await {
        let _ = tokio::fs::remove_dir_all(&staging).await;
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) && platform.arch == "x64" {
            return Err(CommandError::new(format!(
                "Minecraft requires the Intel Java {required_major} compatibility runtime. Install Rosetta 2 and try again."
            )));
        }
        return Err(CommandError::new(format!(
            "The downloaded runtime is not compatible with Java {required_major}."
        )));
    }
    let relative_java = extracted_java
        .strip_prefix(&staging)
        .map_err(|_| CommandError::new("The Java archive used an unsafe directory layout."))?
        .to_path_buf();
    let destination = managed_dir(paths, required_major, platform);
    if destination.exists() {
        tokio::fs::remove_dir_all(&destination).await?;
    }
    tokio::fs::rename(&staging, &destination).await?;
    Ok(destination.join(relative_java))
}

async fn latest_package(
    client: &reqwest::Client,
    major: u32,
    platform: Platform,
    image_type: &str,
) -> Result<Option<AdoptiumPackage>, CommandError> {
    let response = client
        .get(format!(
            "https://api.adoptium.net/v3/assets/latest/{major}/hotspot"
        ))
        .query(&[
            ("architecture", platform.arch),
            ("image_type", image_type),
            ("os", platform.os),
            ("vendor", "eclipse"),
            ("heap_size", "normal"),
        ])
        .send()
        .await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let releases = response
        .error_for_status()?
        .json::<Vec<AdoptiumRelease>>()
        .await?;
    Ok(releases
        .into_iter()
        .next()
        .map(|release| release.binary.package))
}

async fn supports(candidate: &Path, required_major: u32, required_arch: &str) -> bool {
    runtime_details(candidate)
        .await
        .is_some_and(|(_, major, arch)| major == required_major && arch == required_arch)
}

async fn runtime_details(candidate: &Path) -> Option<(String, u32, &'static str)> {
    let mut command = tokio::process::Command::new(candidate);
    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    let Ok(output) = command
        .args(["-XshowSettings:properties", "-version"])
        .output()
        .await
    else {
        return None;
    };
    if !output.status.success() {
        return None;
    }
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let version = parse_version(&text)?;
    let major = parse_major(&text)?;
    let arch = parse_arch(&text)?;
    Some((version, major, arch))
}

fn extract_archive(archive: &Path, target: &Path, extension: &str) -> Result<(), CommandError> {
    let file = std::fs::File::open(archive)?;
    if extension == "zip" {
        let mut zip = zip::ZipArchive::new(file)
            .map_err(|error| CommandError::new(format!("Invalid Java archive: {error}")))?;
        for index in 0..zip.len() {
            let mut entry = zip
                .by_index(index)
                .map_err(|error| CommandError::new(format!("Invalid Java archive: {error}")))?;
            let relative = entry
                .enclosed_name()
                .ok_or_else(|| CommandError::new("The Java archive contains an unsafe path."))?;
            let destination = target.join(relative);
            if entry.is_dir() {
                std::fs::create_dir_all(destination)?;
            } else {
                if let Some(parent) = destination.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::io::copy(&mut entry, &mut std::fs::File::create(destination)?)?;
            }
        }
    } else {
        let decoder = flate2::read::GzDecoder::new(file);
        let mut tar = tar::Archive::new(decoder);
        tar.unpack(target)
            .map_err(|error| CommandError::new(format!("Invalid Java archive: {error}")))?;
    }
    Ok(())
}

fn find_java(root: &Path) -> Result<Option<PathBuf>, CommandError> {
    if !root.is_dir() {
        return Ok(None);
    }
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                directories.push(entry.path());
            } else if entry.file_name() == executable()
                && entry.path().parent().and_then(Path::file_name) == Some("bin".as_ref())
            {
                return Ok(Some(entry.path()));
            }
        }
    }
    Ok(None)
}

fn java_from_setting(value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_dir() {
        path.join("bin").join(executable())
    } else {
        path
    }
}

fn managed_dir(paths: &PathManager, major: u32, platform: Platform) -> PathBuf {
    paths
        .shared_dir()
        .join("java")
        .join(runtime_key(major, platform))
}

fn runtime_key(major: u32, platform: Platform) -> String {
    format!("temurin-{major}-{}-{}", platform.os, platform.arch)
}

fn platform(required_arch: Option<&str>) -> Result<Platform, CommandError> {
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        return Err(CommandError::new(
            "Automatic Java installation is not supported on this operating system.",
        ));
    };
    let native_arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        return Err(CommandError::new(
            "Automatic Java installation is not supported on this processor architecture.",
        ));
    };
    let arch = match required_arch {
        None => native_arch,
        Some("aarch64") if cfg!(target_arch = "aarch64") => "aarch64",
        Some("x64") if cfg!(target_os = "macos") || cfg!(target_arch = "x86_64") => "x64",
        Some(value) if value == native_arch => native_arch,
        Some(_) => {
            return Err(CommandError::new(
                "The required Java processor architecture is not supported on this system.",
            ));
        }
    };
    Ok(Platform {
        os,
        arch,
        archive_extension: if os == "windows" { "zip" } else { "tar.gz" },
    })
}

fn executable() -> &'static str {
    if cfg!(windows) {
        "java.exe"
    } else {
        "java"
    }
}

fn parse_major(version: &str) -> Option<u32> {
    let parsed = parse_version(version)?;
    let first: u32 = parsed.split('.').next()?.parse().ok()?;
    if first == 1 {
        parsed.split('.').nth(1)?.parse().ok()
    } else {
        Some(first)
    }
}

fn parse_version(output: &str) -> Option<String> {
    if let Some(quoted) = output.split('"').nth(1).filter(|value| !value.is_empty()) {
        return Some(quoted.into());
    }
    output
        .lines()
        .flat_map(str::split_whitespace)
        .map(|token| {
            token.trim_matches(|character: char| {
                !character.is_ascii_alphanumeric() && character != '.'
            })
        })
        .find(|token| {
            token
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
                && token.contains('.')
        })
        .map(String::from)
}

fn parse_arch(output: &str) -> Option<&'static str> {
    let value = output.lines().find_map(|line| {
        let (key, value) = line.trim().split_once('=')?;
        (key.trim() == "os.arch").then_some(value.trim())
    })?;
    match value {
        "aarch64" | "arm64" => Some("aarch64"),
        "x86_64" | "amd64" => Some("x64"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_java_versions() {
        assert_eq!(parse_major("java version \"1.8.0_402\""), Some(8));
        assert_eq!(parse_major("openjdk version \"21.0.2\""), Some(21));
        assert_eq!(parse_major("openjdk 25.0.3 2026-04-21"), Some(25));
    }

    #[test]
    fn parses_java_architectures() {
        assert_eq!(parse_arch("    os.arch = aarch64"), Some("aarch64"));
        assert_eq!(parse_arch("    os.arch = x86_64"), Some("x64"));
    }

    #[test]
    fn managed_runtime_key_contains_only_validated_components() {
        let platform = Platform {
            os: "mac",
            arch: "aarch64",
            archive_extension: "tar.gz",
        };
        assert_eq!(runtime_key(25, platform), "temurin-25-mac-aarch64");
    }
}
