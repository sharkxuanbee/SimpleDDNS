use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IpVersion {
    IPv4,
    IPv6,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdnsProfile {
    pub id: String,
    pub name: String,
    pub provider_type: String, // "cloudflare" or "generic"
    pub domain: String,
    pub enable_ipv4: bool,
    pub enable_ipv6: bool,
    pub enabled: bool,
    // JSON details for the provider configuration
    pub provider_config: serde_json::Value,
}

impl Default for DdnsProfile {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "New Profile".to_string(),
            provider_type: "cloudflare".to_string(),
            domain: "".to_string(),
            enable_ipv4: true,
            enable_ipv6: false,
            enabled: true,
            provider_config: serde_json::json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileStatus {
    pub profile_id: String,
    pub last_update: Option<chrono::DateTime<chrono::Utc>>,
    pub current_ipv4: Option<String>,
    pub current_ipv6: Option<String>,
    pub status_message: String,
    pub is_running: bool,
}
