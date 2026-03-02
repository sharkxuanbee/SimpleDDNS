use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use simpleddns_core::network::parse_domain_parts;
use simpleddns_core::provider::{DdnsProvider, ProviderError};
use std::collections::BTreeMap;
use std::net::IpAddr;
use tracing::{debug, info};

use base64::Engine;

pub struct AliyunProvider;

impl Default for AliyunProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AliyunProvider {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize, Debug)]
struct AliyunDescribeRecordResponse {
    pub records: Option<AliyunRecords>,
}

#[derive(Deserialize, Debug)]
struct AliyunRecords {
    pub record: Option<Vec<AliyunRecord>>,
}

#[derive(Deserialize, Debug)]
struct AliyunRecord {
    pub record_id: String,
    pub value: String,
}

struct AliyunContext<'a> {
    client: &'a Client,
    access_key_id: &'a str,
    access_key_secret: &'a str,
}

fn sign(secret: &str, params: &BTreeMap<String, String>) -> String {
    let sorted: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    let string_to_sign = sorted.join("&");
    let string_to_sign = format!("POST&%2F&{}", urlencoding::encode(&string_to_sign));

    let key = format!("{}&", secret);
    let mac = hmac_sha1::hmac_sha1(key.as_bytes(), string_to_sign.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(mac)
}

async fn find_record(
    ctx: &AliyunContext<'_>,
    domain: &str,
    record_type: &str,
) -> Result<Option<AliyunRecord>, ProviderError> {
    let mut params = BTreeMap::new();
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    params.insert("Format".to_string(), "JSON".to_string());
    params.insert("Version".to_string(), "2015-01-09".to_string());
    params.insert("AccessKeyId".to_string(), ctx.access_key_id.to_string());
    params.insert("SignatureMethod".to_string(), "HMAC-SHA1".to_string());
    params.insert("Timestamp".to_string(), timestamp);
    params.insert("SignatureVersion".to_string(), "1.0".to_string());
    params.insert(
        "SignatureNonce".to_string(),
        uuid::Uuid::new_v4().to_string(),
    );
    params.insert("Action".to_string(), "DescribeSubDomainRecords".to_string());
    params.insert("SubDomain".to_string(), domain.to_string());
    params.insert("Type".to_string(), record_type.to_string());
    params.insert("PageSize".to_string(), "500".to_string());

    let signature = sign(ctx.access_key_secret, &params);
    params.insert("Signature".to_string(), signature);

    let url = "https://alidns.aliyuncs.com/";
    let resp: AliyunDescribeRecordResponse = ctx
        .client
        .post(url)
        .form(&params)
        .send()
        .await?
        .json()
        .await
        .map_err(|e| ProviderError::Api(format!("Failed to parse response: {}", e)))?;

    if let Some(records) = resp.records {
        if let Some(record_list) = records.record {
            if let Some(record) = record_list.into_iter().next() {
                return Ok(Some(record));
            }
        }
    }
    Ok(None)
}

async fn add_record(
    ctx: &AliyunContext<'_>,
    domain: &str,
    sub_domain: &str,
    record_type: &str,
    value: &str,
) -> Result<(), ProviderError> {
    let mut params = BTreeMap::new();
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    params.insert("Format".to_string(), "JSON".to_string());
    params.insert("Version".to_string(), "2015-01-09".to_string());
    params.insert("AccessKeyId".to_string(), ctx.access_key_id.to_string());
    params.insert("SignatureMethod".to_string(), "HMAC-SHA1".to_string());
    params.insert("Timestamp".to_string(), timestamp);
    params.insert("SignatureVersion".to_string(), "1.0".to_string());
    params.insert(
        "SignatureNonce".to_string(),
        uuid::Uuid::new_v4().to_string(),
    );
    params.insert("Action".to_string(), "AddDomainRecord".to_string());
    params.insert("DomainName".to_string(), domain.to_string());
    params.insert("RR".to_string(), sub_domain.to_string());
    params.insert("Type".to_string(), record_type.to_string());
    params.insert("Value".to_string(), value.to_string());
    params.insert("TTL".to_string(), "600".to_string());

    let signature = sign(ctx.access_key_secret, &params);
    params.insert("Signature".to_string(), signature);

    let url = "https://alidns.aliyuncs.com/";
    let resp = ctx.client.post(url).form(&params).send().await?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(ProviderError::Api(format!("Add record failed: {}", text)));
    }

    info!(
        "Aliyun: Created {} record for {}.{} -> {}",
        record_type, sub_domain, domain, value
    );
    Ok(())
}

