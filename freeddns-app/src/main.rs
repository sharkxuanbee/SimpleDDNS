#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod i18n;

use eframe::egui;
use freeddns_core::models::DdnsProfile;
use freeddns_core::provider::DdnsProvider;
use freeddns_core::scheduler::{DdnsScheduler, SchedulerConfig, SharedLogs, SharedStatus};
use freeddns_providers::cloudflare::CloudflareProvider;
use freeddns_providers::generic::GenericHttpProvider;
use freeddns_storage::config::{AppConfig, FullExport, StorageManager};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::{watch, Mutex};
use tracing_subscriber::EnvFilter;

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Install Noto Sans SC for Chinese support
    fonts.font_data.insert(
        "NotoSansSC".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/NotoSansSC-Regular.otf"
        ))),
    );

    // Put NotoSansSC as the highest priority for proportional fonts (UI text)
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "NotoSansSC".to_owned());

    // Also for monospace fonts (Logs)
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "NotoSansSC".to_owned());

    ctx.set_fonts(fonts);
}

fn load_icon() -> egui::IconData {
    let icon_raw = include_bytes!("../assets/icons/icon.jpg");
    let image = image::load_from_memory(icon_raw).expect("Failed to open icon");
    let image = image.to_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    egui::IconData {
        rgba,
        width,
        height,
    }
}

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

fn main() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let rt = Runtime::new().expect("Failed to create tokio runtime");

    let storage = StorageManager::new().expect("Failed to initialize storage");
    let mut config = storage.load_config().unwrap_or_default();
    if prefer_interface_ipv6_sources(&mut config) {
        let _ = storage.save_config(&config);
    }

    // Shared state
    let statuses: SharedStatus = Arc::new(Mutex::new(HashMap::new()));
    let logs: SharedLogs = Arc::new(Mutex::new(Vec::new()));

    // Build scheduler config
    let sched_config = build_scheduler_config(&config, &storage);
    let (config_tx, config_rx) = watch::channel(sched_config);

    // Spawn scheduler loop in background
    let statuses_clone = statuses.clone();
    let logs_clone = logs.clone();
    let stop_tx =
        rt.block_on(async { spawn_scheduler(statuses_clone, logs_clone, config_rx).await });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([850.0, 600.0])
            .with_min_inner_size([650.0, 450.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "SimpleDDNS",
        options,
        Box::new(move |cc| {
            setup_custom_fonts(&cc.egui_ctx);
            Ok(Box::new(DdnsApp::new(
                storage, config, statuses, logs, config_tx, stop_tx, rt,
            )))
        }),
    )
}

fn build_scheduler_config(config: &AppConfig, storage: &StorageManager) -> SchedulerConfig {
    // Inject tokens from encrypted secrets into provider_config for each profile if missing
    let mut profiles = config.profiles.clone();
    for p in profiles.iter_mut() {
        if p.provider_type == "cloudflare" {
            // Only load from encrypted secrets if not already present in config
            let has_token = p.provider_config.get("api_token").is_some();
            if !has_token {
                if let Ok(token) = storage.load_secret(&p.id) {
                    if let Some(obj) = p.provider_config.as_object_mut() {
                        obj.insert("api_token".to_string(), serde_json::Value::String(token));
                    }
                }
            }
        }
    }

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
    map.insert(
        "cloudflare".to_string(),
        Box::new(CloudflareProvider::new()),
    );
    map.insert("generic".to_string(), Box::new(GenericHttpProvider::new()));
    map
}

