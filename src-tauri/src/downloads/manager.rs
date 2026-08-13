use crate::error::CommandError;
use futures_util::{stream, StreamExt, TryStreamExt};
use sha1::{Digest, Sha1};
use sha2::{Sha256, Sha512};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[derive(Clone, Debug)]
pub struct DownloadSpec {
    pub url: String,
    pub path: PathBuf,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub sha512: Option<String>,
}

pub async fn download_all(
    client: &reqwest::Client,
    downloads: Vec<DownloadSpec>,
    concurrency: usize,
) -> Result<(), CommandError> {
    stream::iter(
        downloads
            .into_iter()
            .map(|download| async move { download_one(client, download).await }),
    )
    .buffer_unordered(concurrency.max(1))
    .try_collect::<Vec<_>>()
    .await?;
    Ok(())
}

pub async fn download_one(
    client: &reqwest::Client,
    spec: DownloadSpec,
) -> Result<(), CommandError> {
    if !spec.url.starts_with("https://") {
        return Err(CommandError::new(
            "A download was rejected because it did not use HTTPS.",
        ));
    }
    if spec.path.is_file()
        && verify(
            &spec.path,
            spec.sha1.as_deref(),
            spec.sha256.as_deref(),
            spec.sha512.as_deref(),
        )
        .await?
    {
        return Ok(());
    }
    if let Some(parent) = spec.path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let temporary = spec.path.with_extension(format!(
        "{}part",
        spec.path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
    ));
    let response = client.get(&spec.url).send().await?.error_for_status()?;
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(&temporary).await?;
    let mut sha1 = Sha1::new();
    let mut sha256 = Sha256::new();
    let mut sha512 = Sha512::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        sha1.update(&chunk);
        sha256.update(&chunk);
        sha512.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    file.sync_all().await?;
    let actual_sha1 = format!("{:x}", sha1.finalize());
    let actual_sha256 = format!("{:x}", sha256.finalize());
    let actual_sha512 = format!("{:x}", sha512.finalize());
    if spec
        .sha1
        .as_ref()
        .is_some_and(|expected| !expected.eq_ignore_ascii_case(&actual_sha1))
        || spec
            .sha256
            .as_ref()
            .is_some_and(|expected| !expected.eq_ignore_ascii_case(&actual_sha256))
        || spec
            .sha512
            .as_ref()
            .is_some_and(|expected| !expected.eq_ignore_ascii_case(&actual_sha512))
    {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(CommandError::new(format!(
            "Checksum verification failed for {}.",
            spec.path.file_name().unwrap_or_default().to_string_lossy()
        )));
    }
    if spec.path.exists() {
        tokio::fs::remove_file(&spec.path).await?;
    }
    tokio::fs::rename(temporary, spec.path).await?;
    Ok(())
}

async fn verify(
    path: &std::path::Path,
    expected_sha1: Option<&str>,
    expected_sha256: Option<&str>,
    expected_sha512: Option<&str>,
) -> Result<bool, CommandError> {
    if expected_sha1.is_none() && expected_sha256.is_none() && expected_sha512.is_none() {
        return Ok(true);
    }
    let bytes = tokio::fs::read(path).await?;
    if let Some(expected) = expected_sha1 {
        if !expected.eq_ignore_ascii_case(&format!("{:x}", Sha1::digest(&bytes))) {
            return Ok(false);
        }
    }
    if let Some(expected) = expected_sha256 {
        if !expected.eq_ignore_ascii_case(&format!("{:x}", Sha256::digest(&bytes))) {
            return Ok(false);
        }
    }
    if let Some(expected) = expected_sha512 {
        if !expected.eq_ignore_ascii_case(&format!("{:x}", Sha512::digest(&bytes))) {
            return Ok(false);
        }
    }
    Ok(true)
}
