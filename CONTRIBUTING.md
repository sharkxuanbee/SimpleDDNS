# 贡献指南

感谢你愿意改进 SimpleDDNS。

## 提交前检查

- 本地通过构建与测试：
  - `dotnet build SimpleDDNS.sln -m:1`
  - `dotnet test SimpleDDNS.sln -m:1`
- 不要在提交中包含任何真实凭据（Token / Secret / Password）。

## 建议的贡献方向

- 新增 Provider：实现 `SimpleDDNS.Core/Abstractions/IDdnsProvider.cs` 并在 UI 中注册。
- 增加探测源：为 IPv4/IPv6 提供更多公开服务或更健壮的解析策略。
- UI/交互优化：向导流程、表单校验、错误提示、无障碍支持。
- 文档与截图：补充真实截图、使用教程、常见问题。

## 提交规范

- 保持改动聚焦、可回滚。
- 尽量为新增/修复添加单元测试（位于 `tests/SimpleDDNS.Tests`）。
- 代码风格以仓库现有写法为准。

