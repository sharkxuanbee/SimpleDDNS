use async_trait::async_trait;
use simpleddns_core::provider::{DdnsProvider, ProviderError};
use reqwest::Client;
use serde::Deserialize;
use std::net::IpAddr;
use tracing::{debug, info};

pub struct NamecheapProvider;

impl Default for NamecheapProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl NamecheapProvider {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize, Debug)]
struct NamecheapResponse {
    #[serde(rename = "CommandResponse")]
    command_response: Option<NamecheapCommandResponse>,
    #[serde(rename = "ErrCount")]
    err_count: Option<String>,
    #[serde(rename = "errors")]
    errors: Option<NamecheapErrors>,
}

#[derive(Deserialize, Debug)]
struct NamecheapErrors {
    #[serde(rename = "Error")]
    error: Option<Vec<NamecheapError>>,
}

#[derive(Deserialize, Debug)]
struct NamecheapError {
    #[serde(rename = "Number")]
    number: Option<String>,
    #[serde(rename = "Description")]
    description: Option<String>,
}

#[derive(Deserialize, Debug)]
struct NamecheapCommandResponse {
    #[serde(rename = "DomainDNSSetDBRows")]
    domain_dns_set: Option<NamecheapDnsSetResponse>,
    #[serde(rename = "domain")]
    domain: Option<NamecheapDomainResult>,
}

#[derive(Deserialize, Debug)]
struct NamecheapDnsSetResponse {
    #[serde(rename = "rr")]
    rr: Option<Vec<NamecheapRecord>>,
}

#[derive(Deserialize, Debug)]
struct NamecheapDomainResult {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "DNS")]
    dns: NamecheapDNS,
}

#[derive(Deserialize, Debug)]
struct NamecheapDNS {
    #[serde(rename = "DNS1")]
    dns1: String,
    #[serde(rename = "DNS2")]
    dns2: Option<String>,
}

#[derive(Deserialize, Debug)]
struct NamecheapRecord {
    #[serde(rename = "ID")]
    id: Option<String>,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Type")]
    record_type: String,
    #[serde(rename = "Address")]
    address: String,
    #[serde(rename = "TTL")]
    ttl: Option<String>,
}

fn build_signature(
    api_key: &str,
    api_user: &str,
    user_name: &str,
    command: &str,
    client_ip: &str,
) -> String {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let pre_hash = format!("{}{}{}{}{}", api_key, command, api_user, user_name, timestamp);
    let hash = md5::compute(pre_hash.as_bytes());
    format!("{}--{:x}", timestamp, hash)
}

async fn set_dns(
    client: &Client,
    api_key: &str,
    api_user: &str,
    user_name: &str,
    client_ip: &str,
    domain: &str,
    hostname: &str,
    record_type: &str,
    value: &str,
    ttl: &str,
) -> Result<(), ProviderError> {
    let command = "namecheap.domains.dns.setHosts";
    let signature = build_signature(api_key, api_user, user_name, command, client_ip);
    
    let url = "https://api.namecheap.com/xml.response";
    let params = [
        ("ApiUser", user_name),
        ("ApiKey", api_key),
        ("UserName", user_name),
        ("Command", command),
        ("ClientIP", client_ip),
        ("Signature", &signature),
        ("DomainName", domain),
        ("HostName1", hostname),
        ("RecordType1", record_type),
        ("Address1", value),
        ("TTL1", ttl),
    ];
    
    let resp = client
        .get(url)
        .query(&params)
        .send()
        .await?;
    
    let text = resp.text().await;
    
    if let Ok(response_text) = text {
        if response_text.contains("<Errors>") || response_text.contains("<Error>") {
            return Err(ProviderError::Api(format!("Namecheap set DNS failed: {}", response_text)));
        }
    }
    
    info!("Namecheap: Set {} record for {}.{} -> {}", record_type, hostname, domain, value);
    Ok(())
}

#[async_trait]
impl DdnsProvider for NamecheapProvider {
    fn id(&self) -> &'static str {
        "namecheap"
    }

    async fn update_record(
        &self,
        domain: &str,
        ipv4: Option<IpAddr>,
        ipv6: Option<IpAddr>,
        config: &serde_json::Value,
        client: &Client,
    ) -> Result<(), ProviderError> {
        let api_key = config
            .get("api_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Config("Missing api_key".into()))?;
        
        let api_user = config
            .get("api_user")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Config("Missing api_user".into()))?;
        
        let user_name = config
            .get("user_name")
            .and_then(|v| v.as_str())
            .unwrap_or(api_user);
        
        let client_ip = config
            .get("client_ip")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Config("Missing client_ip (your public IP)".into()))?;
        
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
        
        let ttl = config
            .get("ttl")
            .and_then(|v| v.as_str())
            .unwrap_or("1800");
        
        debug!("Namecheap: zone={}, sub_domain={}", zone_name, sub_domain);
        
        if let Some(ip) = ipv4 {
            let ip_str = ip.to_string();
            set_dns(
                client,
                api_key,
                api_user,
                user_name,
                client_ip,
                &zone_name,
                &sub_domain,
                "A",
                &ip_str,
                ttl,
            ).await?;
        }
        
        if let Some(ip) = ipv6 {
            let ip_str = ip.to_string();
            set_dns(
                client,
                api_key,
                api_user,
                user_name,
                client_ip,
                &zone_name,
                &sub_domain,
                "AAAA",
                &ip_str,
                ttl,
            ).await?;
        }
        
        Ok(())
    }
}
