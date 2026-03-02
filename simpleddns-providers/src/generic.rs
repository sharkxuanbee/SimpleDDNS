use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Client;
use simpleddns_core::provider::{DdnsProvider, ProviderError};
use std::net::IpAddr;
use tracing::info;

pub struct GenericHttpProvider;

impl Default for GenericHttpProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GenericHttpProvider {
    pub fn new() -> Self {
        Self
    }
}

fn render_template(
    template: &str,
    domain: &str,
    ipv4: Option<IpAddr>,
    ipv6: Option<IpAddr>,
) -> String {
    let v4_str = ipv4.map(|ip| ip.to_string()).unwrap_or_default();
    let v6_str = ipv6.map(|ip| ip.to_string()).unwrap_or_default();

    template
        .replace("{domain}", domain)
        .replace("{ipv4}", &v4_str)
        .replace("{ipv6}", &v6_str)
}

#[async_trait]
impl DdnsProvider for GenericHttpProvider {
    fn id(&self) -> &'static str {
        "generic"
    }

    async fn update_record(
        &self,
        domain: &str,
        ipv4: Option<IpAddr>,
        ipv6: Option<IpAddr>,
        config: &serde_json::Value,
        client: &Client,
    ) -> Result<(), ProviderError> {
        let url_template = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Config("Missing URL template".into()))?;

        let method = config
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        let final_url = render_template(url_template, domain, ipv4, ipv6);
        info!("Generic provider {} -> {}", method, final_url);

        // Build custom headers
        let mut headers = HeaderMap::new();
        if let Some(header_obj) = config.get("headers").and_then(|v| v.as_object()) {
            for (key, val) in header_obj {
                if let Some(val_str) = val.as_str() {
                    let rendered = render_template(val_str, domain, ipv4, ipv6);
                    if let (Ok(name), Ok(value)) =
                        (key.parse::<HeaderName>(), rendered.parse::<HeaderValue>())
                    {
                        headers.insert(name, value);
                    }
                }
            }
        }

        let resp = match method.as_str() {
            "POST" | "PUT" | "PATCH" | "DELETE" => {
                let body = config.get("body").and_then(|v| v.as_str()).unwrap_or("");
                let rendered_body = render_template(body, domain, ipv4, ipv6);

                let builder = match method.as_str() {
                    "POST" => client.post(&final_url),
                    "PUT" => client.put(&final_url),
                    "PATCH" => client.patch(&final_url),
                    "DELETE" => client.delete(&final_url),
                    _ => unreachable!(),
                };

                builder
                    .headers(headers)
                    .header("Content-Type", "application/json")
                    .body(rendered_body)
                    .send()
                    .await?
            }
            _ => {
                // Default GET
                client.get(&final_url).headers(headers).send().await?
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!("HTTP {} - {}", status, text)));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_template() {
        let result = render_template(
            "https://api.example.com/update?domain={domain}&ip={ipv4}",
            "test.example.com",
            Some("1.2.3.4".parse().unwrap()),
            None,
        );
        assert_eq!(
            result,
            "https://api.example.com/update?domain=test.example.com&ip=1.2.3.4"
        );
    }

    #[test]
    fn test_render_template_ipv6() {
        let result = render_template(
            "{\"ipv4\":\"{ipv4}\",\"ipv6\":\"{ipv6}\"}",
            "test.example.com",
            None,
            Some("2001:db8::1".parse().unwrap()),
        );
        assert_eq!(result, "{\"ipv4\":\"\",\"ipv6\":\"2001:db8::1\"}");
    }
}