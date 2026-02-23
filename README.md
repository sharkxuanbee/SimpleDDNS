# SimpleDDNS

中文 | [English](README.en.md)

跨平台、低资源占用的 DDNS 客户端（Rust + egui）。

目标用户是普通用户：配置好 Profile 后，后台自动将域名 A/AAAA 记录同步到当前公网 IP。

支持 **Windows / macOS / Linux**。

## 功能

- 多 Profile 管理：新增 / 编辑 / 删除 / 启用 / 停用。
- Provider：
  - **Cloudflare**（API Token，自动按 Zone Name 查 Zone ID，自动查找/创建/更新 A/AAAA）。
  - **通用 HTTP 自定义**（URL 模板、GET/POST、自定义 Header、POST JSON 模板）。
- IPv4/IPv6 独立探测：
  - IPv4 探测源列表（可自定义）。
  - IPv6 探测源列表（可自定义）。
  - 支持本地网卡探测源：`local://ipv4`、`local://ipv6`。
  - 任一协议失败不会阻塞另一协议。
- 仅变更时更新：仅当 A/AAAA 对应 IP 变化才调用 DNS API。
- 后台调度：
  - 全局 Start/Stop。
  - 每个 Profile 单独启停。
  - 可配置检查间隔。
  - HTTP 请求 15 秒超时。
- GUI（egui 原生）：
  - 主窗口 Profile 列表（名称、域名、A/AAAA 开关、状态指示灯、当前 IP、上次更新时间）。
  - Profile 编辑弹窗（Provider 选择 → 域名 → 凭据 → IPv4/IPv6）。
  - 实时日志面板。
  - 全局 Start/Stop 按钮。
- 设置：
  - 开机自启动（跨平台 `auto-launch`）。
  - 可配置检查间隔。
  - IPv4/IPv6 探测源管理（增删）。
- 安全：
  - 敏感字段使用 AES-256-GCM 加密存储（`secrets.enc`）。
  - 加密密钥由机器信息派生，绑定当前设备。
  - Token 不会以明文写入 config.json。

## 技术栈

| 组件 | 技术 |
|------|------|
| 语言 | Rust |
| GUI | egui (eframe) |
| 异步运行时 | tokio |
| HTTP | reqwest (rustls) |
| 安全存储 | AES-256-GCM (aes-gcm) |
| 配置路径 | directories |
| 开机自启 | auto-launch |

## 工程结构

```text
Cargo.toml (workspace)
freeddns-app/          # GUI 入口 + 后台调度集成
freeddns-core/         # 模型、Provider trait、IP 探测、调度器
freeddns-providers/    # Cloudflare + Generic HTTP 实现
freeddns-storage/      # JSON 持久化 + AES-GCM 加密存储
```

## Cloudflare Token 最小权限建议

在 Cloudflare 创建 API Token：

- Permissions：
  - `Zone.DNS:Edit`
  - `Zone.Zone:Read`（用于按 Zone Name 查 Zone ID）
- Zone Resources：
  - 建议限制到目标 Zone（例如 `example.com`）。

不要把 Token 硬编码进源码。请在 GUI 的 Profile 编辑中填写。

## 构建与运行

### 环境要求

- [Rust](https://www.rust-lang.org/tools/install)（推荐 stable 最新版）

### 开发运行

```bash
cargo run --bin freeddns-app
```

### 发布构建

```bash
cargo build --release
```

生成的可执行文件位于 `target/release/freeddns-app`（Linux/macOS）或 `target/release/freeddns-app.exe`（Windows）。

### 交叉编译示例

```bash
# Linux
cargo build --release --target x86_64-unknown-linux-gnu

# macOS
cargo build --release --target x86_64-apple-darwin

# Windows
cargo build --release --target x86_64-pc-windows-msvc
```

## 配置文件位置

配置文件路径遵循各系统标准（由 `directories` crate 管理）：

| 系统 | 路径 |
|------|------|
| Windows | `%APPDATA%\sharkxuanbee\freeddns\config\config.json` |
| macOS | `~/Library/Application Support/com.sharkxuanbee.freeddns/config.json` |
| Linux | `~/.config/freeddns/config.json` |

敏感字段加密存储在同目录下的 `secrets.enc` 文件中，不在 `config.json` 内。

## FAQ

### 1) 为什么更新了 DDNS 还是连不上家里设备？

常见原因是 **无公网可入站 IP（例如 CGNAT）**。DDNS 只能把域名指向当前出口 IP，不能绕过运营商的入站限制。

可选方案：

- 光猫/路由器端口映射（前提是你有可入站公网 IP）。
- 使用内网穿透方案（反向代理/Tunnel）。

### 2) 仅 IPv4 或仅 IPv6 网络能用吗？

可以。应用会独立探测并独立更新：

- 仅 IPv4：IPv6 显示不可用，不影响 IPv4 更新。
- 仅 IPv6：IPv4 显示不可用，不影响 IPv6 更新。
- 双栈：两者并行独立工作。

### 3) 如何扩展更多 Provider？

在 `freeddns-providers` 中实现 `freeddns_core::provider::DdnsProvider` trait，然后在 `freeddns-app/src/main.rs` 的 `build_providers()` 中注册即可。

---

## 开源

- 许可证：MIT，见 [LICENSE](LICENSE)
- 贡献指南：见 [CONTRIBUTING.md](CONTRIBUTING.md)
- 安全策略：见 [SECURITY.md](SECURITY.md)
- 行为准则：见 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
