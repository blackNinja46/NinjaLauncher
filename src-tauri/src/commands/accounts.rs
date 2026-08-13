use crate::{
    auth::microsoft::{self, DeviceCode, PublicAccount},
    error::CommandError,
    storage::paths::PathManager,
};
use tauri::State;

#[tauri::command]
pub fn get_account(paths: State<'_, PathManager>) -> Result<Option<PublicAccount>, CommandError> {
    Ok(microsoft::load(&paths)?.as_ref().map(PublicAccount::from))
}

#[tauri::command]
pub async fn begin_microsoft_login(client_id: String) -> Result<DeviceCode, CommandError> {
    microsoft::begin(&client_id).await
}

#[tauri::command]
pub async fn complete_microsoft_login(
    client_id: String,
    device_code: DeviceCode,
    paths: State<'_, PathManager>,
) -> Result<PublicAccount, CommandError> {
    let account = microsoft::complete(&client_id, &device_code).await?;
    microsoft::save(&paths, &account)?;
    Ok(PublicAccount::from(&account))
}

#[tauri::command]
pub fn logout(paths: State<'_, PathManager>) -> Result<(), CommandError> {
    microsoft::logout(&paths)
}
