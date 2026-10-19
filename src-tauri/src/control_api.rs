use crate::error::CommandError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, time::Duration};

const DEFAULT_CONTROL_API_URL: &str = "https://admin.ninjalauncher.blackninja.live/api/v1";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherConfiguration {
    pub maintenance_enabled: bool,
    pub maintenance_active: bool,
    pub maintenance_message: String,
    pub maintenance_starts_at: Option<String>,
    pub maintenance_ends_at: Option<String>,
    pub registration_enabled: bool,
    pub recommended_minecraft_version: Option<String>,
    pub minimum_supported_launcher_version: Option<String>,
    pub latest_launcher_version: Option<String>,
    pub launcher_home_announcements_enabled: bool,
    pub launcher_notifications_enabled: bool,
    pub enabled_features: HashSet<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientBootstrap {
    pub generated_at: String,
    pub configuration: LauncherConfiguration,
    pub announcements: Vec<ClientAnnouncement>,
    pub minecraft_versions: Vec<ClientMinecraftVersion>,
    pub latest_release: Option<ClientRelease>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientAnnouncement {
    pub id: String,
    pub title: String,
    pub message: String,
    #[serde(rename = "type")]
    pub announcement_type: String,
    pub priority: String,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub display_types: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientMinecraftVersion {
    pub version: String,
    pub recommended: bool,
    pub release_type: String,
    pub loaders: Vec<ClientMinecraftLoader>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientMinecraftLoader {
    #[serde(rename = "type")]
    pub loader_type: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientRelease {
    pub version: String,
    pub build_number: u64,
    pub mandatory: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientEvent<'a> {
    #[serde(rename = "type")]
    event_type: &'a str,
    launcher_version: &'static str,
    minecraft_version: Option<&'a str>,
    operating_system: &'static str,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReportSubmission {
    pub client_report_id: String,
    pub profile_id: String,
    pub profile_name: String,
    pub minecraft_version: String,
    pub loader: String,
    pub account_uuid: Option<String>,
    pub account_name: Option<String>,
    pub exit_code: Option<i32>,
    pub title: String,
    pub message: String,
    pub suggestion: String,
    pub technical_detail: Option<String>,
    pub log_excerpt: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CrashReportEnvelope<'a> {
    client_report_id: &'a str,
    launcher_version: &'static str,
    operating_system: &'static str,
    profile_id: &'a str,
    profile_name: &'a str,
    minecraft_version: &'a str,
    loader: &'a str,
    account_uuid: Option<&'a str>,
    account_name: Option<&'a str>,
    exit_code: Option<i32>,
    title: &'a str,
    message: &'a str,
    suggestion: &'a str,
    technical_detail: Option<&'a str>,
    log_excerpt: &'a [String],
}

pub async fn fetch_launcher_bootstrap(
    minecraft_version: Option<&str>,
) -> Result<ClientBootstrap, CommandError> {
    let base_url = option_env!("NINJA_CONTROL_API_URL")
        .unwrap_or(DEFAULT_CONTROL_API_URL)
        .trim_end_matches('/');
    let mut query = vec![
        ("launcherVersion", env!("CARGO_PKG_VERSION")),
        ("operatingSystem", operating_system()),
        ("channel", "STABLE"),
        ("platform", release_platform()),
    ];
    if let Some(version) = minecraft_version {
        query.push(("minecraftVersion", version));
    }

    let response = http_client()?
        .get(format!("{base_url}/client/bootstrap"))
        .query(&query)
        .send()
        .await
        .map_err(|_| CommandError::new("The Ninja Launcher service could not be reached."))?;

    if !response.status().is_success() {
        return Err(CommandError::new(format!(
            "The Ninja Launcher service returned HTTP {}.",
            response.status().as_u16()
        )));
    }

    response.json().await.map_err(|_| {
        CommandError::new("The Ninja Launcher service returned invalid bootstrap data.")
    })
}

pub async fn record_event(
    event_type: &str,
    minecraft_version: Option<&str>,
) -> Result<(), CommandError> {
    if event_type != "LAUNCHER_START" && event_type != "GAME_START" {
        return Err(CommandError::new("Unsupported launcher event type."));
    }
    let base_url = option_env!("NINJA_CONTROL_API_URL")
        .unwrap_or(DEFAULT_CONTROL_API_URL)
        .trim_end_matches('/');
    let response = http_client()?
        .post(format!("{base_url}/client/events"))
        .json(&ClientEvent {
            event_type,
            launcher_version: env!("CARGO_PKG_VERSION"),
            minecraft_version,
            operating_system: operating_system(),
        })
        .send()
        .await
        .map_err(|_| CommandError::new("Anonymous usage data could not be sent."))?;
    if !response.status().is_success() {
        return Err(CommandError::new(format!(
            "The anonymous usage endpoint returned HTTP {}.",
            response.status().as_u16()
        )));
    }
    Ok(())
}

pub async fn upload_crash_report(report: &CrashReportSubmission) -> Result<(), CommandError> {
    validate_crash_report(report)?;
    let base_url = option_env!("NINJA_CONTROL_API_URL")
        .unwrap_or(DEFAULT_CONTROL_API_URL)
        .trim_end_matches('/');
    let response = http_client()?
        .post(format!("{base_url}/client/crash-reports"))
        .json(&CrashReportEnvelope {
            client_report_id: &report.client_report_id,
            launcher_version: env!("CARGO_PKG_VERSION"),
            operating_system: operating_system(),
            profile_id: &report.profile_id,
            profile_name: &report.profile_name,
            minecraft_version: &report.minecraft_version,
            loader: &report.loader,
            account_uuid: report.account_uuid.as_deref(),
            account_name: report.account_name.as_deref(),
            exit_code: report.exit_code,
            title: &report.title,
            message: &report.message,
            suggestion: &report.suggestion,
            technical_detail: report.technical_detail.as_deref(),
            log_excerpt: &report.log_excerpt,
        })
        .send()
        .await
        .map_err(|_| {
            CommandError::new("The crash report could not reach the Ninja Launcher dashboard.")
        })?;

    if matches!(response.status().as_u16(), 401 | 403 | 404) {
        return Err(CommandError::new(
            "Crash uploads are not enabled on the Ninja Launcher dashboard yet.",
        ));
    }
    if !response.status().is_success() {
        return Err(CommandError::new(format!(
            "The crash report endpoint returned HTTP {}.",
            response.status().as_u16()
        )));
    }
    Ok(())
}

fn validate_crash_report(report: &CrashReportSubmission) -> Result<(), CommandError> {
    let required = [
        report.client_report_id.trim(),
        report.profile_id.trim(),
        report.profile_name.trim(),
        report.minecraft_version.trim(),
        report.loader.trim(),
        report.title.trim(),
        report.message.trim(),
    ];
    if required.iter().any(|value| value.is_empty()) {
        return Err(CommandError::new("The crash report is incomplete."));
    }
    if report.profile_id.len() > 128
        || report.client_report_id.len() > 64
        || report.profile_name.len() > 256
        || report.minecraft_version.len() > 64
        || report.loader.len() > 32
        || report.title.len() > 256
        || report.message.len() > 4_000
        || report.suggestion.len() > 4_000
        || report
            .technical_detail
            .as_ref()
            .is_some_and(|value| value.len() > 32_000)
        || report.log_excerpt.len() > 200
        || report.log_excerpt.iter().map(String::len).sum::<usize>() > 128_000
    {
        return Err(CommandError::new(
            "The crash report is too large to upload.",
        ));
    }
    Ok(())
}

fn http_client() -> Result<Client, CommandError> {
    Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent(concat!("NinjaLauncher/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| CommandError::new("The Ninja Launcher service could not be prepared."))
}

fn operating_system() -> &'static str {
    match std::env::consts::OS {
        "windows" => "WINDOWS",
        "macos" => "MACOS",
        "linux" => "LINUX",
        _ => "OTHER",
    }
}

fn release_platform() -> &'static str {
    match std::env::consts::OS {
        "windows" => "WINDOWS",
        "macos" => "MACOS",
        "linux" => "LINUX",
        _ => "ALL",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_uses_backend_field_names() {
        let bootstrap: ClientBootstrap = serde_json::from_str(
            r#"{"generatedAt":"2026-09-19T12:00:00Z","configuration":{"maintenanceEnabled":true,"maintenanceActive":true,"maintenanceMessage":"Updating","maintenanceStartsAt":null,"maintenanceEndsAt":null,"registrationEnabled":true,"recommendedMinecraftVersion":"1.21.11","minimumSupportedLauncherVersion":"2.0.0","latestLauncherVersion":"2.0.1","launcherHomeAnnouncementsEnabled":true,"launcherNotificationsEnabled":true,"enabledFeatures":["news"],"updatedAt":"2026-09-19T12:00:00Z"},"announcements":[],"minecraftVersions":[{"version":"1.21.11","recommended":true,"releaseType":"RELEASE","loaders":[{"type":"FABRIC","version":"0.18.4"}]}],"latestRelease":null}"#,
        )
        .expect("valid launcher bootstrap");

        assert!(bootstrap.configuration.maintenance_active);
        assert_eq!(
            bootstrap
                .configuration
                .recommended_minecraft_version
                .as_deref(),
            Some("1.21.11")
        );
        assert!(bootstrap.configuration.enabled_features.contains("news"));
        assert_eq!(bootstrap.minecraft_versions[0].loaders[0].version, "0.18.4");
    }

    #[test]
    fn crash_report_uses_dashboard_field_names() {
        let report = CrashReportSubmission {
            client_report_id: "report-1".into(),
            profile_id: "default-fabric".into(),
            profile_name: "My Fabric World".into(),
            minecraft_version: "26.3".into(),
            loader: "fabric".into(),
            account_uuid: None,
            account_name: None,
            exit_code: Some(1),
            title: "Minecraft crashed".into(),
            message: "The game stopped unexpectedly.".into(),
            suggestion: "Review the console.".into(),
            technical_detail: Some("ExampleException".into()),
            log_excerpt: vec!["[stderr] ExampleException".into()],
        };
        validate_crash_report(&report).expect("valid report");

        let value = serde_json::to_value(CrashReportEnvelope {
            client_report_id: &report.client_report_id,
            launcher_version: "2.0.0-test",
            operating_system: "OTHER",
            profile_id: &report.profile_id,
            profile_name: &report.profile_name,
            minecraft_version: &report.minecraft_version,
            loader: &report.loader,
            account_uuid: None,
            account_name: None,
            exit_code: report.exit_code,
            title: &report.title,
            message: &report.message,
            suggestion: &report.suggestion,
            technical_detail: report.technical_detail.as_deref(),
            log_excerpt: &report.log_excerpt,
        })
        .expect("serializable report");

        assert_eq!(value["clientReportId"], "report-1");
        assert_eq!(value["minecraftVersion"], "26.3");
        assert_eq!(value["logExcerpt"][0], "[stderr] ExampleException");
    }
}
