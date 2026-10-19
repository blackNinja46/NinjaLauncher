use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use std::sync::Mutex;

const APPLICATION_ID: &str = "1539265650395189348";
const LOGO_URL: &str = "https://raw.githubusercontent.com/blackNinja46/NinjaLauncher/main/static/brand/ninja-launcher-discord-icon.png";

struct PresenceState {
    enabled: bool,
    client: Option<DiscordIpcClient>,
}

pub struct DiscordPresence {
    state: Mutex<PresenceState>,
}

impl DiscordPresence {
    pub fn new(enabled: bool) -> Self {
        Self {
            state: Mutex::new(PresenceState {
                enabled,
                client: None,
            }),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state.enabled == enabled {
            return;
        }
        state.enabled = enabled;
        if !enabled {
            disconnect(&mut state);
            return;
        }
        drop(state);
        self.show_launcher();
    }

    pub fn show_launcher(&self) {
        self.publish("Minecraft Java Edition", "In the launcher");
    }

    pub fn show_launching(&self) {
        self.publish("Starting Minecraft", "Preparing game files");
    }

    pub fn show_playing(&self, instance_name: &str, minecraft_version: &str) {
        self.publish(
            format!("Playing Minecraft {minecraft_version}"),
            format!("Profile: {instance_name}"),
        );
    }

    fn publish(&self, details: impl Into<String>, state_text: impl Into<String>) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if !state.enabled {
            return;
        }
        let activity = activity::Activity::new()
            .details(details.into())
            .state(state_text.into())
            .assets(
                activity::Assets::new()
                    .large_image(LOGO_URL)
                    .large_text("Ninja Launcher"),
            );

        if let Some(client) = state.client.as_mut() {
            if client.set_activity(activity.clone()).is_ok() {
                return;
            }
            disconnect(&mut state);
        }

        let mut client = DiscordIpcClient::new(APPLICATION_ID);
        if client.connect().is_ok() && client.set_activity(activity).is_ok() {
            state.client = Some(client);
        } else {
            let _ = client.close();
        }
    }
}

fn disconnect(state: &mut PresenceState) {
    if let Some(mut client) = state.client.take() {
        let _ = client.clear_activity();
        let _ = client.close();
    }
}
