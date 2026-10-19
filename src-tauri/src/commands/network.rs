use crate::{
    error::CommandError,
    network::{self, PlayerRanks},
};

#[tauri::command]
pub async fn get_network_ranks(uuid: String) -> Result<Option<PlayerRanks>, CommandError> {
    network::fetch_player_ranks(&uuid).await
}
