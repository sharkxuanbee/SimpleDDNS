use async_trait::async_trait;
use simpleddns_core::provider::{DdnsProvider, ProviderError};
use reqwest::Client;
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

struct NamecheapContext<'a> {
    client: &'a Client,
    api_key: &'a str,
    api_user: &'a str,
    user_name: &'a str,
    client_ip: &'a str,
}

fn build_signature(
    api_key: &str,
    api_user: &str,
    user_name: &str,
    command: &str,
) -> String {
    // Namecheap API requires a specific signature format:
    // md5(api_key + command + api_user + user_name + timestamp)
    // Note: This logic seems to be a placeholder implementation in the original code,
    // as Namecheap API usually doesn't require this specific signature for simple DNS updates
    // if using the correct endpoint, or it might be for a different API version.
    // However, preserving the logic while removing unused parameters.
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let pre_hash = format!("{}{}{}{}{}", api_key, command, api_user, user_name, timestamp);
    let hash = md5::compute(pre_hash.as_bytes());
    format!("{}--{:x}", timestamp, hash)
}

async fn set_dns(
    ctx: &NamecheapContext<'_>,
    domain: &str,
    hostname: &str,
    record_type: &str,
    value: &str,
    ttl: &str,
) -> Result<(), ProviderError> {
    let command = "namecheap.domains.dns.setHosts";
    // client_ip was unused in build_signature
    let signature = build_signature(ctx.api_key, ctx.api_user, ctx.user_name, command);
    
    // Note: The URL seems to be generic. In a real implementation, this should probably be
    // https://api.namecheap.com/xml.response for production or sandbox URL.
    let url = "https://api.namecheap.com/xml.response";
    let params = [
        ("ApiUser", ctx.user_name),
        ("ApiKey", ctx.api_key),
        ("UserName", ctx.user_name),
        ("Command", command),
        ("ClientIP", ctx.client_ip),
        ("Signature", &signature),
        ("DomainName", domain),
        ("HostName1", hostname),
        ("RecordType1", record_type),
        ("Address1", value),
        ("TTL1", ttl),
    ];
    
    let resp = ctx.client
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
        
        let ctx = NamecheapContext {
            client,
            api_key,
            api_user,
            user_name,
            client_ip,
        };
        
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
                &ctx,
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
                &ctx,
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
