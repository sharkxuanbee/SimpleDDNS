use async_trait::async_trait;
use simpleddns_core::provider::{DdnsProvider, ProviderError};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::collections::BTreeMap;
use base64::Engine;
use tracing::{debug, info};

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
struct AliyunDescribeDomainsResponse {
    pub request_id: String,
    pub domains: Option<AliyunDomains>,
}

#[derive(Deserialize, Debug)]
struct AliyunDomains {
    pub domain: Option<Vec<AliyunDomain>>,
}

#[derive(Deserialize, Debug)]
struct AliyunDomain {
    pub domain_name: String,
}

#[derive(Deserialize, Debug)]
struct AliyunDescribeRecordResponse {
    pub request_id: String,
    pub records: Option<AliyunRecords>,
}

#[derive(Deserialize, Debug)]
struct AliyunRecords {
    pub record: Option<Vec<AliyunRecord>>,
}

#[derive(Deserialize, Debug)]
struct AliyunRecord {
    pub record_id: String,
    pub rr: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub value: String,
}

#[derive(Serialize, Debug)]
struct AliyunAddRecordRequest {
    #[serde(rename = "RR")]
    rr: String,
    #[serde(rename = "Type")]
    record_type: String,
    #[serde(rename = "Value")]
    value: String,
    #[serde(rename = "TTL")]
    ttl: i64,
}

fn sign(secret: &str, params: &BTreeMap<String, String>) -> String {
    let mut sorted: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    sorted.sort();
    let string_to_sign = sorted.join("&");
    let string_to_sign = format!("POST&%2F&{}", urlencoding::encode(&string_to_sign));
    
    let key = format!("{}&", secret);
    let mac = hmac_sha1::hmac_sha1(key.as_bytes(), string_to_sign.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(&mac)
}

async fn find_record(
    client: &Client,
    access_key_id: &str,
    access_key_secret: &str,
    domain: &str,
    record_type: &str,
) -> Result<Option<AliyunRecord>, ProviderError> {
    let mut params = BTreeMap::new();
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    params.insert("Format".to_string(), "JSON".to_string());
    params.insert("Version".to_string(), "2015-01-09".to_string());
    params.insert("AccessKeyId".to_string(), access_key_id.to_string());
    params.insert("SignatureMethod".to_string(), "HMAC-SHA1".to_string());
    params.insert("Timestamp".to_string(), timestamp.clone());
    params.insert("SignatureVersion".to_string(), "1.0".to_string());
    params.insert("SignatureNonce".to_string(), uuid::Uuid::new_v4().to_string());
    params.insert("Action".to_string(), "DescribeSubDomainRecords".to_string());
    params.insert("SubDomain".to_string(), domain.to_string());
    params.insert("Type".to_string(), record_type.to_string());
    
    let signature = sign(access_key_secret, &params);
    params.insert("Signature".to_string(), signature);
    
    let url = "https://alidns.aliyuncs.com/";
    let resp: AliyunDescribeRecordResponse = client
        .post(url)
        .form(&params)
        .send()
        .await?
        .json()
        .await
        .map_err(|e| ProviderError::Api(format!("Failed to parse response: {}", e)))?;
    
    if let Some(records) = resp.records {
        if let Some(record_list) = records.record {
            for record in record_list {
                return Ok(Some(record));
            }
        }
    }
    Ok(None)
}

async fn add_record(
    client: &Client,
    access_key_id: &str,
    access_key_secret: &str,
    domain: &str,
    sub_domain: &str,
    record_type: &str,
    value: &str,
) -> Result<(), ProviderError> {
    let mut params = BTreeMap::new();
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    params.insert("Format".to_string(), "JSON".to_string());
    params.insert("Version".to_string(), "2015-01-09".to_string());
    params.insert("AccessKeyId".to_string(), access_key_id.to_string());
    params.insert("SignatureMethod".to_string(), "HMAC-SHA1".to_string());
    params.insert("Timestamp".to_string(), timestamp.clone());
    params.insert("SignatureVersion".to_string(), "1.0".to_string());
    params.insert("SignatureNonce".to_string(), uuid::Uuid::new_v4().to_string());
    params.insert("Action".to_string(), "AddDomainRecord".to_string());
    params.insert("DomainName".to_string(), domain.to_string());
    params.insert("RR".to_string(), sub_domain.to_string());
    params.insert("Type".to_string(), record_type.to_string());
    params.insert("Value".to_string(), value.to_string());
    params.insert("TTL".to_string(), "600".to_string());
    
    let signature = sign(access_key_secret, &params);
    params.insert("Signature".to_string(), signature);
    
    let url = "https://alidns.aliyuncs.com/";
    let resp = client
        .post(url)
        .form(&params)
        .send()
        .await?;
    
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(ProviderError::Api(format!("Add record failed: {}", text)));
    }
    
    info!("Aliyun: Created {} record for {}.{} -> {}", record_type, sub_domain, domain, value);
    Ok(())
}

