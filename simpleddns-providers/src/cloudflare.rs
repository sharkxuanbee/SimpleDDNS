use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use simpleddns_core::network::parse_domain_parts;
use simpleddns_core::provider::{DdnsProvider, ProviderError};
use std::net::IpAddr;
use tracing::{debug, info};

pub struct CloudflareProvider;

impl Default for CloudflareProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudflareProvider {
    pub fn new() -> Self {
        Self
    }
}

const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";

// --- Cloudflare API response types ---

#[derive(Deserialize, Debug)]
struct CfListResponse<T> {
    success: bool,
    result: Vec<T>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct CfZone {
    id: String,
    name: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct CfDnsRecord {
    id: String,
    name: String,
    #[serde(rename = "type")]
    record_type: String,
    content: String,
}

#[derive(Serialize, Debug)]
struct CfCreateRecord {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    content: String,
    ttl: u32,
    proxied: bool,
}

#[derive(Serialize, Debug)]
struct CfUpdateRecord {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    content: String,
    ttl: u32,
    proxied: bool,
}

// --- Helper functions ---

async fn get_zone_id(
    client: &Client,
    token: &str,
    zone_name: &str,
) -> Result<String, ProviderError> {
    let url = format!("{}/zones", CF_API_BASE);
    let resp: CfListResponse<CfZone> = client
        .get(&url)
        .query(&[("name", zone_name)])
        .bearer_auth(token)
        .send()
        .await?
        .json()
        .await?;

    if !resp.success || resp.result.is_empty() {
        return Err(ProviderError::Api(format!(
            "Zone '{}' not found or API error",
            zone_name
        )));
    }
    Ok(resp.result[0].id.clone())
}

async fn find_dns_record(
    client: &Client,
    token: &str,
    zone_id: &str,
    record_name: &str,
    record_type: &str,
) -> Result<Option<CfDnsRecord>, ProviderError> {
    let url = format!("{}/zones/{}/dns_records", CF_API_BASE, zone_id);
    let resp: CfListResponse<CfDnsRecord> = client
        .get(&url)
        .query(&[("type", record_type), ("name", record_name)])
        .bearer_auth(token)
        .send()
        .await?
        .json()
        .await?;

    if !resp.success {
        return Err(ProviderError::Api("Failed to list DNS records".into()));
    }
    Ok(resp.result.into_iter().next())
}

async fn create_dns_record(
    client: &Client,
    token: &str,
    zone_id: &str,
    record_type: &str,
    name: &str,
    content: &str,
) -> Result<(), ProviderError> {
    let url = format!("{}/zones/{}/dns_records", CF_API_BASE, zone_id);
    let body = CfCreateRecord {
        record_type: record_type.to_string(),
        name: name.to_string(),
        content: content.to_string(),
        ttl: 1, // Auto
        proxied: false,
    };
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(ProviderError::Api(format!(
            "Create DNS record failed: {}",
            text
        )));
    }
    info!(
        "Created {} record for {} -> {}",
        record_type, name, content
    );
    Ok(())
}

async fn update_dns_record(
    client: &Client,
    token: &str,
    zone_id: &str,
    record_id: &str,
    record_type: &str,
    name: &str,
    content: &str,
) -> Result<(), ProviderError> {
    let url = format!(
        "{}/zones/{}/dns_records/{}",
        CF_API_BASE, zone_id, record_id
    );
    let body = CfUpdateRecord {
        record_type: record_type.to_string(),
        name: name.to_string(),
        content: content.to_string(),
        ttl: 1,
        proxied: false,
    };
    let resp = client
        .put(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(ProviderError::Api(format!(
            "Update DNS record failed: {}",
            text
        )));
    }
    info!(
        "Updated {} record for {} -> {}",
        record_type, name, content
    );
    Ok(())
}

// --- Trait implementation ---

#[async_trait]
impl DdnsProvider for CloudflareProvider {
    fn id(&self) -> &'static str {
        "cloudflare"
    }

    async fn update_record(
        &self,
        domain: &str,
        ipv4: Option<IpAddr>,
        ipv6: Option<IpAddr>,
        config: &serde_json::Value,
        client: &Client,
    ) -> Result<(), ProviderError> {
        let token = config
            .get("api_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Config("Missing api_token".into()))?;

        // Determine zone name: explicit config or extract from domain
        let zone_name = config
            .get("zone_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                let (zone, _) = parse_domain_parts(domain);
                zone
            });

        debug!("Looking up zone ID for: {}", zone_name);
        let zone_id = get_zone_id(client, token, &zone_name).await?;
        debug!("Zone ID: {}", zone_id);

        // Update A record (IPv4)
        if let Some(ip) = ipv4 {
            let ip_str = ip.to_string();
            match find_dns_record(client, token, &zone_id, domain, "A").await? {
                Some(existing) => {
                    if existing.content != ip_str {
                        update_dns_record(
                            client,
                            token,
                            &zone_id,
                            &existing.id,
                            "A",
                            domain,
                            &ip_str,
                        )
                        .await?;
                    } else {
                        debug!("A record already up-to-date: {}", ip_str);
                    }
                }
                None => {
                    create_dns_record(client, token, &zone_id, "A", domain, &ip_str).await?;
                }
            }
        }

        // Update AAAA record (IPv6)
        if let Some(ip) = ipv6 {
            let ip_str = ip.to_string();
            match find_dns_record(client, token, &zone_id, domain, "AAAA").await? {
                Some(existing) => {
                    if existing.content != ip_str {
                        update_dns_record(
                            client,
                            token,
                            &zone_id,
                            &existing.id,
                            "AAAA",
                            domain,
                            &ip_str,
                        )
                        .await?;
                    } else {
                        debug!("AAAA record already up-to-date: {}", ip_str);
                    }
                }
                None => {
                    create_dns_record(client, token, &zone_id, "AAAA", domain, &ip_str).await?;
                }
            }
        }

        Ok(())
    }
}