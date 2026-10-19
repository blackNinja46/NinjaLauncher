use crate::error::CommandError;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Manager};

#[derive(Clone, Debug)]
pub struct PathManager {
    root: PathBuf,
    content_root: Arc<RwLock<PathBuf>>,
}

impl PathManager {
    pub fn from_app(app: &AppHandle) -> Result<Self, CommandError> {
        let root = app
            .path()
            .app_data_dir()
            .map_err(|error| CommandError::new(format!("Could not locate app data: {error}")))?;
        Self::new(root)
    }

    pub fn new(root: PathBuf) -> Result<Self, CommandError> {
        std::fs::create_dir_all(&root)?;
        Ok(Self {
            content_root: Arc::new(RwLock::new(root.clone())),
            root,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("launcher.json")
    }

    pub fn content_root(&self) -> PathBuf {
        self.content_root
            .read()
            .expect("content root lock poisoned")
            .clone()
    }

    pub fn set_content_root(&self, path: Option<&str>) -> Result<(), CommandError> {
        let root = path
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.root.clone());
        if !root.is_absolute() {
            return Err(CommandError::new(
                "The installation directory must be an absolute path.",
            ));
        }
        std::fs::create_dir_all(&root)?;
        *self
            .content_root
            .write()
            .map_err(|_| CommandError::new("Could not update the installation directory."))? = root;
        Ok(())
    }

    pub fn instances_dir(&self) -> PathBuf {
        self.content_root().join("instances")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.content_root().join("logs")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.content_root().join("cache")
    }

    pub fn shared_dir(&self) -> PathBuf {
        self.content_root().join("shared")
    }

    pub fn account_file(&self) -> PathBuf {
        self.root.join("account.json")
    }

    pub fn running_instances_file(&self) -> PathBuf {
        self.root.join("running-instances.json")
    }

    pub fn ensure_directories(&self) -> Result<(), CommandError> {
        for path in [
            self.instances_dir(),
            self.logs_dir(),
            self.cache_dir(),
            self.shared_dir(),
        ] {
            std::fs::create_dir_all(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_all_paths_from_a_single_root() {
        let root = std::env::temp_dir().join("ninjalauncher-path-test");
        let manager = PathManager::new(root.clone()).expect("path manager");
        assert_eq!(manager.instances_dir(), root.join("instances"));
        assert_eq!(manager.config_file(), root.join("launcher.json"));
    }

    #[test]
    fn rejects_relative_content_roots() {
        let root = std::env::temp_dir().join("ninjalauncher-relative-path-test");
        let manager = PathManager::new(root).expect("path manager");
        assert!(manager.set_content_root(Some("relative/path")).is_err());
    }
}
