use crate::models::{DdnsProfile, IpVersion, ProfileStatus};
use crate::provider::DdnsProvider;
use crate::resolver::{resolve_ip, ResolverError};
use chrono::Utc;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Mutex};
use tracing::{error, info, warn};

/// Shared state between the scheduler and the GUI.
pub type SharedStatus = Arc<Mutex<HashMap<String, ProfileStatus>>>;
pub type SharedLogs = Arc<Mutex<Vec<String>>>;
pub struct DdnsScheduler {
    client: Client,
    statuses: SharedStatus,
    logs: SharedLogs,
}
impl DdnsScheduler {
    pub fn new(statuses: SharedStatus, logs: SharedLogs) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| Client::new()),
            statuses,
            logs,
        }
    }
    fn log(&self, logs: &mut Vec<String>, mut msg: String) {
        if msg.len() > 256 {
            msg.truncate(253);
            msg.push_str("...");
        }
        let ts = Utc::now().format("%H:%M:%S");
        let entry = format!("[{}] {}", ts, msg);
        logs.push(entry);
        // Keep last 100 log entries to save memory
        if logs.len() > 100 {
            logs.drain(0..logs.len() - 100);
        }
    }
    pub async fn run_once(
        &self,
        profiles: &[DdnsProfile],
        providers: &HashMap<String, Box<dyn DdnsProvider>>,
        ipv4_urls: &[String],
        ipv6_urls: &[String],
    ) {
        for profile in profiles.iter().filter(|p| p.enabled) {
            let provider = match providers.get(&profile.provider_type) {
                Some(p) => p,
                None => {
                    let msg = format!(
                        "Unknown provider '{}' for profile '{}'",
                        profile.provider_type, profile.name
                    );
                    warn!("{}", msg);
                    let mut logs = self.logs.lock().await;
                    self.log(&mut logs, msg);
                    continue;
                }
            };
            // Resolve IPv4
            let ipv4 = if profile.enable_ipv4 && !ipv4_urls.is_empty() {
                match self.resolve_first(ipv4_urls, IpVersion::IPv4).await {
                    Ok(ip) => Some(ip),
                    Err(e) => {
                        let msg = format!("[{}] IPv4 resolve failed: {}", profile.name, e);
                        warn!("{}", msg);
                        let mut logs = self.logs.lock().await;
                        self.log(&mut logs, msg);
                        None
                    }
                }
            }
else {
                None
            };
            // Resolve IPv6
            let ipv6 = if profile.enable_ipv6 && !ipv6_urls.is_empty() {
                match self.resolve_first(ipv6_urls, IpVersion::IPv6).await {
                    Ok(ip) => Some(ip),
                    Err(e) => {
                        let msg = format!("[{}] IPv6 resolve failed: {}", profile.name, e);
                        warn!("{}", msg);
                        let mut logs = self.logs.lock().await;
                        self.log(&mut logs, msg);
                        None
                    }
                }
            }
else {
                None
            };
            if ipv4.is_none() && ipv6.is_none() {
                let msg = format!("[{}] No IP resolved, skipping update", profile.name);
                info!("{}", msg);
                let mut logs = self.logs.lock().await;
                self.log(&mut logs, msg);
                continue;
            }
            // Read token from provider_config (will be populated from keyring at app level)
            let config = &profile.provider_config;
            match provider
                .update_record(&profile.domain, ipv4, ipv6, config, &self.client)
                .await
            {
                Ok(()) => {
                    let msg = format!(
                        "[{}] Updated {}
- IPv4: {}
IPv6: {}",
                        profile.name,
                        profile.domain,
                        ipv4.map(|ip| ip.to_string()).unwrap_or_else(|| "N/A".into()),
                        ipv6.map(|ip| ip.to_string()).unwrap_or_else(|| "N/A".into()),
                    );
                    info!("{}", msg);
                    let mut logs = self.logs.lock().await;
                    self.log(&mut logs, msg);
                    let mut statuses = self.statuses.lock().await;
                    statuses.insert(
                        profile.id.clone(),
                        ProfileStatus {
                            profile_id: profile.id.clone(),
                            last_update: Some(Utc::now()),
                            current_ipv4: ipv4.map(|ip| ip.to_string()),
                            current_ipv6: ipv6.map(|ip| ip.to_string()),
                            status_message: "OK".into(),
                            is_running: false,
                        },
                    );
                }
                Err(e) => {
                    let msg = format!("[{}] Update failed: {}", profile.name, e);
                    error!("{}", msg);
                    let mut logs = self.logs.lock().await;
                    self.log(&mut logs, msg);
                    let mut statuses = self.statuses.lock().await;
                    statuses.insert(
                        profile.id.clone(),
                        ProfileStatus {
                            profile_id: profile.id.clone(),
                            last_update: Some(Utc::now()),
                            current_ipv4: ipv4.map(|ip| ip.to_string()),
                            current_ipv6: ipv6.map(|ip| ip.to_string()),
                            status_message: format!("Error: {}", e),
                            is_running: false,
                        },
                    );
                }
            }
        }
    }
    
/// Try each URL in order until one succeeds.
    async fn resolve_first(
        &self,
        urls: &[String],
        version: IpVersion,
    ) -> Result<std::net::IpAddr, ResolverError> {
        let mut last_err = ResolverError::NoLocalAddress;
        for url in urls {
            match resolve_ip(&self.client, url, version).await {
                Ok(ip) => return Ok(ip),
                Err(e) => {
                    last_err = e;
                }
            }
        }
        Err(last_err)
    }
}

/// Config snapshot sent to the scheduler loop via watch channel.
#[derive(Clone)]
pub struct SchedulerConfig {
    pub running: bool,
    pub profiles: Vec<DdnsProfile>,
    pub interval_minutes: u32,
    pub ipv4_urls: Vec<String>,
    pub ipv6_urls: Vec<String>,
}
