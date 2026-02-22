use async_trait::async_trait;
use reqwest::Client;
use std::net::IpAddr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Other error: {0}")]
    Other(String),
}

#[async_trait]
pub trait DdnsProvider: Send + Sync {
    /// Gets the unique string id of the provider (e.g. "cloudflare")
    fn id(&self) -> &'static str;

    /// Updates the DNS record to the given IP addresses.
    async fn update_record(
        &self,
        domain: &str,
        ipv4: Option<IpAddr>,
        ipv6: Option<IpAddr>,
        config: &serde_json::Value,
        client: &Client,
    ) -> Result<(), ProviderError>;
}
