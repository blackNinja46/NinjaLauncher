use crate::error::CommandError;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

const DEFAULT_RANK_API_URL: &str = "https://blackninja.live/api/launcher/ranks";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkRank {
    pub name: String,
    pub display_name: String,
    pub prefix: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub weight: i32,
    pub primary: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerRanks {
    pub uuid: String,
    pub username: Option<String>,
    pub primary_group: String,
    pub ranks: Vec<NetworkRank>,
}

pub async fn fetch_player_ranks(uuid: &str) -> Result<Option<PlayerRanks>, CommandError> {
    let player_id = Uuid::parse_str(uuid)
        .map_err(|_| CommandError::new("The Minecraft account UUID is invalid."))?;
    let base_url = option_env!("NINJA_RANK_API_URL")
        .unwrap_or(DEFAULT_RANK_API_URL)
        .trim_end_matches('/');
    let url = format!("{base_url}/{player_id}");
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent(concat!("NinjaLauncher/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| {
            CommandError::new("The BlackNinja.Live rank service could not be prepared.")
        })?;

    let response =
        client.get(url).send().await.map_err(|_| {
            CommandError::new("The BlackNinja.Live rank service could not be reached.")
        })?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(CommandError::new(format!(
            "The BlackNinja.Live rank service returned HTTP {}.",
            response.status().as_u16()
        )));
    }

    response
        .json::<PlayerRanks>()
        .await
        .map(Some)
        .map_err(|_| CommandError::new("The BlackNinja.Live rank response is invalid."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_response_uses_launcher_field_names() {
        let ranks: PlayerRanks = serde_json::from_str(
            r##"{"uuid":"069a79f4-44e9-4726-a5be-fca90e38aaf5","username":"Notch","primaryGroup":"vip","ranks":[{"name":"vip","displayName":"VIP","prefix":"[VIP]","icon":"vip","color":"#19d8f1","weight":10,"primary":true}]}"##,
        )
        .expect("valid rank response");
        assert_eq!(ranks.primary_group, "vip");
        assert_eq!(ranks.ranks[0].display_name, "VIP");
    }
}
