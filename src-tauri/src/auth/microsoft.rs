use crate::{error::CommandError, storage::paths::PathManager};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
}

impl Account {
    pub fn demo() -> Self {
        Self {
            username: "NinjaPlayer".into(),
            uuid: "00000000000000000000000000000000".into(),
            access_token: "0".into(),
            refresh_token: String::new(),
            expires_at: u64::MAX,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicAccount {
    pub username: String,
    pub uuid: String,
}

impl From<&Account> for PublicAccount {
    fn from(account: &Account) -> Self {
        Self {
            username: account.username.clone(),
            uuid: account.uuid.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
}

#[derive(Deserialize)]
struct MicrosoftToken {
    access_token: String,
    refresh_token: String,
}
#[derive(Deserialize)]
struct OAuthError {
    error: String,
    error_description: Option<String>,
}
#[derive(Deserialize)]
struct XboxToken {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: DisplayClaims,
}
#[derive(Deserialize)]
struct DisplayClaims {
    xui: Vec<UserHash>,
}
#[derive(Deserialize)]
struct UserHash {
    uhs: String,
}
#[derive(Deserialize)]
struct MinecraftToken {
    access_token: String,
    expires_in: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MinecraftServiceError {
    error_message: Option<String>,
}
#[derive(Deserialize)]
struct Entitlements {
    items: Vec<serde_json::Value>,
}
#[derive(Deserialize)]
struct MinecraftProfile {
    id: String,
    name: String,
}

pub async fn begin(client_id: &str) -> Result<DeviceCode, CommandError> {
    if client_id.trim().is_empty() {
        return Err(CommandError::new(
            "Add a Microsoft OAuth client ID in Settings first.",
        ));
    }
    Ok(reqwest::Client::new()
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .form(&[
            ("client_id", client_id),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

pub async fn complete(client_id: &str, code: &DeviceCode) -> Result<Account, CommandError> {
    let client = reqwest::Client::new();
    let deadline = now() + code.expires_in;
    let microsoft = loop {
        if now() >= deadline {
            return Err(CommandError::new("The Microsoft sign-in code expired."));
        }
        tokio::time::sleep(std::time::Duration::from_secs(code.interval.max(1))).await;
        let response = client
            .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", client_id),
                ("device_code", &code.device_code),
            ])
            .send()
            .await?;
        if response.status().is_success() {
            break response.json::<MicrosoftToken>().await?;
        }
        let error: OAuthError = response.json().await?;
        match error.error.as_str() {
            "authorization_pending" => continue,
            "slow_down" => {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
            "authorization_declined" => {
                return Err(CommandError::new("Microsoft sign-in was declined."))
            }
            _ => {
                return Err(CommandError::new(
                    error.error_description.unwrap_or(error.error),
                ))
            }
        }
    };
    exchange_for_minecraft(&client, microsoft).await
}

pub async fn refresh(client_id: &str, refresh_token: &str) -> Result<Account, CommandError> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&[
            ("client_id", client_id),
            ("scope", "XboxLive.signin offline_access"),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?
        .error_for_status()?;
    exchange_for_minecraft(&client, response.json::<MicrosoftToken>().await?).await
}

async fn exchange_for_minecraft(
    client: &reqwest::Client,
    microsoft: MicrosoftToken,
) -> Result<Account, CommandError> {
    let xbox: XboxToken = client
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .header("x-xbl-contract-version", "1")
        .json(&serde_json::json!({
            "Properties": {"AuthMethod":"RPS","SiteName":"user.auth.xboxlive.com","RpsTicket":format!("d={}", microsoft.access_token)},
            "RelyingParty":"http://auth.xboxlive.com","TokenType":"JWT"
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let uhs = xbox
        .display_claims
        .xui
        .first()
        .ok_or_else(|| CommandError::new("Xbox Live did not return a user identity."))?
        .uhs
        .clone();
    let xsts: XboxToken = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("x-xbl-contract-version", "1")
        .json(&serde_json::json!({
            "Properties":{"SandboxId":"RETAIL","UserTokens":[xbox.token]},
            "RelyingParty":"rp://api.minecraftservices.com/","TokenType":"JWT"
        }))
        .send()
        .await?
        .error_for_status()
        .map_err(|_| {
            CommandError::new("Xbox authorization failed. The account may need an Xbox profile.")
        })?
        .json()
        .await?;
    let response = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({"identityToken":format!("XBL3.0 x={uhs};{}", xsts.token)}))
        .send()
        .await?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(minecraft_login_error(status, &body));
    }
    let minecraft: MinecraftToken = response.json().await?;
    let entitlements: Entitlements = client
        .get("https://api.minecraftservices.com/entitlements/mcstore")
        .bearer_auth(&minecraft.access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if entitlements.items.is_empty() {
        return Err(CommandError::new(
            "This Microsoft account does not own Minecraft Java Edition.",
        ));
    }
    let profile: MinecraftProfile = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&minecraft.access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(Account {
        username: profile.name,
        uuid: profile.id,
        access_token: minecraft.access_token,
        refresh_token: microsoft.refresh_token,
        expires_at: now() + minecraft.expires_in,
    })
}

fn minecraft_login_error(status: reqwest::StatusCode, body: &str) -> CommandError {
    let service_message = serde_json::from_str::<MinecraftServiceError>(body)
        .ok()
        .and_then(|error| error.error_message);
    if status == reqwest::StatusCode::FORBIDDEN
        && service_message
            .as_deref()
            .is_some_and(|message| message.contains("Invalid app registration"))
    {
        return CommandError::new(
            "Microsoft sign-in succeeded, but Minecraft rejected this OAuth client ID. The app registration must be approved for Minecraft Services; an ordinary Azure/Entra client ID is not sufficient. You can continue with Play Demo while the launcher app is being approved.",
        );
    }
    CommandError::new(format!(
        "Minecraft sign-in failed ({status}): {}",
        service_message.unwrap_or_else(|| "The service returned no error details.".into())
    ))
}

pub fn load(paths: &PathManager) -> Result<Option<Account>, CommandError> {
    if !paths.account_file().is_file() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&std::fs::read(
        paths.account_file(),
    )?)?))
}

pub fn save(paths: &PathManager, account: &Account) -> Result<(), CommandError> {
    let target = paths.account_file();
    let temporary = target.with_extension("json.tmp");
    let mut file = std::fs::File::create(&temporary)?;
    file.write_all(&serde_json::to_vec(account)?)?;
    file.sync_all()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(temporary, target)?;
    Ok(())
}

pub fn logout(paths: &PathManager) -> Result<(), CommandError> {
    if paths.account_file().is_file() {
        std::fs::remove_file(paths.account_file())?;
    }
    Ok(())
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::{minecraft_login_error, Account};

    #[test]
    fn demo_account_does_not_contain_persisted_credentials() {
        let account = Account::demo();
        assert_eq!(account.username, "NinjaPlayer");
        assert_eq!(account.access_token, "0");
        assert!(account.refresh_token.is_empty());
        assert_eq!(account.expires_at, u64::MAX);
    }

    #[test]
    fn explains_unapproved_minecraft_app_registration() {
        let error = minecraft_login_error(
            reqwest::StatusCode::FORBIDDEN,
            r#"{"path":"/authentication/login_with_xbox","errorMessage":"Invalid app registration, see https://aka.ms/AppRegInfo for more information"}"#,
        );
        assert!(error
            .message
            .contains("must be approved for Minecraft Services"));
        assert!(!error
            .message
            .contains("online service could not be reached"));
    }
}
