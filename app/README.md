# 极速云进销存 - Android App（Flutter）

> 对齐文档基线：`产品需求文档.md`、`架构设计文档.md`、`API接口定义文档.md`

## 1. 当前阶段

已完成 Android 端 **M1 启动骨架**：

- Flutter 工程基础配置（`pubspec.yaml`、lint、目录结构）
- 统一 API 客户端封装（`code/message/data/request_id`）
- 认证主流程（注册 / 登录 / 退出）
- 会话持久化（`shared_preferences`）
- 首页经营看板读取（`GET /reports/dashboard`）

## 2. 目录结构

```text
app/
├─ lib/
│  ├─ main.dart
│  └─ src/
│     ├─ app.dart
│     ├─ config/
│     ├─ core/
│     ├─ network/
│     ├─ storage/
│     └─ features/
│        ├─ auth/
│        └─ dashboard/
├─ pubspec.yaml
└─ analysis_options.yaml
```

## 3. 环境变量

默认 API 地址：

```text
http://1.14.45.242:8080/api/v1
```

可通过 `--dart-define` 覆盖：

```bash
flutter run --dart-define=API_BASE_URL=http://1.14.45.242:8080/api/v1
```

## 4. 后续建议（下一步）

1. 接入扫码能力（`MobileScanner`）与离线队列（PRD P0/P1）。
2. 落地采购入库 / 销售出库 / 盘点三条移动端主链路页面。
3. 增加 refresh token 自动续期与离线冲突处理（`4091/4092`）。