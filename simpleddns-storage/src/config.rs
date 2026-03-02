use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use simpleddns_core::models::DdnsProfile;
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
    #[error("Encryption error: {0}")]
    Encryption(String),
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

/// Returns the list of secret field names for a given provider type.
fn secret_fields_for_provider(provider_type: &str) -> &'static [&'static str] {
    match provider_type {
        "cloudflare" => &["api_token"],
        "aliyun" => &["access_key_id", "access_key_secret"],
        "dnspod" => &["login_token"],
        "godaddy" => &["key", "secret"],
        "namecheap" => &["api_key"],
        "generic" => &[], // generic has no secret fields by default
        _ => &[],
    }
}

pub struct StorageManager {
    config_path: PathBuf,
}

impl StorageManager {
    pub fn new() -> Result<Self, StorageError> {
        let proj_dirs = ProjectDirs::from("com", "sharkxuanbee", "SimpleDDNS")
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
        let tmp_path = self.config_path.with_extension("tmp");
        fs::write(&tmp_path, &data)?;
        fs::rename(&tmp_path, &self.config_path)?;
        Ok(())
    }

    pub fn save_secret(&self, key: &str, secret: &str) -> Result<(), StorageError> {
        let entry = keyring::Entry::new("SimpleDDNS", key)
            .map_err(|e| StorageError::Encryption(e.to_string()))?;
        entry
            .set_password(secret)
            .map_err(|e| StorageError::Encryption(e.to_string()))?;
        Ok(())
    }

    pub fn load_secret(&self, key: &str) -> Result<String, StorageError> {
        let entry = keyring::Entry::new("SimpleDDNS", key)
            .map_err(|e| StorageError::Encryption(e.to_string()))?;
        entry
            .get_password()
            .map_err(|e| StorageError::Encryption(e.to_string()))
    }

    pub fn delete_secret(&self, key: &str) -> Result<(), StorageError> {
        if let Ok(entry) = keyring::Entry::new("SimpleDDNS", key) {
            let _ = entry.delete_password();
        }
        Ok(())
    }

    /// Remove secret fields from a profile's provider_config and store them
    /// securely in the OS keyring. The profile is modified in-place.
    pub fn extract_and_save_secrets(&self, profile: &mut DdnsProfile) {
        let fields = secret_fields_for_provider(&profile.provider_type);
        if let Some(obj) = profile.provider_config.as_object_mut() {
            for field in fields {
                if let Some(val) = obj.remove(*field) {
                    if let Some(secret) = val.as_str() {
                        if !secret.is_empty() {
                            let keyring_key = format!("{}_{}", profile.id, field);
                            let _ = self.save_secret(&keyring_key, secret);
                        }
                    }
                }
            }
        }
    }

    /// Inject secrets from the keyring back into profile provider_config
    /// so they are available for the scheduler.
    pub fn inject_secrets_into_profiles(&self, profiles: &mut [DdnsProfile]) {
        for p in profiles.iter_mut() {
            let fields = secret_fields_for_provider(&p.provider_type);
            if let Some(obj) = p.provider_config.as_object_mut() {
                for field in fields {
                    // Only inject if not already present
                    if !obj.contains_key(*field) {
                        let keyring_key = format!("{}_{}", p.id, field);
                        if let Ok(secret) = self.load_secret(&keyring_key) {
                            obj.insert(
                                field.to_string(),
                                serde_json::Value::String(secret),
                            );
                        }
                    }
                }
            }

            // Migration: if the secret is already in the config (old format),
            // move it to keyring and remove from config.
            let fields = secret_fields_for_provider(&p.provider_type);
            let mut migrated = false;
            if let Some(obj) = p.provider_config.as_object_mut() {
                for field in fields {
                    if let Some(val) = obj.get(*field) {
                        if let Some(secret) = val.as_str() {
                            if !secret.is_empty() {
                                let keyring_key = format!("{}_{}", p.id, field);
                                // Try to save to keyring — if it works, we'll
                                // strip on next save.
                                let _ = self.save_secret(&keyring_key, secret);
                                migrated = true;
                            }
                        }
                    }
                }
            }
            let _ = migrated; // migration happens silently
        }
    }

    /// Delete all keyring secrets associated with a profile.
    pub fn delete_profile_secrets(&self, profile_id: &str, provider_type: &str) {
        let fields = secret_fields_for_provider(provider_type);
        for field in fields {
            let keyring_key = format!("{}_{}", profile_id, field);
            let _ = self.delete_secret(&keyring_key);
        }
        // Also clean up legacy key (old format used profile_id directly)
        let _ = self.delete_secret(profile_id);
    }

    pub fn export_full_config(&self) -> Result<FullExport, StorageError> {
        let config = self.load_config()?;
        // Export empty secrets to prevent leakage in plain text.
        // Secrets are stored in the OS keyring and not exported.
        let secrets = HashMap::new();
        Ok(FullExport { config, secrets })
    }

    pub fn import_full_config(&self, export: FullExport) -> Result<(), StorageError> {
        self.save_config(&export.config)?;
        for (key, secret) in export.secrets {
            let _ = self.save_secret(&key, &secret);
        }
        Ok(())
    }
}
