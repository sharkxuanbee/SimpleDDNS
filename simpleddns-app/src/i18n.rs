#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    En,
    Zh,
}

impl Default for Language {
    fn default() -> Self {
        Language::En
    }
}

impl Language {
    pub fn from_str(s: &str) -> Self {
        match s {
            "zh" => Language::Zh,
            _ => Language::En,
        }
    }
    pub fn to_str(&self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Zh => "zh",
        }
    }
}

pub struct I18n;

impl I18n {
    pub fn t<'a>(lang: Language, key: &'a str) -> &'a str {
        match lang {
            Language::En => match key {
                "tab_profiles" => "Profiles",
                "tab_settings" => "Settings",
                "btn_start" => "Start",
                "btn_stop" => "Stop",
                "btn_add_profile" => "Add Profile",
                "btn_update_now" => "Update Now",
                "no_profiles" => "No profiles configured. Click 'Add Profile' to get started.",
                "log_panel_title" => "Operation Logs",
                "btn_clear" => "Clear",
                "no_logs" => "No logs yet. Logs will appear here when profiles are updated.",
                "edit_profile" => "Edit Profile",
                "add_profile" => "Add Profile",
                "name" => "Name:",
                "domain" => "Domain:",
                "provider" => "Provider:",
                "enable_ipv4" => "Enable IPv4 (A)",
                "enable_ipv6" => "Enable IPv6 (AAAA)",
                "api_token" => "API Token:",
                "zone_name" => "Zone Name (optional):",
                "url_template" => "URL Template:",
                "method" => "Method:",
                "body_template" => "Body Template (JSON):",
                "custom_headers" => "Custom Headers (JSON):",
                "placeholders" => "Placeholders: {domain}, {ipv4}, {ipv6}",
                "btn_save" => "Save",
                "btn_cancel" => "Cancel",
                "btn_delete" => "Delete",
                "btn_edit" => "Edit",
                "settings_check_interval" => "Check Interval (minutes):",
                "settings_start_on_boot" => "Start on boot",
                "settings_ipv4_sources" => "IPv4 Probe Sources",
                "settings_ipv6_sources" => "IPv6 Probe Sources",
                "cloudflare_config" => "Cloudflare Configuration",
                "generic_http_config" => "Generic HTTP Configuration",
                "settings_interfaces" => "Network Interfaces",
                "btn_add" => "Add",
                "language" => "Language:",
                "backup_restore" => "Backup & Restore",
                "btn_export" => "Export Config",
                "btn_import" => "Import Config",
                "lang_en" => "English",
                "lang_zh" => "Simplified Chinese",
                _ => key,
            },
            Language::Zh => match key {
                "tab_profiles" => "配置列表",
                "tab_settings" => "软件设置",
                "btn_start" => "启动服务",
                "btn_stop" => "停止运行",
                "btn_add_profile" => "添加配置",
                "btn_update_now" => "立即更新",
                "no_profiles" => "暂无配置资料。点击「添加配置」开始使用。",
                "log_panel_title" => "操作日志",
                "btn_clear" => "清空",
                "no_logs" => "暂无日志，配置更新时日志会显示在这里。",
                "edit_profile" => "编辑配置",
                "add_profile" => "添加配置",
                "name" => "配置名称:",
                "domain" => "域名:",
                "provider" => "服务商:",
                "enable_ipv4" => "启用 IPv4 (A 记录)",
                "enable_ipv6" => "启用 IPv6 (AAAA 记录)",
                "api_token" => "API Token:",
                "zone_name" => "Zone名称 (可选):",
                "url_template" => "URL 模板:",
                "method" => "请求方法:",
                "body_template" => "请求体 (JSON模板):",
                "custom_headers" => "自定义 Headers (JSON):",
                "placeholders" => "可用变量: {domain}, {ipv4}, {ipv6}",
                "btn_save" => "保存",
                "btn_cancel" => "取消",
                "btn_delete" => "删除",
                "btn_edit" => "编辑",
                "settings_check_interval" => "检查间隔 (分钟):",
                "settings_start_on_boot" => "开机自启",
                "settings_ipv4_sources" => "IPv4 获取接口",
                "settings_ipv6_sources" => "IPv6 获取接口",
                "cloudflare_config" => "Cloudflare 配置",
                "generic_http_config" => "通用 HTTP 配置",
                "settings_interfaces" => "通过网卡获取 (Network Interfaces)",
                "btn_add" => "添加",
                "language" => "界面语言:",
                "backup_restore" => "备份与恢复",
                "btn_export" => "导出配置",
                "btn_import" => "导入配置",
                "lang_en" => "English",
                "lang_zh" => "简体中文",
                _ => key,
            },
        }
    }
}
