#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

slint::include_modules!();

use simpleddns_core::models::{DdnsProfile, SchedulerEvent};
use simpleddns_core::provider::DdnsProvider;
use simpleddns_core::scheduler::{DdnsScheduler, SchedulerConfig, SharedLogs, SharedStatus};
use simpleddns_providers::aliyun::AliyunProvider;
use simpleddns_providers::cloudflare::CloudflareProvider;
use simpleddns_providers::dnspod::DnspodProvider;
use simpleddns_providers::generic::GenericHttpProvider;
use simpleddns_providers::godaddy::GodaddyProvider;
use simpleddns_providers::namecheap::NamecheapProvider;
use simpleddns_storage::config::{AppConfig, StorageManager};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, watch, Mutex};
use slint::{SharedString, VecModel};
use tracing_subscriber::EnvFilter;

fn is_unicast_global(ip: &std::net::Ipv6Addr) -> bool {
    !ip.is_loopback() && !ip.is_multicast() && (ip.segments()[0] & 0xffc0) != 0xfe80
}

fn pick_default_ipv6_interface() -> Option<String> {
    let addrs = get_if_addrs::get_if_addrs().ok()?;
    let mut names: Vec<String> = addrs
        .into_iter()
        .filter_map(|iface| match iface.addr.ip() {
            std::net::IpAddr::V6(ip) if !ip.is_loopback() && is_unicast_global(&ip) => {
                Some(iface.name)
            }
            _ => None,
        })
        .collect();
    names.sort();
    names.dedup();
    names.into_iter().next()
}

fn prefer_interface_ipv6_sources(config: &mut AppConfig) -> bool {
    let default_remote = vec![
        "https://v6.ident.me".to_string(),
        "https://api64.ipify.org".to_string(),
    ];
    if config.default_ipv6_urls.is_empty() || config.default_ipv6_urls == default_remote {
        if let Some(name) = pick_default_ipv6_interface() {
            config.default_ipv6_urls = vec![format!("interface://{}", name)];
            return true;
        }
    }
    false
}

fn build_scheduler_config(config: &AppConfig, storage: &StorageManager) -> SchedulerConfig {
    let mut profiles = config.profiles.clone();
    storage.inject_tokens_into_profiles(&mut profiles);

    SchedulerConfig {
        running: true,
        profiles,
        interval_minutes: config.check_interval_minutes,
        ipv4_urls: config.default_ipv4_urls.clone(),
        ipv6_urls: config.default_ipv6_urls.clone(),
    }
}

fn build_providers() -> HashMap<String, Box<dyn DdnsProvider>> {
    let mut map: HashMap<String, Box<dyn DdnsProvider>> = HashMap::new();
    map.insert("aliyun".to_string(), Box::new(AliyunProvider::new()));
    map.insert("cloudflare".to_string(), Box::new(CloudflareProvider::new()));
    map.insert("dnspod".to_string(), Box::new(DnspodProvider::new()));
    map.insert("generic".to_string(), Box::new(GenericHttpProvider::new()));
    map.insert("godaddy".to_string(), Box::new(GodaddyProvider::new()));
    map.insert("namecheap".to_string(), Box::new(NamecheapProvider::new()));
    map
}

async fn spawn_scheduler(
    statuses: SharedStatus,
    logs: SharedLogs,
    mut config_rx: watch::Receiver<SchedulerConfig>,
    event_tx: Option<mpsc::UnboundedSender<SchedulerEvent>>,
) -> watch::Sender<bool> {
    let (stop_tx, mut stop_rx) = watch::channel(false);
    let providers = build_providers();

    tokio::spawn(async move {
        let scheduler = DdnsScheduler::new(statuses, logs, event_tx);
        loop {
            if *stop_rx.borrow() {
                break;
            }

            let cfg = config_rx.borrow().clone();
            if cfg.running {
                scheduler
                    .run_once(&cfg.profiles, &providers, &cfg.ipv4_urls, &cfg.ipv6_urls)
                    .await;
            }

            let interval = std::time::Duration::from_secs(cfg.interval_minutes as u64 * 60);
            tokio::select! {
                _ = tokio::time::sleep(interval) => {},
                _ = stop_rx.changed() => {},
                _ = config_rx.changed() => {},
            }
        }
    });

    stop_tx
}

