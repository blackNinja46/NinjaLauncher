use crate::{error::CommandError, minecraft::metadata::VersionMetadata};

pub async fn profile(
    client: &reqwest::Client,
    minecraft: &str,
    loader: &str,
) -> Result<VersionMetadata, CommandError> {
    let url =
        format!("https://meta.fabricmc.net/v2/versions/loader/{minecraft}/{loader}/profile/json");
    Ok(client
        .get(url)
        .send()
        .await?
        .error_for_status()
        .map_err(|_| {
            CommandError::new(format!(
                "Fabric {loader} is not available for Minecraft {minecraft}."
            ))
        })?
        .json()
        .await?)
}
