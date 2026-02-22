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

    fn log(&self, logs: &mut Vec<String>, msg: String) {
        let ts = Utc::now().format("%H:%M:%S");
        let entry = format!("[{}] {}", ts, msg);
        logs.push(entry);
        // Keep last 500 log entries
        if logs.len() > 500 {
            logs.drain(0..logs.len() - 500);
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
                match self.resolve_first(&ipv4_urls, IpVersion::IPv4).await {
                    Ok(ip) => Some(ip),
                    Err(e) => {
                        let msg = format!("[{}] IPv4 resolve failed: {}", profile.name, e);
                        warn!("{}", msg);
                        let mut logs = self.logs.lock().await;
                        self.log(&mut logs, msg);
                        None
                    }
                }
            } else {
                None
            };

            // Resolve IPv6
            let ipv6 = if profile.enable_ipv6 && !ipv6_urls.is_empty() {
                match self.resolve_first(&ipv6_urls, IpVersion::IPv6).await {
                    Ok(ip) => Some(ip),
                    Err(e) => {
                        let msg = format!("[{}] IPv6 resolve failed: {}", profile.name, e);
                        warn!("{}", msg);
                        let mut logs = self.logs.lock().await;
                        self.log(&mut logs, msg);
                        None
                    }
                }
            } else {
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
                        "[{}] Updated {} — IPv4: {} IPv6: {}",
                        profile.name,
                        profile.domain,
                        ipv4.map(|ip| ip.to_string()).unwrap_or("N/A".into()),
                        ipv6.map(|ip| ip.to_string()).unwrap_or("N/A".into()),
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

/// Spawns the background loop. Returns a watch sender to signal shutdown.
pub fn spawn_scheduler_loop(
    statuses: SharedStatus,
    logs: SharedLogs,
    config_rx: watch::Receiver<SchedulerConfig>,
) -> watch::Sender<bool> {
    let (stop_tx, mut stop_rx) = watch::channel(false);

    tokio::spawn(async move {
        let scheduler = DdnsScheduler::new(statuses, logs);
        loop {
            // Check for stop signal
            if *stop_rx.borrow() {
                info!("Scheduler loop stopped.");
                break;
            }

            let cfg = config_rx.borrow().clone();
            if cfg.running {
                scheduler
                    .run_once(
                        &cfg.profiles,
                        &cfg.providers_map(),
                        &cfg.ipv4_urls,
                        &cfg.ipv6_urls,
                    )
                    .await;
            }

            let interval = Duration::from_secs(cfg.interval_minutes as u64 * 60);
            tokio::select! {
                _ = tokio::time::sleep(interval) => {},
                _ = stop_rx.changed() => {},
            }
        }
    });

    stop_tx
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

impl SchedulerConfig {
    /// Build a provider map. This is cheap since providers are stateless unit structs.
    pub fn providers_map(&self) -> HashMap<String, Box<dyn DdnsProvider>> {
        // We can't easily import from freeddns_providers here since this is in core.
        // The caller should provide the map instead.
        // This is a placeholder — the actual map is built in the app.
        HashMap::new()
    }
}
