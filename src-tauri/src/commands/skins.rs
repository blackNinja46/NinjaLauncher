use crate::{auth::microsoft, error::CommandError, storage::paths::PathManager};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{io::Cursor, time::Duration};
use tauri::State;

const MAX_PNG: usize = 1024 * 1024;
const PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSkin {
    pub id: String,
    pub name: String,
    pub texture: String,
    pub model: String,
    pub cape_id: Option<String>,
    pub cape_account_uuid: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Cape {
    pub id: String,
    pub url: String,
    pub alias: String,
    pub state: String,
}

#[derive(Deserialize, Serialize)]
pub struct ProfileSkin {
    pub url: String,
    pub variant: String,
    pub state: String,
}

#[derive(Deserialize, Serialize)]
pub struct SkinProfile {
    pub id: String,
    #[serde(default)]
    pub skins: Vec<ProfileSkin>,
    #[serde(default)]
    pub capes: Vec<Cape>,
}

#[derive(Serialize)]
pub struct ImportedSkin {
    pub texture: String,
    pub model: String,
    pub name: String,
}

fn client() -> Result<reqwest::Client, CommandError> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(25))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()?)
}

fn validate_png(bytes: &[u8]) -> Result<(), CommandError> {
    if bytes.len() > MAX_PNG {
        return Err(CommandError::new(
            "Die PNG-Datei darf höchstens 1 MB groß sein.",
        ));
    }
    let invalid = || {
        CommandError::new("Bitte eine gültige Skin-PNG mit 64 × 64 oder 64 × 32 Pixeln auswählen.")
    };
    let mut reader = png::Decoder::new(Cursor::new(bytes))
        .read_info()
        .map_err(|_| invalid())?;
    let info = reader.info();
    if info.width != 64 || ![32, 64].contains(&info.height) {
        return Err(invalid());
    }
    let mut buffer = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut buffer).map_err(|_| invalid())?;
    Ok(())
}

// Compare saved and account skins by pixels, independent of PNG metadata/compression.
fn canonical_texture(bytes: &[u8]) -> Result<String, CommandError> {
    validate_png(bytes)?;
    let invalid = || CommandError::new("Die Skin-PNG konnte nicht gelesen werden.");
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|_| invalid())?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).map_err(|_| invalid())?;
    let data = &buffer[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|v| [*v, *v, *v, 255]).collect(),
        png::ColorType::GrayscaleAlpha => data
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        _ => return Err(invalid()),
    };
    let mut output = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut output, info.width, info.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_| invalid())?;
        writer.write_image_data(&rgba).map_err(|_| invalid())?;
    }
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(output)))
}

fn decode_texture(texture: &str) -> Result<Vec<u8>, CommandError> {
    let data = texture
        .strip_prefix("data:image/png;base64,")
        .filter(|data| data.len() <= MAX_PNG * 4 / 3 + 4)
        .ok_or_else(|| CommandError::new("Ungültige Skin-Datei."))?;
    let bytes = STANDARD
        .decode(data)
        .map_err(|_| CommandError::new("Ungültige PNG-Daten."))?;
    validate_png(&bytes)?;
    Ok(bytes)
}

async fn download_png(url: &str) -> Result<String, CommandError> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|_| CommandError::new("Bitte eine gültige HTTPS-URL eingeben."))?;
    if parsed.scheme() != "https" || !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(CommandError::new(
            "Bitte eine HTTPS-URL ohne Zugangsdaten verwenden.",
        ));
    }
    let mut response = client()?.get(parsed).send().await?.error_for_status()?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > MAX_PNG {
            return Err(CommandError::new(
                "Die PNG-Datei darf höchstens 1 MB groß sein.",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    canonical_texture(&bytes)
}

fn skin_path(paths: &PathManager, id: &str) -> Result<std::path::PathBuf, CommandError> {
    let id = uuid::Uuid::parse_str(id).map_err(|_| CommandError::new("Ungültige Skin-ID."))?;
    Ok(paths.root().join("skins").join(format!("{id}.json")))
}

#[tauri::command]
pub fn list_skins(paths: State<'_, PathManager>) -> Result<Vec<SavedSkin>, CommandError> {
    read_library(&paths)
}

fn read_library(paths: &PathManager) -> Result<Vec<SavedSkin>, CommandError> {
    let directory = paths.root().join("skins");
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut skins = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            skins.push(serde_json::from_slice::<SavedSkin>(&std::fs::read(path)?)?);
        }
    }
    skins.sort_by_key(|skin| skin.name.to_lowercase());
    Ok(skins)
}

