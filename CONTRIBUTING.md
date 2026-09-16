# 贡献指南

感谢你愿意改进极速云进销存（Hyper JXC）。本文说明怎么把改动提过来、以及提交前需要跑通什么。

## 开发环境

| 组件 | 版本要求 |
| --- | --- |
| Rust | stable（服务端使用 edition 2024，需要 1.85+） |
| Flutter | 3.41.4（与 CI 固定版本一致） |
| Node.js | 22 |
| PostgreSQL | 16（跑 Postgres 仓储回归测试时需要） |

## 提 issue

- 用仓库自带的 issue 模板，把复现步骤、期望结果、实际结果和版本写清楚。
- 安全问题不要开公开 issue，按 [SECURITY.md](SECURITY.md) 的方式报告。
- 提交前先搜索已有 issue，避免重复。

## 提 pull request

- 从 `main` 切出特性分支，分支名建议 `feat/xxx`、`fix/xxx`、`docs/xxx`。
- 保持改动聚焦：一个 PR 解决一件事，避免顺带的大范围重排。
- 跨端改动要三端一起改。API 契约、响应结构、错误码属于共享约定，改动任何一端都必须同步其余端。
- 提交信息用祈使句，必要时在正文里说明「为什么」。
- PR 描述里写清楚：改了什么、怎么验证的、有没有破坏兼容性。

## 文档先行约束

本项目采用「文档先行」：业务代码改动前，先更新基线文档再进入实现（约束见 [`.clinerules`](.clinerules)）。

| 文档 | 何时需要更新 |
| --- | --- |
| [产品需求文档.md](产品需求文档.md) | 功能范围、优先级、验收标准、交互规则变化 |
| [架构设计文档.md](架构设计文档.md) | 分层、数据模型、事务与并发策略、状态机变化 |
| [API接口定义文档.md](API接口定义文档.md) | 接口、字段约束、错误码、幂等语义变化 |

`server/openapi.yaml` 是接口契约的机读版本，接口改动时与 `API接口定义文档.md` 一并更新。

## 提交前必须跑通

```bash
# 服务端
cd server
cargo test                                    # 默认会跳过 Postgres 回归用例
TEST_DATABASE_URL=postgres://... cargo test    # 有 Postgres 时请连它一起跑

# Android 端
cd app
flutter analyze
flutter test

# Windows 客户端
cd client
npm run typecheck
npm run build
```

CI 会对以上几项做同样的检查，另外还会校验五处版本声明是否一致：

```text
app/pubspec.yaml
client/package.json
client/src-tauri/tauri.conf.json
client/src-tauri/Cargo.toml
server/Cargo.toml
```

发版时把这五处一起改，CI 的 `version-consistency` 任务会对不上就失败。

## 数据库迁移

迁移脚本有两份且必须等价：`server/migrations/*.sql`（PostgreSQL）与
`server/migrations/sqlite/*.sql`（SQLite）。

- 编号递增，不要修改已经发布的迁移。
- 两份脚本要表达同样的语义，注意 PostgreSQL 与 SQLite 的类型和语法差异。
- 迁移会在服务端启动时自动应用，请确保脚本可重复执行（`IF NOT EXISTS` 等）。

## 已知未完成项

[README 的「已知缺口」](README.md) 列出了当前明确未实现的部分。若你想认领其中一项，
建议先在 issue 里说明计划，避免多人重复劳动。

## 许可

贡献的代码将以 [Apache License 2.0](LICENSE) 授权。
