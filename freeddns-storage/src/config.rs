use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use directories::ProjectDirs;
use freeddns_core::models::DdnsProfile;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

pub struct StorageManager {
    config_path: PathBuf,
    secrets_path: PathBuf,
    encryption_key: [u8; 32],
}

/// Derive a 32-byte AES key from machine-specific data.
fn derive_key() -> [u8; 32] {
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown-host".to_string());

    let mut hasher = Sha256::new();
    hasher.update(hostname.as_bytes());
    hasher.update(b"freeddns-salt-v1");
    let result = hasher.finalize();

    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Encrypt plaintext using AES-256-GCM. Returns base64(nonce || ciphertext).
fn encrypt_secret(key: &[u8; 32], plaintext: &str) -> Result<String, StorageError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| StorageError::Encryption(format!("Failed to create cipher: {}", e)))?;

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| StorageError::Encryption(format!("Encryption failed: {}", e)))?;

    // Concatenate nonce + ciphertext, then base64 encode
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(&combined))
}

/// Decrypt a base64(nonce || ciphertext) string using AES-256-GCM.
fn decrypt_secret(key: &[u8; 32], encoded: &str) -> Result<String, StorageError> {
    let combined = BASE64
        .decode(encoded)
        .map_err(|e| StorageError::Encryption(format!("Base64 decode failed: {}", e)))?;

    if combined.len() < 13 {
        return Err(StorageError::Encryption(
            "Encrypted data too short".to_string(),
        ));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| StorageError::Encryption(format!("Failed to create cipher: {}", e)))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| StorageError::Encryption(format!("Decryption failed: {}", e)))?;

    String::from_utf8(plaintext)
        .map_err(|e| StorageError::Encryption(format!("UTF-8 decode failed: {}", e)))
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
        let secrets_path = config_dir.join("secrets.enc");
        let encryption_key = derive_key();

        Ok(Self {
            config_path,
            secrets_path,
            encryption_key,
        })
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

    /// Load the encrypted secrets map from disk.
    fn load_secrets_map(&self) -> HashMap<String, String> {
        if !self.secrets_path.exists() {
            return HashMap::new();
        }
        fs::read_to_string(&self.secrets_path)
            .ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    /// Save the secrets map to disk.
    fn save_secrets_map(&self, map: &HashMap<String, String>) -> Result<(), StorageError> {
        let data = serde_json::to_string_pretty(map)?;
        fs::write(&self.secrets_path, data)?;
        Ok(())
    }

    pub fn save_secret(&self, profile_id: &str, secret: &str) -> Result<(), StorageError> {
        let encrypted = encrypt_secret(&self.encryption_key, secret)?;
        let mut map = self.load_secrets_map();
        map.insert(profile_id.to_string(), encrypted);
        self.save_secrets_map(&map)?;
        Ok(())
    }

    pub fn load_secret(&self, profile_id: &str) -> Result<String, StorageError> {
        let map = self.load_secrets_map();
        let encrypted = map.get(profile_id).ok_or_else(|| {
            StorageError::Encryption(format!("No secret found for profile '{}'", profile_id))
        })?;
        decrypt_secret(&self.encryption_key, encrypted)
    }

    pub fn delete_secret(&self, profile_id: &str) -> Result<(), StorageError> {
        let mut map = self.load_secrets_map();
        map.remove(profile_id);
        self.save_secrets_map(&map)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = derive_key();
        let plaintext = "my-super-secret-api-token-12345";

        let encrypted = encrypt_secret(&key, plaintext).expect("Encryption failed");
        assert_ne!(encrypted, plaintext);

        let decrypted = decrypt_secret(&key, &encrypted).expect("Decryption failed");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_encryptions_differ() {
        let key = derive_key();
        let plaintext = "same-token";

        let enc1 = encrypt_secret(&key, plaintext).unwrap();
        let enc2 = encrypt_secret(&key, plaintext).unwrap();

        // Random nonce means different ciphertext each time
        assert_ne!(enc1, enc2);

        // But both decrypt to the same plaintext
        assert_eq!(decrypt_secret(&key, &enc1).unwrap(), plaintext);
        assert_eq!(decrypt_secret(&key, &enc2).unwrap(), plaintext);
    }
}