async fn update_record(
    client: &Client,
    access_key_id: &str,
    access_key_secret: &str,
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
    params.insert("AccessKeyId".to_string(), access_key_id.to_string());
    params.insert("SignatureMethod".to_string(), "HMAC-SHA1".to_string());
    params.insert("Timestamp".to_string(), timestamp.clone());
    params.insert("SignatureVersion".to_string(), "1.0".to_string());
    params.insert("SignatureNonce".to_string(), uuid::Uuid::new_v4().to_string());
    params.insert("Action".to_string(), "UpdateDomainRecord".to_string());
    params.insert("RecordId".to_string(), record_id.to_string());
    params.insert("RR".to_string(), sub_domain.to_string());
    params.insert("Type".to_string(), record_type.to_string());
    params.insert("Value".to_string(), value.to_string());
    params.insert("TTL".to_string(), "600".to_string());
    
    let signature = sign(access_key_secret, &params);
    params.insert("Signature".to_string(), signature);
    
    let url = "https://alidns.aliyuncs.com/";
    let resp = client
        .post(url)
        .form(&params)
        .send()
        .await?;
    
    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(ProviderError::Api(format!("Update record failed: {}", text)));
    }
    
    info!("Aliyun: Updated {} record for {}.{} -> {}", record_type, sub_domain, domain, value);
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
        
        debug!("Aliyun: zone={}, sub_domain={}", zone_name, sub_domain);
        
        if let Some(ip) = ipv4 {
            let ip_str = ip.to_string();
            let full_domain = if sub_domain == "@" {
                zone_name.clone()
            } else {
                format!("{}.{}", sub_domain, zone_name)
            };
            
            match find_record(client, access_key_id, access_key_secret, &full_domain, "A").await? {
                Some(existing) => {
                    if existing.value != ip_str {
                        update_record(
                            client,
                            access_key_id,
                            access_key_secret,
                            &existing.record_id,
                            &zone_name,
                            &sub_domain,
                            "A",
                            &ip_str,
                        ).await?;
                    } else {
                        debug!("Aliyun: A record already up-to-date: {}", ip_str);
                    }
                }
                None => {
                    add_record(client, access_key_id, access_key_secret, &zone_name, &sub_domain, "A", &ip_str).await?;
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
            
            match find_record(client, access_key_id, access_key_secret, &full_domain, "AAAA").await? {
                Some(existing) => {
                    if existing.value != ip_str {
                        update_record(
                            client,
                            access_key_id,
                            access_key_secret,
                            &existing.record_id,
                            &zone_name,
                            &sub_domain,
                            "AAAA",
                            &ip_str,
                        ).await?;
                    } else {
                        debug!("Aliyun: AAAA record already up-to-date: {}", ip_str);
                    }
                }
                None => {
                    add_record(client, access_key_id, access_key_secret, &zone_name, &sub_domain, "AAAA", &ip_str).await?;
                }
            }
        }
        
        Ok(())
    }
}
