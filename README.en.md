# SimpleDDNS

[中文](README.md) | English

Cross-platform, low-resource DDNS client (Rust + egui).

Designed for regular users: configure a profile and the app will automatically sync your domain's A/AAAA records to your current public IP in the background.

Supports **Windows / macOS / Linux**.

## Features

- Multi-profile management: add / edit / delete / enable / disable.
- Providers:
  - **Cloudflare** (API Token, auto Zone ID lookup, auto find/create/update A/AAAA records).
  - **Generic HTTP** (URL template, GET/POST, custom headers, JSON body template).
- Independent IPv4/IPv6 detection:
  - Customizable IPv4 probe source list.
  - Customizable IPv6 probe source list.
  - Local network interface detection: `local://ipv4`, `local://ipv6`.
  - Failure in one protocol does not block the other.
- Update only on change: DNS API is only called when the IP actually differs.
- Background scheduling:
  - Global Start/Stop.
  - Per-profile enable/disable.
  - Configurable check interval.
  - 15-second HTTP timeout.
- Native GUI (egui):
  - Profile list with status indicators (🟢/🔴), current IPs, last update time.
  - Profile editor dialog (provider → domain → credentials → IPv4/IPv6).
  - Live scrolling log panel.
  - Global Start/Stop button.
- Settings:
  - Start on boot (cross-platform via `auto-launch`).
  - Configurable check interval.
  - IPv4/IPv6 probe source management.
- Security:
  - Sensitive fields stored in native OS secure storage (`keyring`):
    - Windows: Credential Manager
    - macOS: Keychain
    - Linux: Secret Service (GNOME Keyring / KWallet)
  - Tokens are never written to config.json.

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust |
| GUI | egui (eframe) |
| Async Runtime | tokio |
| HTTP | reqwest (rustls) |
| Secure Storage | keyring |
| Config Paths | directories |
| Auto-start | auto-launch |

## Project Structure

```text
Cargo.toml (workspace)
freeddns-app/          # GUI entry + scheduler integration
freeddns-core/         # Models, Provider trait, IP resolvers, scheduler
freeddns-providers/    # Cloudflare + Generic HTTP implementations
freeddns-storage/      # JSON persistence + keyring secure storage
```

## Building

### Requirements

- [Rust](https://www.rust-lang.org/tools/install) (stable, latest recommended)

### Development

```bash
cargo run --bin freeddns-app
```

### Release Build

```bash
cargo build --release
```

The binary will be at `target/release/freeddns-app` (Linux/macOS) or `target/release/freeddns-app.exe` (Windows).

## Config File Location

Config paths follow OS standards (managed by the `directories` crate):

| OS | Path |
|----|------|
| Windows | `%APPDATA%\sharkxuanbee\freeddns\config\config.json` |
| macOS | `~/Library/Application Support/com.sharkxuanbee.freeddns/config.json` |
| Linux | `~/.config/freeddns/config.json` |

Sensitive fields are stored in the OS secure storage, not in the config file.

## License

MIT — see [LICENSE](LICENSE)
