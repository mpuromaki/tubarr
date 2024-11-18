//! Standard configuration paths

use std::path::PathBuf;

use thiserror::Error;

/// Application details, used for paths on some platforms.
#[derive(Debug)]
pub struct AppDetails {
    pub name: String,
    pub organization: String,
    pub tld: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Unsupported platform for system configuration path")]
    UnsupportedPlatform,
}

// Returns system path for application configuration
pub fn system_configuration(app: &AppDetails) -> Result<PathBuf, ConfigError> {
    if cfg!(target_os = "linux") {
        Ok(PathBuf::from("/etc").join(&app.name))
    } else {
        Err(ConfigError::UnsupportedPlatform)
    }
}