#[tauri::command]
pub fn save_skin(
    skin: SavedSkin,
    paths: State<'_, PathManager>,
) -> Result<SavedSkin, CommandError> {
    write_skin(skin, &paths)
}

fn write_skin(mut skin: SavedSkin, paths: &PathManager) -> Result<SavedSkin, CommandError> {
    skin.name = skin.name.trim().to_owned();
    if skin.name.is_empty() || skin.name.chars().count() > 48 {
        return Err(CommandError::new(
            "Der Skin-Name muss zwischen 1 und 48 Zeichen lang sein.",
        ));
    }
    if !["classic", "slim"].contains(&skin.model.as_str()) {
        return Err(CommandError::new("Ungültiges Skin-Modell."));
    }
    skin.texture = canonical_texture(&decode_texture(&skin.texture)?)?;
    let path = skin_path(&paths, &skin.id)?;
    std::fs::create_dir_all(path.parent().unwrap())?;
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    std::fs::write(&temporary, serde_json::to_vec(&skin)?)?;
    std::fs::rename(temporary, path)?;
    Ok(skin)
}

#[tauri::command]
pub fn delete_skin(id: String, paths: State<'_, PathManager>) -> Result<(), CommandError> {
    std::fs::remove_file(skin_path(&paths, &id)?)?;
    Ok(())
}

#[tauri::command]
pub async fn import_skin(source: String, value: String) -> Result<ImportedSkin, CommandError> {
    let value = value.trim();
    if source == "url" {
        return Ok(ImportedSkin {
            texture: download_png(value).await?,
            model: "classic".into(),
            name: "Mein Skin".into(),
        });
    }
    let client = client()?;
    let (id, name) = match source.as_str() {
        "uuid" => {
            let uuid = uuid::Uuid::parse_str(value)
                .map_err(|_| CommandError::new("Bitte eine gültige Spieler-UUID eingeben."))?;
            (uuid.simple().to_string(), "Mein Skin".to_string())
        }
        "name" => {
            if !(3..=16).contains(&value.len())
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            {
                return Err(CommandError::new(
                    "Ein Spielername besteht aus 3–16 Buchstaben, Zahlen oder Unterstrichen.",
                ));
            }
            let response = client
                .get(format!(
                    "https://api.mojang.com/users/profiles/minecraft/{value}"
                ))
                .send()
                .await?;
            if response.status() == reqwest::StatusCode::NOT_FOUND
                || response.status() == reqwest::StatusCode::NO_CONTENT
            {
                return Err(CommandError::new(
                    "Dieser Minecraft-Spieler wurde nicht gefunden.",
                ));
            }
            let profile: serde_json::Value = response.error_for_status()?.json().await?;
            let id = profile["id"]
                .as_str()
                .ok_or_else(|| CommandError::new("Spieler nicht gefunden."))?;
            (id.to_string(), value.to_string())
        }
        _ => return Err(CommandError::new("Unbekannte Importquelle.")),
    };
    let profile: serde_json::Value = client
        .get(format!(
            "https://sessionserver.mojang.com/session/minecraft/profile/{id}"
        ))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let encoded = profile["properties"]
        .as_array()
        .and_then(|properties| properties.iter().find(|p| p["name"] == "textures"))
        .and_then(|p| p["value"].as_str())
        .ok_or_else(|| CommandError::new("Dieser Spieler hat keinen verfügbaren Skin."))?;
    let decoded = STANDARD
        .decode(encoded)
        .map_err(|_| CommandError::new("Ungültige Spielertextur."))?;
    let textures: serde_json::Value = serde_json::from_slice(&decoded)?;
    let skin = &textures["textures"]["SKIN"];
    let url = skin["url"]
        .as_str()
        .ok_or_else(|| CommandError::new("Dieser Spieler hat keinen verfügbaren Skin."))?;
    // Mojang's signed texture records can still contain their legacy HTTP URL.
    let mut url =
        reqwest::Url::parse(url).map_err(|_| CommandError::new("Ungültige Textur-URL."))?;
    if url.host_str() != Some("textures.minecraft.net") {
        return Err(CommandError::new("Unbekannter Minecraft-Texturserver."));
    }
    let _ = url.set_scheme("https");
    Ok(ImportedSkin {
        texture: download_png(url.as_str()).await?,
        model: if skin["metadata"]["model"] == "slim" {
            "slim"
        } else {
            "classic"
        }
        .into(),
        name: profile["name"].as_str().unwrap_or(&name).to_owned(),
    })
}

