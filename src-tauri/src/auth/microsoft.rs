use crate::{error::CommandError, storage::paths::PathManager};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

const DEFAULT_MICROSOFT_CLIENT_ID: &str = "f06a20d6-185a-472d-bc99-897f2e7d3535";

pub fn client_id() -> &'static str {
    option_env!("NINJA_MICROSOFT_CLIENT_ID")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_MICROSOFT_CLIENT_ID)
}

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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicAccount {
    pub username: String,
    pub uuid: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountState {
    pub accounts: Vec<PublicAccount>,
    pub active_uuid: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountStore {
    active_uuid: Option<String>,
    accounts: Vec<Account>,
}

impl AccountStore {
    fn empty() -> Self {
        Self {
            active_uuid: None,
            accounts: Vec::new(),
        }
    }

    fn normalize(&mut self) {
        let active_exists = self
            .active_uuid
            .as_ref()
            .is_some_and(|uuid| self.accounts.iter().any(|account| &account.uuid == uuid));
        if !active_exists {
            self.active_uuid = self.accounts.first().map(|account| account.uuid.clone());
        }
    }

    fn active(&self) -> Option<&Account> {
        let active_uuid = self.active_uuid.as_deref()?;
        self.accounts
            .iter()
            .find(|account| account.uuid == active_uuid)
    }

    fn public_state(&self) -> AccountState {
        AccountState {
            accounts: self.accounts.iter().map(PublicAccount::from).collect(),
            active_uuid: self.active_uuid.clone(),
        }
    }
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
    #[serde(default)]
    capes: Vec<MinecraftCape>,
}

#[derive(Deserialize)]
struct MinecraftCape {
    url: String,
    state: Option<String>,
}

pub async fn begin() -> Result<DeviceCode, CommandError> {
    Ok(reqwest::Client::new()
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .form(&[
            ("client_id", client_id()),
            ("scope", "XboxLive.signin offline_access"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

pub async fn complete(code: &DeviceCode) -> Result<Account, CommandError> {
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
                ("client_id", client_id()),
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

pub async fn refresh(refresh_token: &str) -> Result<Account, CommandError> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&[
            ("client_id", client_id()),
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
    Ok(load_store(paths)?.active().cloned())
}

// Refresh credentials without changing whichever account is currently selected.
pub async fn authenticated_account(
    paths: &PathManager,
    uuid: &str,
) -> Result<Account, CommandError> {
    let mut account = load(paths)?
        .filter(|account| account.uuid.eq_ignore_ascii_case(uuid))
        .ok_or_else(|| {
            CommandError::new("Das Minecraft-Konto wurde gewechselt. Bitte erneut versuchen.")
        })?;
    if account.expires_at <= now() + 60 {
        if account.refresh_token.is_empty() {
            return Err(CommandError::new("Bitte erneut bei Microsoft anmelden."));
        }
        account = refresh(&account.refresh_token).await?;
        if !account.uuid.eq_ignore_ascii_case(uuid) {
            return Err(CommandError::new(
                "Die erneute Anmeldung lieferte ein anderes Konto.",
            ));
        }
        let mut store = load_store(paths)?;
        if let Some(existing) = store
            .accounts
            .iter_mut()
            .find(|item| item.uuid == account.uuid)
        {
            *existing = account.clone();
            write_store(paths, &store)?;
        } else {
            return Err(CommandError::new("Das Minecraft-Konto wurde entfernt."));
        }
    }
    Ok(account)
}

pub async fn cape_url(paths: &PathManager, uuid: &str) -> Result<Option<String>, CommandError> {
    let account = authenticated_account(paths, uuid).await?;
    let profile: MinecraftProfile = reqwest::Client::new()
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&account.access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(profile
        .capes
        .into_iter()
        .find(|cape| cape.state.as_deref().is_none_or(|state| state == "ACTIVE"))
        .and_then(|cape| {
            reqwest::Url::parse(&cape.url)
                .ok()
                .filter(|url| {
                    url.scheme() == "https" && url.host_str() == Some("textures.minecraft.net")
                })
                .map(|_| cape.url)
        }))
}

pub fn save(paths: &PathManager, account: &Account) -> Result<(), CommandError> {
    let mut store = load_store(paths)?;
    if let Some(existing) = store
        .accounts
        .iter_mut()
        .find(|existing| existing.uuid == account.uuid)
    {
        *existing = account.clone();
    } else {
        store.accounts.push(account.clone());
    }
    store.active_uuid = Some(account.uuid.clone());
    write_store(paths, &store)
}

pub fn state(paths: &PathManager) -> Result<AccountState, CommandError> {
    Ok(load_store(paths)?.public_state())
}

pub fn switch_account(paths: &PathManager, uuid: &str) -> Result<PublicAccount, CommandError> {
    let mut store = load_store(paths)?;
    let account = store
        .accounts
        .iter()
        .find(|account| account.uuid == uuid)
        .ok_or_else(|| CommandError::new("The selected Minecraft account no longer exists."))?;
    let public_account = PublicAccount::from(account);
    store.active_uuid = Some(uuid.to_owned());
    write_store(paths, &store)?;
    Ok(public_account)
}

pub fn remove_account(paths: &PathManager, uuid: &str) -> Result<AccountState, CommandError> {
    let mut store = load_store(paths)?;
    store.accounts.retain(|account| account.uuid != uuid);
    store.normalize();
    write_store(paths, &store)?;
    Ok(store.public_state())
}

fn load_store(paths: &PathManager) -> Result<AccountStore, CommandError> {
    if !paths.account_file().is_file() {
        return Ok(AccountStore::empty());
    }
    let contents = std::fs::read(paths.account_file())?;
    let mut store = match serde_json::from_slice::<AccountStore>(&contents) {
        Ok(store) => store,
        Err(_) => {
            let account = serde_json::from_slice::<Account>(&contents)?;
            AccountStore {
                active_uuid: Some(account.uuid.clone()),
                accounts: vec![account],
            }
        }
    };
    store.normalize();
    Ok(store)
}

fn write_store(paths: &PathManager, store: &AccountStore) -> Result<(), CommandError> {
    let target = paths.account_file();
    let temporary = target.with_extension("json.tmp");
    let mut file = std::fs::File::create(&temporary)?;
    file.write_all(&serde_json::to_vec(store)?)?;
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
    let mut store = load_store(paths)?;
    if let Some(active_uuid) = store.active_uuid.clone() {
        store.accounts.retain(|account| account.uuid != active_uuid);
        store.normalize();
        write_store(paths, &store)?;
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
    use super::{
        client_id, load, minecraft_login_error, remove_account, save, state, switch_account,
        Account, MinecraftProfile,
    };
    use crate::storage::paths::PathManager;

    fn test_paths(name: &str) -> PathManager {
        let root = std::env::temp_dir().join(format!(
            "ninjalauncher-account-{name}-{}",
            uuid::Uuid::new_v4()
        ));
        PathManager::new(root).expect("test path manager")
    }

    fn account(username: &str, uuid: &str) -> Account {
        Account {
            username: username.into(),
            uuid: uuid.into(),
            access_token: format!("{username}-token"),
            refresh_token: format!("{username}-refresh"),
            expires_at: 123,
        }
    }

    #[test]
    fn bundled_client_id_is_a_valid_uuid() {
        uuid::Uuid::parse_str(client_id()).expect("bundled Microsoft client ID should be a UUID");
    }

    #[test]
    fn reads_active_cape_from_minecraft_profile() {
        let profile: MinecraftProfile = serde_json::from_str(
            r#"{"id":"player-id","name":"Ninja","capes":[{"id":"cape-id","state":"ACTIVE","url":"https://textures.minecraft.net/texture/cape"}]}"#,
        )
        .expect("Minecraft profile with cape");
        assert_eq!(profile.capes.len(), 1);
        assert_eq!(profile.capes[0].state.as_deref(), Some("ACTIVE"));
    }

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

    #[test]
    fn migrates_a_legacy_single_account_file() {
        let paths = test_paths("migration");
        let legacy = account("Alex", "alex-uuid");
        std::fs::write(
            paths.account_file(),
            serde_json::to_vec(&legacy).expect("serialize legacy account"),
        )
        .expect("write legacy account");

        assert_eq!(
            load(&paths).expect("load").expect("active").username,
            "Alex"
        );
        let state = state(&paths).expect("state");
        assert_eq!(state.active_uuid.as_deref(), Some("alex-uuid"));
        assert_eq!(state.accounts.len(), 1);
    }

    #[test]
    fn saves_switches_and_removes_multiple_accounts() {
        let paths = test_paths("multiple");
        save(&paths, &account("Alex", "alex-uuid")).expect("save Alex");
        save(&paths, &account("Steve", "steve-uuid")).expect("save Steve");
        assert_eq!(
            load(&paths).expect("load").expect("active").username,
            "Steve"
        );

        let switched = switch_account(&paths, "alex-uuid").expect("switch account");
        assert_eq!(switched.username, "Alex");
        let remaining = remove_account(&paths, "alex-uuid").expect("remove account");
        assert_eq!(remaining.active_uuid.as_deref(), Some("steve-uuid"));
        assert_eq!(remaining.accounts.len(), 1);
    }
}
