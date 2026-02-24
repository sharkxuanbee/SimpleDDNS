# 贡献指南

感谢你愿意改�?SimpleDDNS�?

## 环境准备

- 安装 [Rust](https://www.rust-lang.org/tools/install) (stable 最新版)

## 提交前检�?

- 本地通过构建与测试：
  ```bash
  cargo check --workspace
  cargo test --workspace
  ```
- 不要在提交中包含任何真实凭据（Token / Secret / Password）�?

## 建议的贡献方�?

- **新增 Provider**：在 `simpleddns-providers` 中实�?`simpleddns_core::provider::DdnsProvider` trait，并�?`simpleddns-app/src/main.rs` �?`build_providers()` 中注册�?
- **增加探测�?*：为 IPv4/IPv6 提供更多公开服务或更健壮的解析策略�?
- **UI/交互优化**：表单校验、错误提示、主题、无障碍支持�?
- **文档**：使用教程、常见问题、截图�?

## 提交规范

- 保持改动聚焦、可回滚�?
- 尽量为新�?修复添加单元测试�?
- 代码风格以仓库现有写法为准，提交前运�?`cargo fmt`�?
