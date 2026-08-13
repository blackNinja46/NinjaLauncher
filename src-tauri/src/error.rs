use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub message: String,
}

impl CommandError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for CommandError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for CommandError {}

impl From<std::io::Error> for CommandError {
    fn from(error: std::io::Error) -> Self {
        Self::new(format!("A storage operation failed: {error}"))
    }
}

impl From<serde_json::Error> for CommandError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(format!("Launcher configuration is invalid: {error}"))
    }
}

impl From<reqwest::Error> for CommandError {
    fn from(error: reqwest::Error) -> Self {
        Self::new(format!("The online service could not be reached: {error}"))
    }
}
