use crate::{
    auth::microsoft::{self, AccountState, DeviceCode, PublicAccount},
    error::CommandError,
    storage::paths::PathManager,
};
use tauri::State;

#[tauri::command]
pub fn get_account(paths: State<'_, PathManager>) -> Result<Option<PublicAccount>, CommandError> {
    Ok(microsoft::load(&paths)?.as_ref().map(PublicAccount::from))
}

#[tauri::command]
pub fn get_accounts(paths: State<'_, PathManager>) -> Result<AccountState, CommandError> {
    microsoft::state(&paths)
}

#[tauri::command]
pub async fn begin_microsoft_login() -> Result<DeviceCode, CommandError> {
    microsoft::begin().await
}

#[tauri::command]
pub async fn complete_microsoft_login(
    device_code: DeviceCode,
    paths: State<'_, PathManager>,
) -> Result<PublicAccount, CommandError> {
    let account = microsoft::complete(&device_code).await?;
    microsoft::save(&paths, &account)?;
    Ok(PublicAccount::from(&account))
}

#[tauri::command]
pub fn switch_microsoft_account(
    uuid: String,
    paths: State<'_, PathManager>,
) -> Result<PublicAccount, CommandError> {
    microsoft::switch_account(&paths, &uuid)
}

#[tauri::command]
pub fn remove_microsoft_account(
    uuid: String,
    paths: State<'_, PathManager>,
) -> Result<AccountState, CommandError> {
    microsoft::remove_account(&paths, &uuid)
}

#[tauri::command]
pub fn logout(paths: State<'_, PathManager>) -> Result<(), CommandError> {
    microsoft::logout(&paths)
}
