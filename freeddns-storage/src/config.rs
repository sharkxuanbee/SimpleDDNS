use directories::ProjectDirs;
use freeddns_core::models::DdnsProfile;
use keyring::Entry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub check_interval_minutes: u32,
    pub run_on_startup: bool,
    pub default_ipv4_urls: Vec<String>,
    pub default_ipv6_urls: Vec<String>,
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String {
    "en".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            profiles: Vec::new(),
            check_interval_minutes: 5,
            run_on_startup: false,
            default_ipv4_urls: vec![
                "https://v4.ident.me".to_string(),
                "https://api.ipify.org".to_string(),
            ],
            default_ipv6_urls: vec![
                "https://v6.ident.me".to_string(),
                "https://api64.ipify.org".to_string(),
            ],
            language: "en".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullExport {
    pub config: AppConfig,
    pub secrets: HashMap<String, String>,
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

    pub fn export_full_config(&self) -> Result<FullExport, StorageError> {
        let config = self.load_config()?;
        let mut secrets = HashMap::new();

        for profile in &config.profiles {
            if let Ok(secret) = self.load_secret(&profile.id) {
                secrets.insert(profile.id.clone(), secret);
            }
        }

        Ok(FullExport { config, secrets })
    }

    pub fn import_full_config(&self, export: FullExport) -> Result<(), StorageError> {
        self.save_config(&export.config)?;

        for (profile_id, secret) in export.secrets {
            self.save_secret(&profile_id, &secret)?;
        }

        Ok(())
    }
}
