use directories::ProjectDirs;
use simpleddns_core::models::DdnsProfile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
 #[derive(Debug, Error)] pub enum StorageError {
    #[error("IO error: {0}")]     Io(#[from] std::io::Error),     #[error("JSON error: {0}")]     Json(#[from] serde_json::Error),     #[error("Encryption error: {0}")]     Encryption(String),     #[error("Failed to determine config directory")]     NoConfigDir, }
 #[derive(Debug, Clone, Serialize, Deserialize)] pub struct AppConfig {
    pub profiles: Vec<DdnsProfile>,     pub check_interval_minutes: u32,     pub run_on_startup: bool,     pub default_ipv4_urls: Vec<String>,     pub default_ipv6_urls: Vec<String>,     #[serde(default = "default_language")]     pub language: String, }
 fn default_language() -> String {
    "en".to_string() }
 impl Default for AppConfig {
    fn default() -> Self {
        Self {
            profiles: Vec::new(),             check_interval_minutes: 5,             run_on_startup: false,             default_ipv4_urls: vec![                 "https://v4.ident.me".to_string(),                 "https://api.ipify.org".to_string(),             ],             default_ipv6_urls: vec![                 "https://v6.ident.me".to_string(),                 "https://api64.ipify.org".to_string(),             ],             language: "en".to_string(),         }
    }
}
 #[derive(Debug, Clone, Serialize, Deserialize)] pub struct FullExport {
    pub config: AppConfig,     pub secrets: HashMap<String, String>, }
 pub struct StorageManager {
    config_path: PathBuf, }
 impl StorageManager {
    pub fn new() -> Result<Self, StorageError> {
        let proj_dirs = ProjectDirs::from("com", "sharkxuanbee", "SimpleDDNS")             .ok_or(StorageError::NoConfigDir)?;
         let config_dir = proj_dirs.config_dir();
        if !config_dir.exists() {
            fs::create_dir_all(config_dir)?;
        }
         let config_path = config_dir.join("config.json");
         Ok(Self {
            config_path,         })     }
     pub fn load_config(&self) -> Result<AppConfig, StorageError> {
        if !self.config_path.exists() {
            return Ok(AppConfig::default());
        }
        let data = fs::read_to_string(&self.config_path)?;
        let config: AppConfig = serde_json::from_str(&data)?;
        Ok(config)     }
     pub fn save_config(&self, config: &AppConfig) -> Result<(), StorageError> {
        let data = serde_json::to_string_pretty(config)?;
        let tmp_path = self.config_path.with_extension("tmp");
        fs::write(&tmp_path, &data)?;
        fs::rename(&tmp_path, &self.config_path)?;
        Ok(())     }
     pub fn save_secret(&self, profile_id: &str, secret: &str) -> Result<(), StorageError> {
        let entry = keyring::Entry::new("SimpleDDNS", profile_id)             .map_err(|e| StorageError::Encryption(e.to_string()))?;
        entry.set_password(secret)             .map_err(|e| StorageError::Encryption(e.to_string()))?;
        Ok(())     }
     pub fn load_secret(&self, profile_id: &str) -> Result<String, StorageError> {
        let entry = keyring::Entry::new("SimpleDDNS", profile_id)             .map_err(|e| StorageError::Encryption(e.to_string()))?;
        entry.get_password()             .map_err(|e| StorageError::Encryption(e.to_string()))     }
     pub fn delete_secret(&self, profile_id: &str) -> Result<(), StorageError> {
        if let Ok(entry) = keyring::Entry::new("SimpleDDNS", profile_id) {
            let _ = entry.delete_password();
        }
        Ok(())     }
     pub fn export_full_config(&self) -> Result<FullExport, StorageError> {
        let config = self.load_config()?;
        // Intentionally export empty secrets to prevent leakage in plain text.
        let secrets = HashMap::new();
        Ok(FullExport {
config, secrets })     }
     pub fn import_full_config(&self, export: FullExport) -> Result<(), StorageError> {
        self.save_config(&export.config)?;
        for (profile_id, secret) in export.secrets {
            let _ = self.save_secret(&profile_id, &secret);
        }
        Ok(())     }
     pub fn inject_tokens_into_profiles(&self, profiles: &mut Vec<DdnsProfile>) {
        for p in profiles.iter_mut() {
            if p.provider_type == "cloudflare" {
                let has_token = p.provider_config.get("api_token").is_some();
                if !has_token {
                    if let Ok(token) = self.load_secret(&p.id) {
                        if let Some(obj) = p.provider_config.as_object_mut() {
                            obj.insert("api_token".to_string(), serde_json::Value::String(token));
                        }
                    }
                }
            }
        }
    }
}