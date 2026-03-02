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
use std::sync::{Arc, Mutex}; // For config and storage (std Mutex)
use tokio::sync::{mpsc, watch, Mutex as TokioMutex}; // For statuses and logs
use tauri::{Manager, Emitter, State};

// State management
struct AppState {
    storage: Arc<Mutex<StorageManager>>,
    config: Arc<Mutex<AppConfig>>,
    config_tx: Arc<Mutex<watch::Sender<SchedulerConfig>>>,
    statuses: SharedStatus,
    logs: SharedLogs,
    running: Arc<Mutex<bool>>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct FrontendProfile {
    id: String,
    name: String,
    provider_type: String,
    domain: String,
    enabled: bool,
    enable_ipv4: bool,
    enable_ipv6: bool,
    provider_config: serde_json::Value,
    // Status fields
    ipv4: String,
    ipv6: String,
    last_update: String,
    status_message: String,
}

fn build_scheduler_config(config: &AppConfig, storage: &StorageManager, running: bool) -> SchedulerConfig {
    let mut profiles = config.profiles.clone();
    storage.inject_tokens_into_profiles(&mut profiles);

    SchedulerConfig {
        running,
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
) {
    let providers = build_providers();
    let scheduler = DdnsScheduler::new(statuses, logs, event_tx);

    loop {
        let cfg = config_rx.borrow().clone();
        if cfg.running {
            scheduler
                .run_once(&cfg.profiles, &providers, &cfg.ipv4_urls, &cfg.ipv6_urls)
                .await;
        }

        let interval = std::time::Duration::from_secs(cfg.interval_minutes as u64 * 60);
        tokio::select! {
            _ = tokio::time::sleep(interval) => {},
            _ = config_rx.changed() => {},
        }
    }
}

// --- Commands ---

#[tauri::command]
async fn get_profiles(state: State<'_, AppState>) -> Result<Vec<FrontendProfile>, String> {
    let profiles_list = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config.profiles.clone()
    };
    
    let statuses = state.statuses.lock().await;
    
    let result = profiles_list.into_iter().map(|p| {
        let status = statuses.get(&p.id);
        let (ipv4, ipv6, last_update, status_message) = match status {
            Some(st) => (
                st.current_ipv4.clone().unwrap_or_default(),
                st.current_ipv6.clone().unwrap_or_default(),
                st.last_update.map(|t: chrono::DateTime<chrono::Utc>| t.format("%H:%M:%S").to_string()).unwrap_or_default(),
                st.status_message.clone(),
            ),
            None => (String::new(), String::new(), String::new(), String::from("Unknown")),
        };

        FrontendProfile {
            id: p.id.clone(),
            name: p.name.clone(),
            provider_type: p.provider_type.clone(),
            domain: p.domain.clone(),
            enabled: p.enabled,
            enable_ipv4: p.enable_ipv4,
            enable_ipv6: p.enable_ipv6,
            provider_config: p.provider_config.clone(),
            ipv4,
            ipv6,
            last_update,
            status_message,
        }
    }).collect();
    
    Ok(result)
}

#[tauri::command]
async fn save_profile(state: State<'_, AppState>, profile: DdnsProfile) -> Result<(), String> {
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        
        // Check if update or create
        if let Some(p) = config.profiles.iter_mut().find(|p| p.id == profile.id) {
            *p = profile;
        } else {
            config.profiles.push(profile);
        }
        
        storage.save_config(&config).map_err(|e| e.to_string())?;
    }
    
