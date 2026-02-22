use directories::ProjectDirs;
use freeddns_core::models::DdnsProfile;
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("Failed to determine config directory")]
    NoConfigDir,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub profiles: Vec<DdnsProfile>,
    pub run_on_startup: bool,
    pub check_interval_minutes: u32,
    pub default_ipv4_urls: Vec<String>,
    pub default_ipv6_urls: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            profiles: vec![],
            run_on_startup: false,
            check_interval_minutes: 5,
            default_ipv4_urls: vec!["https://api.ipify.org".to_string()],
            default_ipv6_urls: vec!["https://api6.ipify.org".to_string()],
        }
    }
}

pub struct StorageManager {
    config_path: PathBuf,
}

impl StorageManager {
    pub fn new() -> Result<Self, StorageError> {
        let proj_dirs = ProjectDirs::from("com", "sharkxuanbee", "freeddns")
            .ok_or(StorageError::NoConfigDir)?;

        let config_dir = proj_dirs.config_dir();
        if !config_dir.exists() {
            fs::create_dir_all(config_dir)?;
        }

        let config_path = config_dir.join("config.json");
        Ok(Self { config_path })
    }

    pub fn load_config(&self) -> Result<AppConfig, StorageError> {
        if !self.config_path.exists() {
            return Ok(AppConfig::default());
        }
        let data = fs::read_to_string(&self.config_path)?;
        let config: AppConfig = serde_json::from_str(&data)?;
        Ok(config)
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), StorageError> {
        let data = serde_json::to_string_pretty(config)?;
        fs::write(&self.config_path, data)?;
        Ok(())
    }

    pub fn save_secret(&self, profile_id: &str, secret: &str) -> Result<(), StorageError> {
        let target = format!("freeddns_{}", profile_id);
        let entry = Entry::new(&target, "ddns_user")?;
        entry.set_password(secret)?;
        Ok(())
    }

    pub fn load_secret(&self, profile_id: &str) -> Result<String, StorageError> {
        let target = format!("freeddns_{}", profile_id);
        let entry = Entry::new(&target, "ddns_user")?;
        let secret = entry.get_password()?;
        Ok(secret)
    }

    pub fn delete_secret(&self, profile_id: &str) -> Result<(), StorageError> {
        let target = format!("freeddns_{}", profile_id);
        let entry = Entry::new(&target, "ddns_user")?;
        let _ = entry.delete_credential(); // Ignore if it doesn't exist
        Ok(())
    }
}
