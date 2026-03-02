use async_trait::async_trait;
use reqwest::Client;
use simpleddns_core::network::parse_domain_parts;
use simpleddns_core::provider::{DdnsProvider, ProviderError};
use std::net::IpAddr;
use tracing::{debug, info};

/// Namecheap DDNS provider using the simple Dynamic DNS update URL.
///
/// This uses Namecheap's dedicated DDNS endpoint which is safe — it only
/// updates the specified host record without affecting other DNS records.
///
/// Required config fields:
/// - `api_key`: The Dynamic DNS password from Namecheap (not the API key)
///
/// The domain is split into zone (SLD.TLD) and host automatically.
/// Namecheap DDNS only supports A records (IPv4). IPv6 AAAA records are
/// not supported by this endpoint.
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

/// Update a single host record via Namecheap's Dynamic DNS URL.
///
/// Endpoint: `https://dynamicdns.park-your-domain.com/update`
///
/// This is safe because it only updates the specified host, not all DNS records.
async fn update_ddns(
    client: &Client,
    domain: &str,
    host: &str,
    password: &str,
    ip: &str,
) -> Result<(), ProviderError> {
    let url = "https://dynamicdns.park-your-domain.com/update";
    let params = [
        ("host", host),
        ("domain", domain),
        ("password", password),
        ("ip", ip),
    ];

    let resp = client.get(url).query(&params).send().await?;

    let text = resp.text().await.unwrap_or_default();

    // The response is XML. Check for errors.
    if text.contains("<ErrCount>0</ErrCount>") {
        info!(
            "Namecheap: Updated A record for {}.{} -> {}",
            host, domain, ip
        );
        Ok(())
    } else if text.contains("<errors>") || text.contains("<Err") {
        Err(ProviderError::Api(format!(
            "Namecheap DDNS update failed: {}",
            text
        )))
    } else {
        // If we can't parse the response, treat as success with warning
        debug!("Namecheap: Unexpected response format: {}", text);
        Ok(())
    }
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
        let password = config
            .get("api_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Config("Missing api_key (DDNS password)".into()))?;

        let zone_name = config
            .get("zone_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                let (zone, _) = parse_domain_parts(domain);
                zone
            });

        let sub_domain = config
            .get("sub_domain")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                let (_, sub) = parse_domain_parts(domain);
                sub
            });

        debug!("Namecheap: zone={}, sub_domain={}", zone_name, sub_domain);

        if let Some(ip) = ipv4 {
            let ip_str = ip.to_string();
            update_ddns(client, &zone_name, &sub_domain, password, &ip_str).await?;
        }

        // Namecheap DDNS endpoint does not support AAAA records
        if ipv6.is_some() {
            debug!("Namecheap: IPv6 (AAAA) records are not supported by the DDNS endpoint");
        }

        Ok(())
    }
}