async fn spawn_scheduler(
    statuses: SharedStatus,
    logs: SharedLogs,
    mut config_rx: watch::Receiver<SchedulerConfig>,
) -> watch::Sender<bool> {
    let (stop_tx, mut stop_rx) = watch::channel(false);
    let providers = build_providers();

    tokio::spawn(async move {
        let scheduler = DdnsScheduler::new(statuses, logs);
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

// ─── App State ───────────────────────────────────────────────

struct ProfileEditor {
    is_open: bool,
    is_new: bool,
    profile: DdnsProfile,
    token: String,           // Cloudflare token (encrypted in secrets.enc)
    generic_url: String,     // Generic HTTP URL template
    generic_method: String,  // GET or POST
    generic_body: String,    // POST body template
    generic_headers: String, // JSON string of headers
    save_error: Option<String>,
}

impl Default for ProfileEditor {
    fn default() -> Self {
        Self {
            is_open: false,
            is_new: false,
            profile: DdnsProfile::default(),
            token: String::new(),
            generic_url: String::new(),
            generic_method: "GET".to_string(),
            generic_body: String::new(),
            generic_headers: String::new(),
            save_error: None,
        }
    }
}

struct DdnsApp {
    storage: StorageManager,
    config: AppConfig,
    active_tab: usize,
    editor: ProfileEditor,
    statuses: SharedStatus,
    logs: SharedLogs,
    config_tx: watch::Sender<SchedulerConfig>,
    _stop_tx: watch::Sender<bool>,
    _rt: Runtime,
    // Cached copies for rendering (updated each frame from shared state)
    cached_statuses: HashMap<String, freeddns_core::models::ProfileStatus>,
    cached_logs: Vec<String>,
    global_running: bool,
    // Settings editor
    new_ipv4_url: String,
    new_ipv6_url: String,
}

impl DdnsApp {
    fn new(
        storage: StorageManager,
        config: AppConfig,
        statuses: SharedStatus,
        logs: SharedLogs,
        config_tx: watch::Sender<SchedulerConfig>,
        stop_tx: watch::Sender<bool>,
        rt: Runtime,
    ) -> Self {
        Self {
            storage,
            config,
            active_tab: 0,
            editor: ProfileEditor::default(),
            statuses,
            logs,
            config_tx,
            _stop_tx: stop_tx,
            _rt: rt,
            cached_statuses: HashMap::new(),
            cached_logs: Vec::new(),
            global_running: true,
            new_ipv4_url: String::new(),
            new_ipv6_url: String::new(),
        }
    }

    fn notify_scheduler(&self) {
        let sched_cfg = SchedulerConfig {
            running: self.global_running,
            profiles: {
                let mut profiles = self.config.profiles.clone();
                for p in profiles.iter_mut() {
                    if p.provider_type == "cloudflare" {
                        // Only load from encrypted secrets if not already present in config
                        let has_token = p.provider_config.get("api_token").is_some();
                        if !has_token {
                            if let Ok(token) = self.storage.load_secret(&p.id) {
                                if let Some(obj) = p.provider_config.as_object_mut() {
                                    obj.insert(
                                        "api_token".to_string(),
                                        serde_json::Value::String(token),
                                    );
                                }
                            }
                        }
                    }
                }
                profiles
            },
            interval_minutes: self.config.check_interval_minutes,
            ipv4_urls: self.config.default_ipv4_urls.clone(),
            ipv6_urls: self.config.default_ipv6_urls.clone(),
        };
        let _ = self.config_tx.send(sched_cfg);
    }

    fn trigger_immediate_update(&self) {
        // Force the scheduler to wake up and run immediately
        self.notify_scheduler();
    }

    fn sync_cached_state(&mut self) {
        // Non-blocking try_lock to read shared state
        if let Ok(s) = self.statuses.try_lock() {
            self.cached_statuses = s.clone();
        }
        if let Ok(l) = self.logs.try_lock() {
            self.cached_logs = l.clone();
        }
    }
}

impl eframe::App for DdnsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let lang = i18n::Language::from_str(&self.config.language);

        // Periodically sync cached state from async world
        self.sync_cached_state();

        // Request repaint every second for live status updates
        ctx.request_repaint_after(std::time::Duration::from_secs(1));

        egui::SidePanel::left("nav_panel")
            .min_width(120.0)
            .show(ctx, |ui| {
                ui.heading("SimpleDDNS");
                ui.add_space(10.0);

                if ui
                    .selectable_label(self.active_tab == 0, i18n::I18n::t(lang, "tab_profiles"))
                    .clicked()
                {
                    self.active_tab = 0;
                }
                if ui
                    .selectable_label(self.active_tab == 1, i18n::I18n::t(lang, "tab_settings"))
                    .clicked()
                {
                    self.active_tab = 1;
                }

                ui.add_space(20.0);
                ui.separator();

                // Global start/stop
                let label = if self.global_running {
                    i18n::I18n::t(lang, "btn_stop")
                } else {
                    i18n::I18n::t(lang, "btn_start")
                };
                if ui.button(label).clicked() {
                    self.global_running = !self.global_running;
                    self.notify_scheduler();
                }
            });

        // Bottom panel: always-visible operation logs
        egui::TopBottomPanel::bottom("log_panel")
            .min_height(100.0)
            .max_height(200.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(i18n::I18n::t(lang, "log_panel_title")).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(i18n::I18n::t(lang, "btn_clear")).clicked() {
                            if let Ok(mut l) = self.logs.try_lock() {
                                l.clear();
                            }
                            self.cached_logs.clear();
                        }
                    });
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if self.cached_logs.is_empty() {
                            ui.colored_label(egui::Color32::GRAY, i18n::I18n::t(lang, "no_logs"));
                        } else {
                            // Show the most recent log entries
                            let logs = &self.cached_logs;
                            for line in logs.iter() {
                                ui.label(egui::RichText::new(line).monospace().size(11.0));
                            }
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            0 => self.render_profiles(ui, lang),
            1 => self.render_settings(ui, lang),
            _ => {}
        });

        self.render_editor_modal(ctx);
    }
}

