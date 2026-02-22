# 安全策略

## 报告安全问题

如果你发现了可能涉及凭据泄露、远程代码执行、权限提升等安全问题，请不要在公开 Issue 中直接披露。

建议通过私下方式联系维护者，说明：

- 复现步骤
- 影响范围（版本/场景）
- 可能的缓解方案（如果有）

## 敏感信息存储

应用使用系统原生安全存储保护敏感字段：

| 系统 | 存储后端 |
|------|---------|
| Windows | Credential Manager |
| macOS | Keychain |
| Linux | Secret Service (GNOME Keyring / KWallet) |

Token 不会写入 `config.json` 配置文件。

## 使用建议

- 不要把 Token/Secret 写入源码或提交到仓库。
- 建议为各 Provider 使用最小权限的凭据（例如 Cloudflare Token 限定到目标 Zone）。