// Helper to construct provider_config JSON from UI inputs
fn build_provider_config(provider: &str, k1: &str, k2: &str, domain: &str, subdomain: &str) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    
    // Common fields
    map.insert("zone_name".to_string(), serde_json::Value::String(domain.to_string()));
    map.insert("sub_domain".to_string(), serde_json::Value::String(subdomain.to_string()));

    match provider {
        "aliyun" => {
            map.insert("access_key_id".to_string(), serde_json::Value::String(k1.to_string()));
            map.insert("access_key_secret".to_string(), serde_json::Value::String(k2.to_string()));
        }
        "cloudflare" => {
            map.insert("token".to_string(), serde_json::Value::String(k1.to_string()));
            // zone_id is optional, maybe support later or put in k2 if needed, but for now stick to simple
        }
        "dnspod" => {
             // login_token format: "id,token"
             // UI provides id in k1, token in k2
             let token_val = format!("{},{}", k1, k2);
             map.insert("login_token".to_string(), serde_json::Value::String(token_val));
        }
        "godaddy" => {
            map.insert("key".to_string(), serde_json::Value::String(k1.to_string()));
            map.insert("secret".to_string(), serde_json::Value::String(k2.to_string()));
        }
        "namecheap" => {
            map.insert("api_user".to_string(), serde_json::Value::String(k1.to_string()));
            map.insert("api_key".to_string(), serde_json::Value::String(k2.to_string()));
            // namecheap needs client_ip, user_name. We might need more inputs or auto-detect.
            // For now assume user manually edits config for advanced stuff if UI is simple.
            // Or add defaults.
        }
        _ => {
            // generic, etc.
        }
    }
    serde_json::Value::Object(map)
}