    {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        let running = *state.running.lock().map_err(|e| e.to_string())?;
        let sched_config = build_scheduler_config(&config, &storage, running);
        state.config_tx.lock().map_err(|e| e.to_string())?.send(sched_config).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
async fn delete_profile(state: State<'_, AppState>, id: String) -> Result<(), String> {
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        
        config.profiles.retain(|p| p.id != id);
        storage.save_config(&config).map_err(|e| e.to_string())?;
    }
    
    state.statuses.lock().await.remove(&id);
    
    {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        let running = *state.running.lock().map_err(|e| e.to_string())?;
        
        let sched_config = build_scheduler_config(&config, &storage, running);
        state.config_tx.lock().map_err(|e| e.to_string())?.send(sched_config).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
async fn toggle_profile(state: State<'_, AppState>, id: String, enabled: bool) -> Result<(), String> {
    let mut updated = false;
    {
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        
        if let Some(p) = config.profiles.iter_mut().find(|p| p.id == id) {
            p.enabled = enabled;
            storage.save_config(&config).map_err(|e| e.to_string())?;
            updated = true;
        }
    }
    
    if updated {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        let running = *state.running.lock().map_err(|e| e.to_string())?;
        let sched_config = build_scheduler_config(&config, &storage, running);
        state.config_tx.lock().map_err(|e| e.to_string())?.send(sched_config).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
async fn get_logs(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let logs = state.logs.lock().await;
    Ok(logs.clone())
}

#[tauri::command]
async fn get_global_running(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(*state.running.lock().map_err(|e| e.to_string())?)
}

#[tauri::command]
async fn toggle_global_running(state: State<'_, AppState>, running: bool) -> Result<(), String> {
    {
        let mut r = state.running.lock().map_err(|e| e.to_string())?;
        *r = running;
    }
    
    {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        
        let sched_config = build_scheduler_config(&config, &storage, running);
        state.config_tx.lock().map_err(|e| e.to_string())?.send(sched_config).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
async fn trigger_update(state: State<'_, AppState>) -> Result<(), String> {
    // We don't really use sched_config but we need to notify
    let (config, storage, running) = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        let running = *state.running.lock().map_err(|e| e.to_string())?;
        (config.clone(), storage, running) // config is cloned, storage is Arc? No StorageManager is struct.
        // But we don't need to return them if we build sched_config here.
    };
    
    // Actually we can just:
    {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        let running = *state.running.lock().map_err(|e| e.to_string())?;
        let _sched_config = build_scheduler_config(&config, &storage, running);
        // Just send modify to notify
        state.config_tx.lock().map_err(|e| e.to_string())?.send_modify(|_| {}); 
    }
    
    Ok(())
}

// Helper for startup
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .setup(|app| {
            let storage = StorageManager::new().expect("Failed to initialize storage");
            let mut config = storage.load_config().unwrap_or_default();
            if prefer_interface_ipv6_sources(&mut config) {
                let _ = storage.save_config(&config);
            }

            // Use Tokio Mutex for statuses and logs
            let statuses: SharedStatus = Arc::new(TokioMutex::new(HashMap::new()));
            let logs: SharedLogs = Arc::new(TokioMutex::new(Vec::new()));
            let (event_tx, mut event_rx) = mpsc::unbounded_channel();
            
            let running = true; // Default running
            let sched_config = build_scheduler_config(&config, &storage, running);
            let (config_tx, config_rx) = watch::channel(sched_config);
            
            let app_state = AppState {
                storage: Arc::new(Mutex::new(storage)),
                config: Arc::new(Mutex::new(config)),
                config_tx: Arc::new(Mutex::new(config_tx)),
                statuses: statuses.clone(),
                logs: logs.clone(),
                running: Arc::new(Mutex::new(running)),
            };
            
            app.manage(app_state);
            
            // Spawn scheduler
            tauri::async_runtime::spawn(async move {
                spawn_scheduler(statuses, logs, config_rx, Some(event_tx)).await;
            });
            
            // Spawn event listener
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    match event {
                        SchedulerEvent::Log(msg) => {
                            let _ = app_handle.emit("log", msg);
                        }
                        SchedulerEvent::StatusUpdate(_) => {
                            let _ = app_handle.emit("status_update", ());
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_profiles,
            save_profile,
            delete_profile,
            toggle_profile,
            get_logs,
            get_global_running,
            toggle_global_running,
            trigger_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