impl DdnsApp {
    fn render_profiles(&mut self, ui: &mut egui::Ui, lang: i18n::Language) {
        ui.horizontal(|ui| {
            ui.heading(i18n::I18n::t(lang, "tab_profiles"));
            ui.add_space(10.0);
            if ui.button(i18n::I18n::t(lang, "btn_add_profile")).clicked() {
                self.editor = ProfileEditor::default();
                self.editor.is_open = true;
                self.editor.is_new = true;
            }
            if ui.button(i18n::I18n::t(lang, "btn_update_now")).clicked() {
                self.trigger_immediate_update();
            }
        });
        ui.separator();

        if self.config.profiles.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(i18n::I18n::t(lang, "no_profiles"));
            });
            return;
        }

        let mut to_delete = None;
        let mut needs_save = false;

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (idx, profile) in self.config.profiles.iter_mut().enumerate() {
                let status = self.cached_statuses.get(&profile.id);

                ui.group(|ui| {
                    // Row 1: enable toggle, name, domain, buttons
                    ui.horizontal(|ui| {
                        let mut enabled = profile.enabled;
                        if ui.checkbox(&mut enabled, "").changed() {
                            profile.enabled = enabled;
                            needs_save = true;
                        }

                        let status_icon = match status {
                            Some(s) if s.status_message == "OK" => "🟢",
                            Some(_) => "🔴",
                            None => "⚪",
                        };
                        ui.label(status_icon);
                        ui.label(egui::RichText::new(&profile.name).strong().size(15.0));
                        ui.label(format!("({})", profile.domain));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(i18n::I18n::t(lang, "btn_delete")).clicked() {
                                to_delete = Some(idx);
                            }
                            if ui.button(i18n::I18n::t(lang, "btn_edit")).clicked() {
                                self.editor.is_open = true;
                                self.editor.is_new = false;
                                self.editor.save_error = None;
                                self.editor.profile = profile.clone();

                                // Load token: check plaintext config first, then encrypted secrets
                                let plaintext_token = profile
                                    .provider_config
                                    .get("api_token")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                if let Some(t) = plaintext_token {
                                    self.editor.token = t;
                                } else {
                                    match self.storage.load_secret(&profile.id) {
                                        Ok(t) => {
                                            self.editor.token = t;
                                        }
                                        Err(_) => {
                                            self.editor.token = String::new();
                                        }
                                    }
                                }

                                // Load generic config
                                let cfg = &profile.provider_config;
                                self.editor.generic_url = cfg
                                    .get("url")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                self.editor.generic_method = cfg
                                    .get("method")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("GET")
                                    .to_string();
                                self.editor.generic_body = cfg
                                    .get("body")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                self.editor.generic_headers = cfg
                                    .get("headers")
                                    .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
                                    .unwrap_or_default();
                            }
                        });
                    });

                    // Row 2: provider, IPv4/IPv6 toggles, status info
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} {}",
                            i18n::I18n::t(lang, "provider"),
                            profile.provider_type
                        ));
                        ui.separator();
                        let mut enable_ipv4 = profile.enable_ipv4;
                        if ui
                            .checkbox(&mut enable_ipv4, i18n::I18n::t(lang, "ipv4"))
                            .changed()
                        {
                            profile.enable_ipv4 = enable_ipv4;
                            needs_save = true;
                        }
                        let mut enable_ipv6 = profile.enable_ipv6;
                        if ui
                            .checkbox(&mut enable_ipv6, i18n::I18n::t(lang, "ipv6"))
                            .changed()
                        {
                            profile.enable_ipv6 = enable_ipv6;
                            needs_save = true;
                        }

                        if let Some(s) = status {
                            ui.separator();
                            if let Some(ref ip4) = s.current_ipv4 {
                                ui.label(format!("A: {}", ip4));
                            }
                            if let Some(ref ip6) = s.current_ipv6 {
                                ui.label(format!("AAAA: {}", ip6));
                            }
                            if let Some(ref t) = s.last_update {
                                ui.label(format!("Last: {}", t.format("%H:%M:%S")));
                            }
                        }
                    });

                    // Row 3: status message if error
                    if let Some(s) = status {
                        if s.status_message != "OK" {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 100, 100),
                                &s.status_message,
                            );
                        }
                    }
                });
                ui.add_space(4.0);
            }
        });

        if let Some(idx) = to_delete {
            let profile = self.config.profiles.remove(idx);
            let _ = self.storage.delete_secret(&profile.id);
            needs_save = true;
        }

        if needs_save {
            let _ = self.storage.save_config(&self.config);
            self.notify_scheduler();
        }
    }

    fn render_settings(&mut self, ui: &mut egui::Ui, lang: i18n::Language) {
        ui.heading(i18n::I18n::t(lang, "tab_settings"));
        ui.separator();

        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label(i18n::I18n::t(lang, "language"));
            let mut current_lang = i18n::Language::from_str(&self.config.language);
            egui::ComboBox::from_id_salt("language_select")
                .selected_text(match current_lang {
                    i18n::Language::En => i18n::I18n::t(lang, "lang_en"),
                    i18n::Language::Zh => i18n::I18n::t(lang, "lang_zh"),
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(
                            &mut current_lang,
                            i18n::Language::En,
                            i18n::I18n::t(lang, "lang_en"),
                        )
                        .changed()
                    {
                        self.config.language = current_lang.to_str().to_string();
                        changed = true;
                    }
                    if ui
                        .selectable_value(
                            &mut current_lang,
                            i18n::Language::Zh,
                            i18n::I18n::t(lang, "lang_zh"),
                        )
                        .changed()
                    {
                        self.config.language = current_lang.to_str().to_string();
                        changed = true;
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label(i18n::I18n::t(lang, "settings_check_interval"));
            if ui
                .add(egui::DragValue::new(&mut self.config.check_interval_minutes).range(1..=1440))
                .changed()
            {
                changed = true;
            }
        });

        let mut run_on_startup = self.config.run_on_startup;
        if ui
            .checkbox(
                &mut run_on_startup,
                i18n::I18n::t(lang, "settings_start_on_boot"),
            )
            .changed()
        {
            self.config.run_on_startup = run_on_startup;
            // Actually toggle auto-launch
            if let Ok(exe) = std::env::current_exe() {
                if let Ok(launcher) = auto_launch::AutoLaunchBuilder::new()
                    .set_app_name("SimpleDDNS")
                    .set_app_path(exe.to_str().unwrap_or(""))
                    .build()
                {
                    if run_on_startup {
                        let _ = launcher.enable();
                    } else {
                        let _ = launcher.disable();
                    }
                }
            }
            changed = true;
        }

        ui.add_space(10.0);
        ui.separator();
        ui.heading(i18n::I18n::t(lang, "settings_interfaces"));

        egui::ScrollArea::vertical()
            .id_salt("interfaces_scroll")
            .max_height(150.0)
            .show(ui, |ui| {
                if let Ok(addrs) = get_if_addrs::get_if_addrs() {
                    // Group by interface name
                    let mut groups: std::collections::HashMap<String, Vec<std::net::IpAddr>> =
                        std::collections::HashMap::new();
                    for iface in addrs {
                        if !iface.addr.ip().is_loopback() {
                            groups.entry(iface.name).or_default().push(iface.addr.ip());
                        }
                    }

                    // Sort keys for consistent display order
                    let mut names: Vec<_> = groups.keys().cloned().collect();
                    names.sort();

                    for name in names {
                        let ips = &groups[&name];
                        let has_v4 = ips.iter().any(|ip| ip.is_ipv4());
                        let has_v6 = ips.iter().any(|ip| ip.is_ipv6());

                        ui.horizontal(|ui| {
                            // Show name and the first IP for context
                            if let Some(first_ip) = ips.first() {
                                ui.label(format!("{} ({})", name, first_ip));
                            } else {
                                ui.label(&name);
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if has_v6 {
                                        if ui.button("+ IPv6").clicked() {
                                            let url = format!("interface://{}", name);
                                            if !self.config.default_ipv6_urls.contains(&url) {
                                                self.config.default_ipv6_urls.push(url);
                                                changed = true;
                                            }
                                        }
                                    }
                                    if has_v4 {
                                        if ui.button("+ IPv4").clicked() {
                                            let url = format!("interface://{}", name);
                                            if !self.config.default_ipv4_urls.contains(&url) {
                                                self.config.default_ipv4_urls.push(url);
                                                changed = true;
                                            }
                                        }
                                    }
                                },
                            );
                        });
                    }
                }
            });

        ui.add_space(10.0);
        ui.separator();
        ui.heading(i18n::I18n::t(lang, "settings_ipv4_sources"));
        let mut ipv4_to_remove = None;
        for (i, url) in self.config.default_ipv4_urls.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(url);
                if ui.small_button("✕").clicked() {
                    ipv4_to_remove = Some(i);
                }
            });
        }
        if let Some(i) = ipv4_to_remove {
            self.config.default_ipv4_urls.remove(i);
            changed = true;
        }
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.new_ipv4_url);
            if ui.button(i18n::I18n::t(lang, "btn_add")).clicked() && !self.new_ipv4_url.is_empty()
            {
                self.config
                    .default_ipv4_urls
                    .push(self.new_ipv4_url.clone());
                self.new_ipv4_url.clear();
                changed = true;
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.heading(i18n::I18n::t(lang, "settings_ipv6_sources"));
        let mut ipv6_to_remove = None;
        let mut ipv6_move_up = None;
        let mut ipv6_move_down = None;
        let ipv6_len = self.config.default_ipv6_urls.len();
        for (i, url) in self.config.default_ipv6_urls.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(url);
                if i > 0 && ui.small_button("↑").clicked() {
                    ipv6_move_up = Some(i);
                }
                if i + 1 < ipv6_len && ui.small_button("↓").clicked() {
                    ipv6_move_down = Some(i);
                }
                if ui.small_button("✕").clicked() {
                    ipv6_to_remove = Some(i);
                }
            });
        }
        if let Some(i) = ipv6_to_remove {
            self.config.default_ipv6_urls.remove(i);
            changed = true;
        } else if let Some(i) = ipv6_move_up {
            self.config.default_ipv6_urls.swap(i, i - 1);
            changed = true;
        } else if let Some(i) = ipv6_move_down {
            self.config.default_ipv6_urls.swap(i, i + 1);
            changed = true;
        }
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.new_ipv6_url);
            if ui.button(i18n::I18n::t(lang, "btn_add")).clicked() && !self.new_ipv6_url.is_empty()
            {
                self.config
                    .default_ipv6_urls
                    .push(self.new_ipv6_url.clone());
                self.new_ipv6_url.clear();
                changed = true;
            }
        });

        if changed {
            let _ = self.storage.save_config(&self.config);
            self.notify_scheduler();
        }

        ui.add_space(20.0);
        ui.separator();
        ui.heading(i18n::I18n::t(lang, "backup_restore"));

        ui.horizontal(|ui| {
            if ui.button(i18n::I18n::t(lang, "btn_export")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .set_file_name("freeddns-backup.json")
                    .save_file()
                {
                    if let Ok(export) = self.storage.export_full_config() {
                        if let Ok(json) = serde_json::to_string_pretty(&export) {
                            let _ = std::fs::write(path, json);
                        }
                    }
                }
            }

            if ui.button(i18n::I18n::t(lang, "btn_import")).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        if let Ok(export) = serde_json::from_str::<FullExport>(&content) {
                            if self.storage.import_full_config(export).is_ok() {
                                // Reload config
                                if let Ok(new_config) = self.storage.load_config() {
                                    self.config = new_config;
                                    self.notify_scheduler();
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    fn render_editor_modal(&mut self, ctx: &egui::Context) {
        let mut is_open = self.editor.is_open;
        if !is_open {
            return;
        }

        let lang = i18n::Language::from_str(&self.config.language);

        egui::Window::new(if self.editor.is_new {
            i18n::I18n::t(lang, "add_profile")
        } else {
            i18n::I18n::t(lang, "edit_profile")
        })
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .min_width(400.0)
        .show(ctx, |ui| {
            egui::Grid::new("profile_editor_grid")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label(i18n::I18n::t(lang, "name"));
                    ui.text_edit_singleline(&mut self.editor.profile.name);
                    ui.end_row();

                    ui.label(i18n::I18n::t(lang, "domain"));
                    ui.text_edit_singleline(&mut self.editor.profile.domain);
                    ui.end_row();

                    ui.label(i18n::I18n::t(lang, "provider"));
                    egui::ComboBox::from_id_salt("provider_type")
                        .selected_text(&self.editor.profile.provider_type)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.editor.profile.provider_type,
                                "cloudflare".to_string(),
                                "Cloudflare",
                            );
                            ui.selectable_value(
                                &mut self.editor.profile.provider_type,
                                "generic".to_string(),
                                "Generic HTTP",
                            );
                        });
                    ui.end_row();

                    ui.label("IPv4:");
                    ui.checkbox(
                        &mut self.editor.profile.enable_ipv4,
                        i18n::I18n::t(lang, "enable_ipv4"),
                    );
                    ui.end_row();

                    ui.label("IPv6:");
                    ui.checkbox(
                        &mut self.editor.profile.enable_ipv6,
                        i18n::I18n::t(lang, "enable_ipv6"),
                    );
                    ui.end_row();
                });

            ui.separator();

            // Provider-specific config
            if self.editor.profile.provider_type == "cloudflare" {
                ui.label(i18n::I18n::t(lang, "cloudflare_config"));
                egui::Grid::new("cf_config_grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(i18n::I18n::t(lang, "api_token"));
                        ui.add(egui::TextEdit::singleline(&mut self.editor.token).password(true));
                        ui.end_row();

                        let mut zone_name = self
                            .editor
                            .profile
                            .provider_config
                            .get("zone_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        ui.label(i18n::I18n::t(lang, "zone_name"));
                        if ui.text_edit_singleline(&mut zone_name).changed() {
                            if let Some(obj) = self.editor.profile.provider_config.as_object_mut() {
                                obj.insert(
                                    "zone_name".to_string(),
                                    serde_json::Value::String(zone_name),
                                );
                            }
                        }
                        ui.end_row();
                    });
            } else if self.editor.profile.provider_type == "generic" {
                ui.label(i18n::I18n::t(lang, "generic_http_config"));
                egui::Grid::new("generic_config_grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(i18n::I18n::t(lang, "url_template"));
                        ui.text_edit_singleline(&mut self.editor.generic_url);
                        ui.end_row();

                        ui.label(i18n::I18n::t(lang, "method"));
                        egui::ComboBox::from_id_salt("http_method")
                            .selected_text(&self.editor.generic_method)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.editor.generic_method,
                                    "GET".to_string(),
                                    "GET",
                                );
                                ui.selectable_value(
                                    &mut self.editor.generic_method,
                                    "POST".to_string(),
                                    "POST",
                                );
                            });
                        ui.end_row();
                    });

                if self.editor.generic_method == "POST" {
                    ui.label(i18n::I18n::t(lang, "body_template"));
                    ui.add(
                        egui::TextEdit::multiline(&mut self.editor.generic_body)
                            .desired_rows(3)
                            .code_editor(),
                    );
                }

                ui.label(i18n::I18n::t(lang, "custom_headers"));
                ui.add(
                    egui::TextEdit::multiline(&mut self.editor.generic_headers)
                        .desired_rows(3)
                        .code_editor(),
                );
                ui.label(i18n::I18n::t(lang, "placeholders"));
            }

            ui.add_space(10.0);

            if let Some(ref err) = self.editor.save_error {
                ui.colored_label(egui::Color32::RED, format!("Error saving: {}", err));
            }

            ui.horizontal(|ui| {
                if ui.button(i18n::I18n::t(lang, "btn_save")).clicked() {
                    println!("DEBUG: Save button clicked");
                    // Reset error
                    self.editor.save_error = None;

                    // Build provider_config
                    if self.editor.profile.provider_type == "cloudflare" {
                        // Save token to encrypted secrets file
                        if !self.editor.token.is_empty() {
                            match self
                                .storage
                                .save_secret(&self.editor.profile.id, &self.editor.token)
                            {
                                Ok(_) => {
                                    // Token saved to encrypted file, remove plaintext from config
                                    if let Some(obj) =
                                        self.editor.profile.provider_config.as_object_mut()
                                    {
                                        obj.remove("api_token");
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Failed to save encrypted token: {}", e);
                                    // Fallback: store in config (plaintext)
                                    if let Some(obj) =
                                        self.editor.profile.provider_config.as_object_mut()
                                    {
                                        obj.insert(
                                            "api_token".to_string(),
                                            serde_json::Value::String(self.editor.token.clone()),
                                        );
                                    }
                                }
                            }
                        }
                    } else if self.editor.profile.provider_type == "generic" {
                        let mut obj = serde_json::Map::new();
                        obj.insert(
                            "url".to_string(),
                            serde_json::Value::String(self.editor.generic_url.clone()),
                        );
                        obj.insert(
                            "method".to_string(),
                            serde_json::Value::String(self.editor.generic_method.clone()),
                        );
                        if !self.editor.generic_body.is_empty() {
                            obj.insert(
                                "body".to_string(),
                                serde_json::Value::String(self.editor.generic_body.clone()),
                            );
                        }
                        if !self.editor.generic_headers.is_empty() {
                            if let Ok(headers) = serde_json::from_str::<serde_json::Value>(
                                &self.editor.generic_headers,
                            ) {
                                obj.insert("headers".to_string(), headers);
                            }
                        }
                        self.editor.profile.provider_config = serde_json::Value::Object(obj);
                    }

                    if self.editor.is_new {
                        self.config.profiles.push(self.editor.profile.clone());
                    } else if let Some(p) = self
                        .config
                        .profiles
                        .iter_mut()
                        .find(|p| p.id == self.editor.profile.id)
                    {
                        *p = self.editor.profile.clone();
                    }
                    println!("DEBUG: Saving config to file...");
                    if let Err(e) = self.storage.save_config(&self.config) {
                        println!("DEBUG: Failed to save config: {}", e);
                        self.editor.save_error = Some(format!("Failed to save config file: {}", e));
                        return;
                    }
                    println!("DEBUG: Config saved successfully");
                    self.notify_scheduler();
                    self.editor.is_open = false;
                }
                if ui.button(i18n::I18n::t(lang, "btn_cancel")).clicked() {
                    self.editor.is_open = false;
                }
            });
        });

        if !is_open {
            self.editor.is_open = false;
        }
    }
}
