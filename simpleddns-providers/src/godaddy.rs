use async_trait::async_trait;
use simpleddns_core::provider::{DdnsProvider, ProviderError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use tracing::{debug, info};

pub struct GodaddyProvider;

impl Default for GodaddyProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GodaddyProvider {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize, Debug)]
struct GodaddyResponse {
    pub code: Option<String>,
    pub message: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GodaddyRecord {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    data: String,
    ttl: Option<i64>,
}

#[derive(Serialize, Debug)]
struct GodaddyRecordRequest {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    data: String,
    ttl: i64,
}

fn build_auth_header(key: &str, secret: &str) -> String {
    format!("sso-key {}:{}", key, secret)
}

async fn find_record(
    client: &Client,
    key: &str,
    secret: &str,
    domain: &str,
    name: &str,
    record_type: &str,
) -> Result<Option<GodaddyRecord>, ProviderError> {
    let url = format!(
        "https://api.godaddy.com/v1/domains/{}/records/{}/{}",
        domain, record_type, name
    );
    
    let auth = build_auth_header(key, secret);
    
    let resp: Vec<GodaddyRecord> = client
        .get(&url)
        .header("Authorization", auth)
        .header("Content-Type", "application/json")
        .send()
        .await?
        .json()
        .await
        .map_err(|e| ProviderError::Api(format!("Failed to find record: {}", e)))?;
    
    if let Some(record) = resp.into_iter().next() {
        Ok(Some(record))
    } else {
        Ok(None)
    }
}

async fn set_record(
    client: &Client,
    key: &str,
    secret: &str,
    domain: &str,
    name: &str,
    record_type: &str,
    value: &str,
) -> Result<(), ProviderError> {
    let url = format!(
        "https://api.godaddy.com/v1/domains/{}/records/{}/{}",
        domain, record_type, name
    );
    
    let auth = build_auth_header(key, secret);
    
    let record = GodaddyRecordRequest {
        record_type: record_type.to_string(),
        name: name.to_string(),
        data: value.to_string(),
        ttl: 3600,
    };
    
    let resp = client
        .put(&url)
        .header("Authorization", auth)
        .header("Content-Type", "application/json")
        .json(&[record])
        .send()
        .await?;
    
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(ProviderError::Api(format!("GoDaddy set record failed: {}", text)));
    }
    
    info!("GoDaddy: Set {} record for {}.{} -> {}", record_type, name, domain, value);
    Ok(())
}

#[async_trait]
impl DdnsProvider for GodaddyProvider {
    fn id(&self) -> &'static str {
        "godaddy"
    }

    async fn update_record(
        &self,
        domain: &str,
        ipv4: Option<IpAddr>,
        ipv6: Option<IpAddr>,
        config: &serde_json::Value,
        client: &Client,
    ) -> Result<(), ProviderError> {
        let key = config
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Config("Missing key".into()))?;
        
        let secret = config
            .get("secret")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Config("Missing secret".into()))?;
        
        let zone_name = config
            .get("zone_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                let parts: Vec<&str> = domain.rsplitn(3, '.').collect();
                if parts.len() >= 2 {
                    format!("{}.{}", parts[1], parts[0])
                } else {
                    domain.to_string()
                }
            });
        
        let sub_domain = config
            .get("sub_domain")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                let parts: Vec<&str> = domain.rsplitn(3, '.').collect();
                if parts.len() >= 3 {
                    parts[2..].iter().copied().rev().collect::<Vec<_>>().join(".")
                } else {
                    "@".to_string()
                }
            });
        
        let record_name = if sub_domain == "@" || sub_domain.is_empty() {
            "@".to_string()
        } else {
            sub_domain
        };
        
        debug!("GoDaddy: zone={}, record_name={}", zone_name, record_name);
        
        if let Some(ip) = ipv4 {
            let ip_str = ip.to_string();
            set_record(client, key, secret, &zone_name, &record_name, "A", &ip_str).await?;
        }
        
        if let Some(ip) = ipv6 {
            let ip_str = ip.to_string();
            set_record(client, key, secret, &zone_name, &record_name, "AAAA", &ip_str).await?;
        }
        
        Ok(())
    }
}
