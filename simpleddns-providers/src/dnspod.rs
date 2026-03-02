use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use simpleddns_core::network::parse_domain_parts;
use simpleddns_core::provider::{DdnsProvider, ProviderError};
use std::net::IpAddr;
use tracing::{debug, info};

pub struct DnspodProvider;

impl Default for DnspodProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DnspodProvider {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize, Debug)]
struct DnspodResponse {
    pub status: DnspodStatus,
    #[serde(rename = "records")]
    records: Option<Vec<DnspodRecord>>,
}

#[derive(Deserialize, Debug)]
struct DnspodStatus {
    pub code: String,
    pub message: String,
}

#[derive(Deserialize, Debug)]
struct DnspodRecord {
    pub id: String,
    pub value: String,
}

#[derive(Deserialize, Debug)]
struct DnspodActionResponse {
    pub status: DnspodStatus,
}

async fn find_record(
    client: &Client,
    login_token: &str,
    domain: &str,
    sub_domain: &str,
    record_type: &str,
    record_line: &str,
) -> Result<Option<DnspodRecord>, ProviderError> {
    let params = [
        ("login_token", login_token),
        ("format", "json"),
        ("domain", domain),
        ("sub_domain", sub_domain),
        ("record_type", record_type),
        ("record_line", record_line),
    ];

    let url = "https://dnsapi.cn/Record.List";
    let resp: DnspodResponse = client
        .post(url)
        .form(&params)
        .send()
        .await?
        .json()
        .await
        .map_err(|e| ProviderError::Api(format!("Failed to parse response: {}", e)))?;

    if resp.status.code != "1" {
        if resp.status.code == "-15" {
            return Ok(None);
        }
        return Err(ProviderError::Api(format!(
            "DNSPod API error: {} - {}",
            resp.status.code, resp.status.message
        )));
    }

    if let Some(records) = resp.records {
        if let Some(record) = records.into_iter().next() {
            return Ok(Some(record));
        }
    }

    Ok(None)
}

async fn add_record(
    client: &Client,
    login_token: &str,
    domain: &str,
    sub_domain: &str,
    record_type: &str,
    value: &str,
    record_line: &str,
) -> Result<(), ProviderError> {
    let params = [
        ("login_token", login_token),
        ("format", "json"),
        ("domain", domain),
        ("sub_domain", sub_domain),
        ("record_type", record_type),
        ("value", value),
        ("record_line", record_line),
        ("ttl", "600"),
    ];

    let url = "https://dnsapi.cn/Record.Create";
    let resp: DnspodActionResponse = client
        .post(url)
        .form(&params)
        .send()
        .await?
        .json()
        .await
        .map_err(|e| ProviderError::Api(format!("Failed to parse response: {}", e)))?;

    if resp.status.code != "1" {
        return Err(ProviderError::Api(format!(
            "DNSPod create record error: {} - {}",
            resp.status.code, resp.status.message
        )));
    }

    info!(
        "DNSPod: Created {} record for {}.{} -> {}",
        record_type, sub_domain, domain, value
    );
    Ok(())
}

async fn update_record(
    client: &Client,
    login_token: &str,
    domain: &str,
    record_id: &str,
    sub_domain: &str,
    record_type: &str,
    value: &str,
    record_line: &str,
) -> Result<(), ProviderError> {
    let params = [
        ("login_token", login_token),
        ("format", "json"),
        ("domain", domain),
        ("record_id", record_id),
        ("sub_domain", sub_domain),
        ("record_type", record_type),
        ("value", value),
        ("record_line", record_line),
    ];

    let url = "https://dnsapi.cn/Record.Modify";
    let resp: DnspodActionResponse = client
        .post(url)
        .form(&params)
        .send()
        .await?
        .json()
        .await
        .map_err(|e| ProviderError::Api(format!("Failed to parse response: {}", e)))?;

    if resp.status.code != "1" {
        return Err(ProviderError::Api(format!(
            "DNSPod update record error: {} - {}",
            resp.status.code, resp.status.message
        )));
    }

    info!(
        "DNSPod: Updated {} record for {}.{} -> {}",
        record_type, sub_domain, domain, value
    );
    Ok(())
}

#[async_trait]
impl DdnsProvider for DnspodProvider {
    fn id(&self) -> &'static str {
        "dnspod"
    }

    async fn update_record(
        &self,
        domain: &str,
        ipv4: Option<IpAddr>,
        ipv6: Option<IpAddr>,
        config: &serde_json::Value,
        client: &Client,
    ) -> Result<(), ProviderError> {
        let login_token = config
            .get("login_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProviderError::Config("Missing login_token (format: id,token)".into())
            })?;

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

        let record_line = config
            .get("record_line")
            .and_then(|v| v.as_str())
            .unwrap_or("默认");

        debug!("DNSPod: zone={}, sub_domain={}, record_line={}", zone_name, sub_domain, record_line);

        if let Some(ip) = ipv4 {
            let ip_str = ip.to_string();

            match find_record(client, login_token, &zone_name, &sub_domain, "A", record_line).await? {
                Some(existing) => {
                    if existing.value != ip_str {
                        update_record(
                            client,
                            login_token,
                            &zone_name,
                            &existing.id,
                            &sub_domain,
                            "A",
                            &ip_str,
                            record_line,
                        )
                        .await?;
                    } else {
                        debug!("DNSPod: A record already up-to-date: {}", ip_str);
                    }
                }
                None => {
                    add_record(client, login_token, &zone_name, &sub_domain, "A", &ip_str, record_line).await?;
                }
            }
        }

        if let Some(ip) = ipv6 {
            let ip_str = ip.to_string();

            match find_record(client, login_token, &zone_name, &sub_domain, "AAAA", record_line).await? {
                Some(existing) => {
                    if existing.value != ip_str {
                        update_record(
                            client,
                            login_token,
                            &zone_name,
                            &existing.id,
                            &sub_domain,
                            "AAAA",
                            &ip_str,
                            record_line,
                        )
                        .await?;
                    } else {
                        debug!("DNSPod: AAAA record already up-to-date: {}", ip_str);
                    }
                }
                None => {
                    add_record(
                        client,
                        login_token,
                        &zone_name,
                        &sub_domain,
                        "AAAA",
                        &ip_str,
                        record_line,
                    )
                    .await?;
                }
            }
        }

        Ok(())
    }
}
