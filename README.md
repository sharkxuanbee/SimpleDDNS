# SimpleDDNS

中文 | [English](README.en.md)

Windows 上简单易用的 GUI DDNS 客户端（WPF / .NET 10）。

目标用户是普通用户：配置好后点按钮即可把域名 A/AAAA 自动同步到当前公网 IP。

## 功能

- 多 Profile 管理：新增 / 编辑 / 删除 / 启用 / 停用。
- Provider：
  - Cloudflare（API Token，自动按 Zone Name 查 Zone ID，自动查找/创建/更新 A/AAAA）。
  - 通用 HTTP 自定义（URL 模板、GET/POST、自定义 Header、POST JSON 模板、测试请求）。
- IPv4/IPv6 独立探测：
  - IPv4 探测源独立列表。
  - IPv6 探测源独立列表（默认内置 3 个 IPv6-only 风格地址）。
  - 支持本地网卡探测源：`local://ipv4`、`local://ipv6`（可用于按本机网卡地址更新）。
  - 任一协议失败不会阻塞另一协议。
- 仅变更时更新：仅当 A/AAAA 对应 IP 变化才调用 DNS 更新。
- 稳定调度：
  - 全局 Start/Stop。
  - 每个 Profile 单独启停。
  - 同一 Profile 不并发执行（`SemaphoreSlim`）。
  - 超时 + 指数退避重试。
- GUI：
  - 主窗口列表（名称、域名、A/AAAA 开关、上次更新、当前 IPv4/IPv6、状态）。
  - 向导式编辑（Provider -> 域名 -> 凭据 -> IPv4/IPv6 -> 高级 -> 完成）。
  - 日志面板 + 导出日志。
  - 托盘菜单（打开、全局启停、立即更新、退出）。
  - 关闭窗口默认最小化到托盘（可在设置中关闭）。
- 设置：
  - 开机自启动（HKCU Run，无需管理员）。
  - 默认间隔。
  - IPv4/IPv6 探测源管理（分别管理）。
- 安全：
  - 敏感字段不明文存储。
  - 使用 Windows DPAPI（`CryptProtectData` / `CryptUnprotectData`）加密后写入本地 JSON。
  - 导出配置默认不含敏感字段；如导出敏感字段会二次确认。
- 中文界面文案集中在 `src/SimpleDDNS.App/Resources/Strings.zh-CN.xaml`，便于后续多语言扩展。

## 截图

![主界面](docs/screenshots/main-window.png)
![向导](docs/screenshots/profile-wizard.png)
![设置](docs/screenshots/settings.png)

## 工程结构

```text
SimpleDDNS.sln
src/
  SimpleDDNS.App/          # WPF UI
  SimpleDDNS.Core/         # 调度、IP 探测、模板渲染
  SimpleDDNS.Providers/    # Cloudflare + Generic HTTP
  SimpleDDNS.Storage/      # JSON 持久化 + DPAPI 加密
  SimpleDDNS.Logging/      # 轻量日志
tests/
  SimpleDDNS.Tests/        # 单元测试
```

## Cloudflare Token 最小权限建议

在 Cloudflare 创建 API Token：

- Permissions:
  - `Zone.DNS:Edit`
  - `Zone.Zone:Read`（用于按 Zone Name 查 Zone ID）
- Zone Resources:
  - 建议限制到目标 Zone（例如 `example.com`）。

不要把 Token 硬编码进源码。请在 GUI 的 Profile 中填写。

## 构建与运行

### 1. 构建

```powershell
dotnet build SimpleDDNS.sln -m:1
```

### 2. 运行

```powershell
dotnet run --project src/SimpleDDNS.App/SimpleDDNS.App.csproj -m:1
```

## 发布（单文件）

### Framework-dependent 单文件（用户需已安装 .NET Runtime）

```powershell
dotnet publish src/SimpleDDNS.App/SimpleDDNS.App.csproj `
  -c Release `
  -r win-x64 `
  --self-contained false `
  -p:PublishSingleFile=true `
  -o publish/win-x64-fdd
```

### Self-contained 单文件（用户无需安装 .NET Runtime）

```powershell
dotnet publish src/SimpleDDNS.App/SimpleDDNS.App.csproj `
  -c Release `
  -r win-x64 `
  --self-contained true `
  -p:PublishSingleFile=true `
  -p:IncludeNativeLibrariesForSelfExtract=true `
  -o publish/win-x64-scd
```

生成的可执行文件位于上述 `publish/...` 目录。

## 配置文件位置

默认配置文件：

- `%APPDATA%\SimpleDDNS\config.json`

配置文件结构示例（不包含敏感字段）：

- [docs/config.example.json](docs/config.example.json)

敏感字段会被加密存储（DPAPI），建议通过 GUI 的导入/导出功能进行迁移，而不是手工编辑配置。

## 测试

```powershell
dotnet test tests/SimpleDDNS.Tests/SimpleDDNS.Tests.csproj -m:1
```

当前测试包含：

- IPv6 响应解析。
- URL/Body 占位符渲染。
- Profile 序列化 + DPAPI 加密/解密。

## FAQ

### 1) 为什么更新了 DDNS 还是连不上家里设备？

常见原因是 **无公网可入站 IP（例如 CGNAT）**。DDNS 只能把域名指向当前出口 IP，不能绕过运营商的入站限制。

可选方案：

- 光猫/路由器端口映射（前提是你有可入站公网 IP）。
- 使用内网穿透方案（反向代理/Tunnel）。

如果你使用 `local://ipv4` / `local://ipv6`，得到的可能是内网地址（如 `192.168.x.x`、`fdxx::/64`），请按实际场景使用。

### 2) 仅 IPv4 或仅 IPv6 网络能用吗？

可以。应用会独立探测并独立更新：

- 仅 IPv4：IPv6 显示不可用，不影响 IPv4 更新。
- 仅 IPv6：IPv4 显示不可用，不影响 IPv6 更新。
- 双栈：两者并行独立工作。

---

如果你要扩展更多 Provider，直接实现 `SimpleDDNS.Core/Abstractions/IDdnsProvider.cs` 并在 `MainWindow` 中注册即可。

## 开源

- 许可证：MIT，见 [LICENSE](LICENSE)
- 贡献指南：见 [CONTRIBUTING.md](CONTRIBUTING.md)
- 安全策略：见 [SECURITY.md](SECURITY.md)
- 行为准则：见 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)
