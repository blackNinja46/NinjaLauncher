use crate::{
    error::CommandError,
    process::{hide_std_console, hide_tokio_console},
    storage::paths::PathManager,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, process::Command, sync::Mutex};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunningProfile {
    instance_id: String,
    pid: u32,
}

pub struct GameProcesses {
    file: PathBuf,
    running: Mutex<HashMap<String, u32>>,
}

impl GameProcesses {
    pub fn new(paths: &PathManager) -> Self {
        let file = paths.running_instances_file();
        let saved: Vec<RunningProfile> = std::fs::read(&file)
            .ok()
            .and_then(|data| serde_json::from_slice(&data).ok())
            .unwrap_or_default();
        let running = saved
            .into_iter()
            .filter(|entry| process_matches_profile(entry.pid, &entry.instance_id))
            .map(|entry| (entry.instance_id, entry.pid))
            .collect();
        let manager = Self {
            file,
            running: Mutex::new(running),
        };
        manager.persist();
        manager
    }

    pub fn register(&self, instance_id: &str, pid: u32) -> Result<(), CommandError> {
        let mut running = self.running.lock().expect("game process registry");
        if running
            .get(instance_id)
            .is_some_and(|saved| process_matches_profile(*saved, instance_id))
        {
            return Err(CommandError::new("This profile is already running."));
        }
        running.insert(instance_id.into(), pid);
        drop(running);
        self.persist();
        Ok(())
    }

    pub fn unregister(&self, instance_id: &str, pid: u32) {
        let mut running = self.running.lock().expect("game process registry");
        if running.get(instance_id) == Some(&pid) {
            running.remove(instance_id);
        }
        drop(running);
        self.persist();
    }

    pub fn list(&self) -> Vec<String> {
        let mut running = self.running.lock().expect("game process registry");
        running.retain(|instance_id, pid| process_matches_profile(*pid, instance_id));
        let mut ids: Vec<_> = running.keys().cloned().collect();
        ids.sort();
        drop(running);
        self.persist();
        ids
    }

    pub async fn stop(&self, instance_id: &str) -> Result<(), CommandError> {
        let pid = self
            .running
            .lock()
            .expect("game process registry")
            .get(instance_id)
            .copied()
            .ok_or_else(|| CommandError::new("This profile is no longer running."))?;
        if !process_matches_profile(pid, instance_id) {
            self.unregister(instance_id, pid);
            return Err(CommandError::new(
                "The saved Minecraft process is no longer running.",
            ));
        }
        #[cfg(windows)]
        let status = {
            let mut command = tokio::process::Command::new("taskkill");
            hide_tokio_console(&mut command);
            command
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .status()
                .await?
        };
        #[cfg(not(windows))]
        let status = {
            let mut command = tokio::process::Command::new("kill");
            hide_tokio_console(&mut command);
            command.args(["-TERM", &pid.to_string()]).status().await?
        };
        if !status.success() {
            return Err(CommandError::new("Minecraft could not be stopped."));
        }
        self.unregister(instance_id, pid);
        Ok(())
    }

    fn persist(&self) {
        let running = self.running.lock().expect("game process registry");
        let entries: Vec<_> = running
            .iter()
            .map(|(instance_id, pid)| RunningProfile {
                instance_id: instance_id.clone(),
                pid: *pid,
            })
            .collect();
        if let Ok(data) = serde_json::to_vec_pretty(&entries) {
            let _ = std::fs::write(&self.file, data);
        }
    }
}

#[cfg(not(windows))]
fn process_matches_profile(pid: u32, instance_id: &str) -> bool {
    let mut command = Command::new("ps");
    hide_std_console(&mut command);
    command
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| {
            let command = String::from_utf8_lossy(&output.stdout);
            command.contains(instance_id) && command.to_lowercase().contains("java")
        })
}

#[cfg(windows)]
fn process_matches_profile(pid: u32, instance_id: &str) -> bool {
    let query = format!("(Get-CimInstance Win32_Process -Filter 'ProcessId = {pid}').CommandLine");
    let mut command = Command::new("powershell");
    hide_std_console(&mut command);
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &query,
        ])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| {
            let command = String::from_utf8_lossy(&output.stdout);
            command.contains(instance_id) && command.to_lowercase().contains("java")
        })
}