async fn update_record(
    ctx: &AliyunContext<'_>,
    record_id: &str,
    domain: &str,
    sub_domain: &str,
    record_type: &str,
    value: &str,
) -> Result<(), ProviderError> {
    let mut params = BTreeMap::new();
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    params.insert("Format".to_string(), "JSON".to_string());
    params.insert("Version".to_string(), "2015-01-09".to_string());
    params.insert("AccessKeyId".to_string(), ctx.access_key_id.to_string());
    params.insert("SignatureMethod".to_string(), "HMAC-SHA1".to_string());
    params.insert("Timestamp".to_string(), timestamp);
    params.insert("SignatureVersion".to_string(), "1.0".to_string());
    params.insert(
        "SignatureNonce".to_string(),
        uuid::Uuid::new_v4().to_string(),
    );
    params.insert("Action".to_string(), "UpdateDomainRecord".to_string());
    params.insert("RecordId".to_string(), record_id.to_string());
    params.insert("RR".to_string(), sub_domain.to_string());
    params.insert("Type".to_string(), record_type.to_string());
    params.insert("Value".to_string(), value.to_string());
    params.insert("TTL".to_string(), "600".to_string());

    let signature = sign(ctx.access_key_secret, &params);
    params.insert("Signature".to_string(), signature);

    let url = "https://alidns.aliyuncs.com/";
    let resp = ctx.client.post(url).form(&params).send().await?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(ProviderError::Api(format!(
            "Update record failed: {}",
            text
        )));
    }

    info!(
        "Aliyun: Updated {} record for {}.{} -> {}",
        record_type, sub_domain, domain, value
    );
    Ok(())
}

#[async_trait]
impl DdnsProvider for AliyunProvider {
    fn id(&self) -> &'static str {
        "aliyun"
    }

    async fn update_record(
        &self,
        domain: &str,
        ipv4: Option<IpAddr>,
        ipv6: Option<IpAddr>,
        config: &serde_json::Value,
        client: &Client,
    ) -> Result<(), ProviderError> {
        let access_key_id = config
            .get("access_key_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Config("Missing access_key_id".into()))?;

        let access_key_secret = config
            .get("access_key_secret")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Config("Missing access_key_secret".into()))?;

        let ctx = AliyunContext {
            client,
            access_key_id,
            access_key_secret,
        };

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

        debug!("Aliyun: zone={}, sub_domain={}", zone_name, sub_domain);

        if let Some(ip) = ipv4 {
            let ip_str = ip.to_string();
            let full_domain = if sub_domain == "@" {
                zone_name.clone()
            } else {
                format!("{}.{}", sub_domain, zone_name)
            };

            match find_record(&ctx, &full_domain, "A").await? {
                Some(existing) => {
                    if existing.value != ip_str {
                        update_record(
                            &ctx,
                            &existing.record_id,
                            &zone_name,
                            &sub_domain,
                            "A",
                            &ip_str,
                        )
                        .await?;
                    } else {
                        debug!("Aliyun: A record already up-to-date: {}", ip_str);
                    }
                }
                None => {
                    add_record(&ctx, &zone_name, &sub_domain, "A", &ip_str).await?;
                }
            }
        }

        if let Some(ip) = ipv6 {
            let ip_str = ip.to_string();
            let full_domain = if sub_domain == "@" {
                zone_name.clone()
            } else {
                format!("{}.{}", sub_domain, zone_name)
            };

            match find_record(&ctx, &full_domain, "AAAA").await? {
                Some(existing) => {
                    if existing.value != ip_str {
                        update_record(
                            &ctx,
                            &existing.record_id,
                            &zone_name,
                            &sub_domain,
                            "AAAA",
                            &ip_str,
                        )
                        .await?;
                    } else {
                        debug!("Aliyun: AAAA record already up-to-date: {}", ip_str);
                    }
                }
                None => {
                    add_record(&ctx, &zone_name, &sub_domain, "AAAA", &ip_str).await?;
                }
            }
        }

        Ok(())
    }
}