async fn profile_for(token: &str) -> Result<SkinProfile, CommandError> {
    let response = client()?.get(PROFILE_URL).bearer_auth(token).send().await?;
    service_response(response)
        .await?
        .json()
        .await
        .map_err(Into::into)
}

async fn service_response(response: reqwest::Response) -> Result<reqwest::Response, CommandError> {
    if response.status().is_success() {
        return Ok(response);
    }
    let message = match response.status().as_u16() {
        401 => "Die Minecraft-Anmeldung ist abgelaufen. Bitte erneut anmelden.",
        403 => "Minecraft hat die Änderung abgelehnt. Bitte Kontoberechtigungen prüfen.",
        429 => "Zu viele Anfragen an Minecraft. Bitte etwas warten und erneut versuchen.",
        _ => "Minecraft konnte die Anfrage nicht ausführen. Bitte erneut versuchen.",
    };
    Err(CommandError::new(message))
}

#[tauri::command]
pub async fn get_skin_profile(
    uuid: String,
    paths: State<'_, PathManager>,
) -> Result<SkinProfile, CommandError> {
    let account = microsoft::authenticated_account(&paths, &uuid).await?;
    profile_for(&account.access_token).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub warning: Option<String>,
}

#[tauri::command]
pub async fn apply_skin(
    id: String,
    uuid: String,
    paths: State<'_, PathManager>,
) -> Result<ApplyResult, CommandError> {
    let skin: SavedSkin = serde_json::from_slice(&std::fs::read(skin_path(&paths, &id)?)?)?;
    let bytes = decode_texture(&skin.texture)?;
    if !["classic", "slim"].contains(&skin.model.as_str()) {
        return Err(CommandError::new("Ungültiges Skin-Modell."));
    }
    let account = microsoft::authenticated_account(&paths, &uuid).await?;
    let profile = profile_for(&account.access_token).await?;
    if let Some(cape_id) = &skin.cape_id {
        if skin.cape_account_uuid.as_deref() != Some(&uuid)
            || !profile.capes.iter().any(|cape| &cape.id == cape_id)
        {
            return Err(CommandError::new("Das gespeicherte Cape gehört nicht zu diesem Konto. Bitte den Skin bearbeiten und ein eigenes Cape auswählen."));
        }
    }
    let form = reqwest::multipart::Form::new()
        .text("variant", skin.model)
        .part(
            "file",
            reqwest::multipart::Part::bytes(bytes)
                .file_name("skin.png")
                .mime_str("image/png")?,
        );
    service_response(
        client()?
            .post(format!("{PROFILE_URL}/skins"))
            .bearer_auth(&account.access_token)
            .multipart(form)
            .send()
            .await?,
    )
    .await?;
    let client = client()?;
    let url = format!("{PROFILE_URL}/capes/active");
    let request = if let Some(cape_id) = skin.cape_id {
        client
            .put(url)
            .json(&serde_json::json!({"capeId": cape_id}))
    } else {
        client.delete(url)
    };
    // The two Minecraft endpoints are not transactional. Report partial success honestly.
    let cape_result = match request.bearer_auth(&account.access_token).send().await {
        Ok(response) => service_response(response).await.map(|_| ()),
        Err(error) => Err(error.into()),
    };
    Ok(ApplyResult {
        warning: cape_result
            .err()
            .map(|error| format!("Skin geändert, Cape konnte nicht geändert werden: {error}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer
                .write_image_data(&vec![255; (width * height * 4) as usize])
                .unwrap();
        }
        bytes
    }
    #[test]
    fn validates_skin_dimensions_and_decodes_full_png() {
        assert!(validate_png(&png(64, 64)).is_ok());
        assert!(validate_png(&png(64, 32)).is_ok());
        assert!(validate_png(&png(32, 32)).is_err());
        assert!(validate_png(&png(64, 64)[..40]).is_err());
        assert!(validate_png(&vec![0; MAX_PNG + 1]).is_err());
        assert!(decode_texture("https://example.com/skin.png").is_err());
    }
    #[test]
    fn canonical_skins_ignore_png_metadata() {
        let original = png(64, 64);
        let mut annotated = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut annotated, 64, 64);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .add_text_chunk("Author".into(), "Test".into())
                .unwrap();
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&vec![255; 64 * 64 * 4]).unwrap();
        }
        assert_ne!(original, annotated);
        assert_eq!(
            canonical_texture(&original).unwrap(),
            canonical_texture(&annotated).unwrap()
        );
    }

    #[test]
    #[ignore = "Contacts Mojang's public profile and texture services"]
    fn imports_public_skin_from_name_uuid_and_url() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let by_name = import_skin("name".into(), "Notch".into()).await.unwrap();
                assert!(decode_texture(&by_name.texture).is_ok());
                let identity: serde_json::Value = client()
                    .unwrap()
                    .get("https://api.mojang.com/users/profiles/minecraft/Notch")
                    .send()
                    .await
                    .unwrap()
                    .error_for_status()
                    .unwrap()
                    .json()
                    .await
                    .unwrap();
                let id = identity["id"].as_str().unwrap();
                let by_uuid = import_skin("uuid".into(), id.into()).await.unwrap();
                assert_eq!(by_name.texture, by_uuid.texture);
                let profile: serde_json::Value = client()
                    .unwrap()
                    .get(format!(
                        "https://sessionserver.mojang.com/session/minecraft/profile/{id}"
                    ))
                    .send()
                    .await
                    .unwrap()
                    .error_for_status()
                    .unwrap()
                    .json()
                    .await
                    .unwrap();
                let property = profile["properties"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|p| p["name"] == "textures")
                    .unwrap();
                let decoded = STANDARD
                    .decode(property["value"].as_str().unwrap())
                    .unwrap();
                let textures: serde_json::Value = serde_json::from_slice(&decoded).unwrap();
                let url = textures["textures"]["SKIN"]["url"]
                    .as_str()
                    .unwrap()
                    .replacen("http://", "https://", 1);
                let by_url = import_skin("url".into(), url).await.unwrap();
                assert_eq!(by_name.texture, by_url.texture);
            });
    }

    #[test]
    fn library_persists_edits_without_losing_other_skins() {
        let paths =
            PathManager::new(std::env::temp_dir().join(uuid::Uuid::new_v4().to_string())).unwrap();
        let mut skin = SavedSkin {
            id: uuid::Uuid::new_v4().to_string(),
            name: "  Test Skin  ".into(),
            texture: format!("data:image/png;base64,{}", STANDARD.encode(png(64, 64))),
            model: "classic".into(),
            cape_id: None,
            cape_account_uuid: None,
        };
        assert!(read_library(&paths).unwrap().is_empty());
        assert_eq!(write_skin(skin.clone(), &paths).unwrap().name, "Test Skin");
        let mut other = skin.clone();
        other.id = uuid::Uuid::new_v4().to_string();
        other.name = "Other Skin".into();
        write_skin(other.clone(), &paths).unwrap();
        skin.name = "Edited".into();
        skin.model = "slim".into();
        write_skin(skin.clone(), &paths).unwrap();
        let reloaded = read_library(&paths).unwrap();
        assert_eq!(reloaded.len(), 2);
        assert_eq!(
            reloaded
                .iter()
                .find(|item| item.id == skin.id)
                .unwrap()
                .model,
            "slim"
        );
        skin.model = "invalid".into();
        assert!(write_skin(skin.clone(), &paths).is_err());
        std::fs::remove_file(skin_path(&paths, &skin.id).unwrap()).unwrap();
        assert_eq!(read_library(&paths).unwrap()[0].id, other.id);
        std::fs::remove_dir_all(paths.root()).unwrap();
    }

    #[test]
    fn skin_ids_cannot_escape_library_directory() {
        let paths =
            PathManager::new(std::env::temp_dir().join(uuid::Uuid::new_v4().to_string())).unwrap();
        assert!(skin_path(&paths, "../account").is_err());
        let id = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            skin_path(&paths, &id).unwrap().parent().unwrap(),
            paths.root().join("skins")
        );
        std::fs::remove_dir_all(paths.root()).unwrap();
    }
}
