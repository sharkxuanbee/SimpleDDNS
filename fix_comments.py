
import os

replacements = [
    ('// Default GET                 client.get', '// Default GET\n                client.get'),
    ('鈥?', '-'),
    ('IPv4: {} \nIPv6: {}', 'IPv4: {} IPv6: {}'),
    ('// Intentionally export empty secrets to prevent leakage in plain text.         let secrets', '// Intentionally export empty secrets to prevent leakage in plain text.\n        let secrets'),
    ('// --- Cloudflare API response types ---  #[derive', '// --- Cloudflare API response types ---\n#[derive'),
    ('// Build custom headers         let mut headers', '// Build custom headers\n        let mut headers'),
    ('// --- Helper functions ---  async fn', '// --- Helper functions ---\nasync fn'),
    ('// --- Trait implementation ---  #[async_trait]', '// --- Trait implementation ---\n#[async_trait]'),
    ('// Determine zone name: explicit config or extract from domain (last 2 parts)         let zone_name', '// Determine zone name: explicit config or extract from domain (last 2 parts)\n        let zone_name'),
    ('// Update A record (IPv4)         if let Some(ip)', '// Update A record (IPv4)\n        if let Some(ip)'),
    ('// Update AAAA record (IPv6)         if let Some(ip)', '// Update AAAA record (IPv6)\n        if let Some(ip)'),
    ('ttl: 1, // Auto         proxied: false,', 'ttl: 1, // Auto\n        proxied: false,'),
    ('// Filter out link-local addresses (fe80::/10) as they are not routable globally                     // and usually not what users want for DDNS                     if is_unicast_global(&ip)', '// Filter out link-local addresses (fe80::/10) as they are not routable globally\n                    // and usually not what users want for DDNS\n                    if is_unicast_global(&ip)'),
    ('// Basic check: not loopback, not multicast, not link-local     !ip.is_loopback()', '// Basic check: not loopback, not multicast, not link-local\n    !ip.is_loopback()'),
    ('// Use a UDP socket trick to find the default outbound address     // This doesn\'t actually send anything over the network     match version', '// Use a UDP socket trick to find the default outbound address\n    // This doesn\'t actually send anything over the network\n    match version'),
    ('// Compare names case-insensitively just in case Windows GUIDs differ in casing         if iface.name', '// Compare names case-insensitively just in case Windows GUIDs differ in casing\n        if iface.name'),
    ('// Install Noto Sans SC for Chinese support     fonts.font_data.insert', '// Install Noto Sans SC for Chinese support\n    fonts.font_data.insert'),
    ('// Put NotoSansSC as the highest priority for proportional fonts (UI text)     fonts', '// Put NotoSansSC as the highest priority for proportional fonts (UI text)\n    fonts'),
    ('// Also for monospace fonts (Logs)     fonts', '// Also for monospace fonts (Logs)\n    fonts'),
    ('// Check for headless mode     let args', '// Check for headless mode\n    let args'),
    ('// Try to attach to parent console (e.g., cmd/powershell)                 // If fails (e.g., launched from explorer), allocate a new console                 if AttachConsole(ATTACH_PARENT_PROCESS).is_err()', '// Try to attach to parent console (e.g., cmd/powershell)\n                // If fails (e.g., launched from explorer), allocate a new console\n                if AttachConsole(ATTACH_PARENT_PROCESS).is_err()'),
    ('// Keep the main thread alive and wait for shutdown signal         rt.block_on', '// Keep the main thread alive and wait for shutdown signal\n        rt.block_on'),
    ('// Inject tokens from encrypted secrets into provider_config for each profile if missing     let mut profiles', '// Inject tokens from encrypted secrets into provider_config for each profile if missing\n    let mut profiles'),
    ('token: String,           // Cloudflare token (encrypted in secrets.enc)     generic_url: String,', 'token: String,           // Cloudflare token (encrypted in secrets.enc)\n    generic_url: String,'),
    ('generic_method: String,  // GET or POST     generic_body: String,    // POST body template     generic_headers: String, // JSON string of headers     save_error: Option<String>,', 'generic_method: String,  // GET or POST\n    generic_body: String,    // POST body template\n    generic_headers: String, // JSON string of headers\n    save_error: Option<String>,'),
    ('// Shared state between the scheduler and the GUI. pub type SharedStatus', '// Shared state between the scheduler and the GUI.\npub type SharedStatus'),
    ('// Do not fallback to plaintext!                                 }', '// Do not fallback to plaintext!\n                                }'),
    ('// 鈹€鈹€鈹€ App State 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€  struct ProfileEditor {', '// App State\nstruct ProfileEditor {'),
    ('_ => {}', '_ => {}\n        }')
]

files_to_check = [
    'simpleddns-storage/src/config.rs',
    'simpleddns-providers/src/cloudflare.rs',
    'simpleddns-providers/src/generic.rs',
    'simpleddns-core/src/resolver.rs',
    'simpleddns-core/src/scheduler.rs',
    'simpleddns-app/src/main.rs'
]

for filepath in files_to_check:
    if os.path.exists(filepath):
        try:
            with open(filepath, 'r', encoding='utf-8') as f:
                content = f.read()
            
            original_content = content
            for old, new in replacements:
                content = content.replace(old, new)
            
            if content != original_content:
                with open(filepath, 'w', encoding='utf-8') as f:
                    f.write(content)
                print(f"Fixed comments in {filepath}")
            else:
                print(f"No changes in {filepath}")
        except Exception as e:
            print(f"Error fixing {filepath}: {e}")