// Helper to extract UI inputs from provider_config JSON
fn extract_provider_config(provider: &str, config: &serde_json::Value) -> (String, String) {
    match provider {
        "aliyun" => (
            config.get("access_key_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            config.get("access_key_secret").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        ),
        "cloudflare" => (
            config.get("token").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            "".to_string(),
        ),
        "dnspod" => {
            let token = config.get("login_token").and_then(|v| v.as_str()).unwrap_or("");
            if let Some((id, secret)) = token.split_once(',') {
                (id.to_string(), secret.to_string())
            } else {
                (token.to_string(), "".to_string())
            }
        },
        "godaddy" => (
            config.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            config.get("secret").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        ),
        "namecheap" => (
            config.get("api_user").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            config.get("api_key").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        ),
        _ => ("".to_string(), "".to_string()),
    }
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = Runtime::new().expect("Failed to create tokio runtime");

    let storage = StorageManager::new().expect("Failed to initialize storage");
    let mut config = storage.load_config().unwrap_or_default();
    if prefer_interface_ipv6_sources(&mut config) {
        let _ = storage.save_config(&config);
    }

    let statuses: SharedStatus = Arc::new(Mutex::new(HashMap::new()));
    let logs: SharedLogs = Arc::new(Mutex::new(Vec::new()));
    
    // Create event channel
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();

    let sched_config = build_scheduler_config(&config, &storage);
    let (config_tx, config_rx) = watch::channel(sched_config);

    let statuses_clone = statuses.clone();
    let logs_clone = logs.clone();
    
    // Pass event_tx to scheduler
    let stop_tx = rt.block_on(async {
        spawn_scheduler(statuses_clone, logs_clone, config_rx, Some(event_tx)).await
    });

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--headless" || arg == "-d") {
        #[cfg(windows)]
        {
            use windows::Win32::System::Console::{
                AllocConsole, AttachConsole, ATTACH_PARENT_PROCESS,
            };
            unsafe {
                if AttachConsole(ATTACH_PARENT_PROCESS).is_err() {
                    let _ = AllocConsole();
                }
            }
        }

        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
            .init();

        tracing::info!("Starting in headless mode...");
        tracing::info!("Press Ctrl+C to stop.");

        rt.block_on(async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        });

        tracing::info!("Shutting down...");
        let _ = stop_tx.send(true);
        Ok(())
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
            .init();

        let ui = MainWindow::new()?;
        let ui_handle = ui.as_weak();

        // Safe clones for callbacks
        let config_arc = Arc::new(std::sync::Mutex::new(config.clone()));
        let storage_arc = Arc::new(std::sync::Mutex::new(storage));
        let config_tx_arc = Arc::new(std::sync::Mutex::new(config_tx));

        let notify_scheduler = {
            let config_arc = config_arc.clone();
            let storage = storage_arc.clone();
            let config_tx = config_tx_arc.clone();
            let ui_handle = ui_handle.clone();
            move || {
                if let Some(ui) = ui_handle.upgrade() {
                    let mut cfg = config_arc.lock().unwrap().clone();
                    storage.lock().unwrap().inject_tokens_into_profiles(&mut cfg.profiles);
                    let global_running = ui.get_global_running();
                    let sched_cfg = SchedulerConfig {
                        running: global_running,
                        profiles: cfg.profiles,
                        interval_minutes: cfg.check_interval_minutes,
                        ipv4_urls: cfg.default_ipv4_urls.clone(),
                        ipv6_urls: cfg.default_ipv6_urls.clone(),
                    };
                    let _ = config_tx.lock().unwrap().send(sched_cfg);
                }
            }
        };

        {
            let notify_scheduler = notify_scheduler.clone();
            let ui_handle_cb = ui_handle.clone();
            ui.on_toggle_global_running(move || {
                if let Some(ui) = ui_handle_cb.upgrade() {
                    let new_state = !ui.get_global_running();
                    ui.set_global_running(new_state);
                    notify_scheduler();
                }
            });
        }

        {
            let notify_scheduler = notify_scheduler.clone();
            let config_arc = config_arc.clone();
            let storage = storage_arc.clone();
            ui.on_toggle_profile(move |id, enabled| {
                let id_str = id.to_string();
                let mut cfg = config_arc.lock().unwrap();
                let mut changed = false;
                for p in &mut cfg.profiles {
                    if p.id == id_str {
                        p.enabled = enabled;
                        changed = true;
                        break;
                    }
                }
                if changed {
                    let _ = storage.lock().unwrap().save_config(&cfg);
                    notify_scheduler();
                }
            });
        }

        {
            let notify_scheduler = notify_scheduler.clone();
            ui.on_trigger_update(move || {
                notify_scheduler();
            });
        }

        // Delete Profile
        {
            let notify_scheduler = notify_scheduler.clone();
            let config_arc = config_arc.clone();
            let storage = storage_arc.clone();
            let statuses = statuses.clone();
            ui.on_delete_profile(move |id| {
                 let id_str = id.to_string();
                 let mut cfg = config_arc.lock().unwrap();
                 let len_before = cfg.profiles.len();
                 cfg.profiles.retain(|p| p.id != id_str);
                 
                 if cfg.profiles.len() < len_before {
                     let _ = storage.lock().unwrap().save_config(&cfg);
                     // Also remove status
                     if let Ok(mut s) = statuses.try_lock() {
                         s.remove(&id_str);
                     }
                     notify_scheduler();
                 }
            });
        }

        // Edit Profile Request (Load data into dialog)
        {
            let config_arc = config_arc.clone();
            let ui_handle = ui_handle.clone();
            ui.on_edit_profile_request(move |id| {
                let id_str = id.to_string();
                let cfg = config_arc.lock().unwrap();
                if let Some(p) = cfg.profiles.iter().find(|p| p.id == id_str) {
                    if let Some(ui) = ui_handle.upgrade() {
                        let (k1, k2) = extract_provider_config(&p.provider_type, &p.provider_config);
                        ui.set_d_id(SharedString::from(p.id.clone()));
                        ui.set_d_name(SharedString::from(p.name.clone()));
                        ui.set_d_provider(SharedString::from(p.provider_type.clone()));
                        ui.set_d_domain(SharedString::from(p.domain.clone()));
                        // Try to extract sub_domain from provider_config, fallback to parsing domain if missing
                        let sub_domain = p.provider_config.get("sub_domain").and_then(|v| v.as_str()).unwrap_or("");
                        ui.set_d_subdomain(SharedString::from(sub_domain));
                        
                        ui.set_d_key1(SharedString::from(k1));
                        ui.set_d_key2(SharedString::from(k2));
                        ui.set_d_v4(p.enable_ipv4);
                        ui.set_d_v6(p.enable_ipv6);
                        ui.set_show_dialog(true);
                    }
                }
            });
        }

        // Save Profile (Add or Update)
        {
            let notify_scheduler = notify_scheduler.clone();
            let config_arc = config_arc.clone();
            let storage = storage_arc.clone();
            ui.on_save_profile(move |id, name, provider, domain, subdomain, k1, k2, v4, v6| {
                let mut cfg = config_arc.lock().unwrap();
                let id_str = id.to_string();
                let provider_config = build_provider_config(&provider, &k1, &k2, &domain, &subdomain);
                
                if id_str.is_empty() {
                    // Create new
                    let new_profile = DdnsProfile {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: name.to_string(),
                        provider_type: provider.to_string(),
                        domain: domain.to_string(),
                        enable_ipv4: v4,
                        enable_ipv6: v6,
                        enabled: true,
                        provider_config,
                    };
                    cfg.profiles.push(new_profile);
                } else {
                    // Update existing
                    if let Some(p) = cfg.profiles.iter_mut().find(|p| p.id == id_str) {
                        p.name = name.to_string();
                        p.provider_type = provider.to_string();
                        p.domain = domain.to_string();
                        p.enable_ipv4 = v4;
                        p.enable_ipv6 = v6;
                        p.provider_config = provider_config;
                    }
                }
                
                let _ = storage.lock().unwrap().save_config(&cfg);
                notify_scheduler();
            });
        }

        // Event handler loop
        {
            let ui_handle = ui_handle.clone();
            let config_arc = config_arc.clone();
            let statuses = statuses.clone();
            
            // Initial load of profiles
            if let Some(ui) = ui_handle.upgrade() {
                let config = config_arc.lock().unwrap();
                let mut profile_data_list = Vec::new();
                if let Ok(s) = statuses.try_lock() {
                    for profile in &config.profiles {
                        let status = s.get(&profile.id);
                        let (ipv4, ipv6, last_update, status_message) = match status {
                            Some(st) => (
                                st.current_ipv4.clone().unwrap_or_default(),
                                st.current_ipv6.clone().unwrap_or_default(),
                                st.last_update.map(|t| t.format("%H:%M:%S").to_string()).unwrap_or_default(),
                                st.status_message.clone(),
                            ),
                            None => (String::new(), String::new(), String::new(), String::from("Unknown")),
                        };
                        profile_data_list.push(ProfileData {
                            id: SharedString::from(profile.id.clone()),
                            name: SharedString::from(profile.name.clone()),
                            domain: SharedString::from(profile.domain.clone()),
                            ipv4: SharedString::from(ipv4),
                            ipv6: SharedString::from(ipv6),
                            last_update: SharedString::from(last_update),
                            enabled: profile.enabled,
                            status_message: SharedString::from(status_message),
                        });
                    }
                }
                let model = std::rc::Rc::new(VecModel::from(profile_data_list));
                ui.set_profiles(model.into());
            }

            rt.spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    let ui_handle = ui_handle.clone();
                    let config_arc = config_arc.clone();
                    let statuses = statuses.clone();
                    
                    slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_handle.upgrade() {
                            match event {
                                SchedulerEvent::Log(msg) => {
                                    let current = ui.get_logs_text();
                                    ui.set_logs_text(current + "\n" + &msg);
                                }
                                SchedulerEvent::StatusUpdate(_) => {
                                    // Refresh profile list
                                    let config = config_arc.lock().unwrap();
                                    let mut profile_data_list = Vec::new();
                                    
                                    // Lock statuses to get latest data
                                    if let Ok(s) = statuses.try_lock() {
                                        for profile in &config.profiles {
                                            let status = s.get(&profile.id);
                                            let (ipv4, ipv6, last_update, status_message) = match status {
                                                Some(st) => (
                                                    st.current_ipv4.clone().unwrap_or_default(),
                                                    st.current_ipv6.clone().unwrap_or_default(),
                                                    st.last_update.map(|t| t.format("%H:%M:%S").to_string()).unwrap_or_default(),
                                                    st.status_message.clone(),
                                                ),
                                                None => (String::new(), String::new(), String::new(), String::from("Unknown")),
                                            };
                                            profile_data_list.push(ProfileData {
                                                id: SharedString::from(profile.id.clone()),
                                                name: SharedString::from(profile.name.clone()),
                                                domain: SharedString::from(profile.domain.clone()),
                                                ipv4: SharedString::from(ipv4),
                                                ipv6: SharedString::from(ipv6),
                                                last_update: SharedString::from(last_update),
                                                enabled: profile.enabled,
                                                status_message: SharedString::from(status_message),
                                            });
                                        }
                                    }
                                    let model = std::rc::Rc::new(VecModel::from(profile_data_list));
                                    ui.set_profiles(model.into());
                                }
                            }
                        }
                    }).unwrap();
                }
            });
        }

        ui.run()?;
        let _ = stop_tx.send(true);
        Ok(())
    }
}
