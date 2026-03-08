# 当前工作焦点 (Active Context)

## 2026-03-08（Android App 启动开发）

### 最新完成任务（Android 端 M1 启动版）
- 新增 `app/` Flutter 工程基础骨架：
  - `pubspec.yaml`（`dio/shared_preferences/uuid`）
  - `analysis_options.yaml`
  - `.gitignore`
  - `README.md`
  - `lib/main.dart`

- 按 PRD/ADD/API 契约落地 Android 统一网络层：
  - `app/lib/src/network/api_client.dart`
    - 统一请求头：`x-request-id`、`x-client-type=android`
    - 写接口自动注入 `x-idempotency-key`
    - 自动附带 Bearer Token（从 `SessionStorage` 读取）
    - 统一解析响应协议：`code/message/data/request_id`
  - `app/lib/src/network/api_envelope.dart`
  - `app/lib/src/network/api_exception.dart`

- 落地 Android 认证与会话闭环（对齐 API 2.x）：
  - 会话模型：
    - `app/lib/src/core/models/user_role.dart`
    - `app/lib/src/core/models/auth_user.dart`
    - `app/lib/src/core/models/user_session.dart`
  - 会话持久化：
    - `app/lib/src/storage/session_storage.dart`（`shared_preferences`）
  - 认证仓储：
    - `app/lib/src/features/auth/models/auth_repository.dart`
    - `app/lib/src/features/auth/models/login_request.dart`
    - `app/lib/src/features/auth/models/register_request.dart`
  - 认证状态控制：
    - `app/lib/src/features/auth/application/session_controller.dart`
    - 覆盖：恢复会话、登录、注册、退出、错误归一化
  - 认证页面：
    - `app/lib/src/features/auth/presentation/auth_page.dart`
    - 登录/注册双模式，字段校验与错误提示

- 落地 Android 经营看板首屏（对齐 API 5.1）：
  - 看板模型与仓储：
    - `app/lib/src/features/dashboard/models/dashboard_data.dart`
    - `app/lib/src/features/dashboard/models/dashboard_repository.dart`
  - 看板状态控制：
    - `app/lib/src/features/dashboard/application/dashboard_controller.dart`
  - 看板页面：
    - `app/lib/src/features/dashboard/presentation/dashboard_page.dart`
    - 支持日期查询（含 `YYYY-MM-DD` 格式校验）、下拉刷新、退出登录

- 应用装配与页面切换：
  - `app/lib/src/core/app_services.dart`（依赖装配）
  - `app/lib/src/app.dart`（启动 -> 认证页/看板页）

### 当前限制与说明
- 本机未安装 Flutter/Dart：
  - `flutter --version` -> `command not found`
  - `dart --version` -> `command not found`
  - `dart format` 无法执行
- 因环境限制，本轮仅完成代码落地与结构对齐，暂无法本机执行 `flutter analyze` / `flutter run`。

### 下一步建议
1. 在具备 Flutter 环境机器执行：
   - `flutter pub get`
   - `dart format lib`
   - `flutter analyze`
   - `flutter run`
2. 按 PRD P0 继续落地 Android 主链路：
   - 扫码入库、扫码出库、库存盘点
3. 增加 refresh token 自动续期与离线冲突处理（`4091/4092`）
4. 增补 Android 端最小化测试（仓储层 + 控制器层）

## 2026-03-08 更新

### 最新完成任务（租户注册 P0 全链路）
- **后端注册链路打通并补测**：
  - `server/src/routes/mod.rs`
    - `public_routes()` 新增 `POST /auth/register` 挂载，正式开放 `POST /api/v1/auth/register`。
  - `server/src/routes/tests.rs`
    - 新增注册核心回归用例：
      - `register_success_returns_owner_tokens_and_tenant_id`
      - `register_with_empty_required_fields_returns_4000`
      - `register_with_duplicate_username_returns_4090`
  - 本地后端验证通过：`cargo test --manifest-path server/Cargo.toml`（114 passed）。

- **文档契约补齐（PRD/API 对齐）**：
  - `server/openapi.yaml`
    - 新增 `/api/v1/auth/register` path（200/400/409）
    - 新增 `RegisterRequest` schema（`tenant_name` 可选，`username/name/password` 必填）
  - `API接口定义文档.md`
    - 认证模块新增“2.1 租户注册”章节（请求/响应/错误语义）
    - Header 规范更新为“除注册/登录/刷新外均需 Bearer Token”

- **前端注册流程落地（登录页一体化）**：
  - `client/src/types/api.ts`
    - 新增 `RegisterRequest` / `RegisterResponseData`
  - `client/src/api/auth.ts`
    - 新增 `registerApi()` -> `POST /auth/register`
  - `client/src/pages/LoginPage.vue`
    - 登录/注册双模式切换（同页）
    - 注册表单字段：`tenantName(可选)`、`username`、`name`、`password`
    - 注册成功自动写入会话并跳转主业务页（与登录行为一致）
  - 本地前端验证通过：
    - `npm run typecheck --prefix client`
    - `npm run build --prefix client`

- **服务端已完成远端部署与健康检查**（按 `.clinerules`）
  - 执行：`server/scripts/deploy_remote.sh`
  - 远端：`ubuntu@1.14.45.242:/projects/jxcServer`
  - 远端验证：
    - `cargo test`（114 passed）
    - `cargo build --release` 成功
    - `jxc-server.service` 重启并 `active (running)`
    - `healthcheck.sh http://127.0.0.1:8080/health` -> `code=200`
  - 运行告警：`STORAGE_BACKEND=postgres` 且未配置 `REDIS_URL`，当前继续使用内存态幂等缓存（非阻断）。

### 最新完成任务
- **客户端认证闭环（P0）已落地第一阶段**：
  - `client/src/types/api.ts` 新增认证相关类型：
    - `RefreshTokenRequest`
    - `RefreshTokenResponseData`
    - `LogoutResponseData`
  - `client/src/api/auth.ts` 新增接口：
    - `refreshTokenApi()` -> `POST /auth/refresh`
    - `logoutApi()` -> `POST /auth/logout`
  - `client/src/stores/auth.ts` 增加 `refreshToken` 计算属性，补齐会话读取能力。
  - `client/src/api/http.ts` 增加 401 自动续期机制：
    - 统一响应拦截器识别 `code=4010`
    - 使用单飞（single-flight）策略避免并发重复刷新 token
    - 刷新成功后自动重放原请求（仅一次，防无限重试）
    - 刷新失败后统一清理会话并重定向登录页
  - `client/src/layouts/MainLayout.vue` 退出流程改为：
    - 先调用后端 `/auth/logout`
    - 无论结果都执行本地会话清理并跳转登录
    - 增加按钮防抖状态（`退出中...`）

- **客户端员工管理（P0）已落地第一阶段**：
  - 新增用户管理 API 文件：`client/src/api/users.ts`
    - `listUsersApi()` -> `GET /users`
    - `createUserApi()` -> `POST /users`
    - `updateUserRoleApi()` -> `PATCH /users/{id}/role`
    - `resetUserPasswordApi()` -> `POST /users/{id}/reset-password`
  - 扩展类型定义：`client/src/types/api.ts`
    - `UserData` / `ListUsersResponseData`
    - `CreateUserRequest` / `UpdateUserRoleRequest`
    - `ResetUserPasswordRequest` / `ResetUserPasswordResponseData`
  - 新增页面：`client/src/pages/UsersPage.vue`
    - 员工列表、创建员工、修改角色、重置密码
    - 仅 OWNER 可用（前端显式提示）
  - 路由接入：`client/src/router/index.ts`
    - 新增 `/users` 路由，`meta.roles=['OWNER']`
    - 守卫增加角色校验，不满足角色回退到 `/dashboard`
  - 导航接入：`client/src/layouts/MainLayout.vue`
    - 新增“员工管理”菜单
    - 菜单按角色可见（最小权限）
    - 采购/销售/审计等菜单按角色显示，前端权限表达更清晰

- **客户端商品扫码建档流程（P0）已落地**：
  - 类型扩展：`client/src/types/api.ts`
    - 新增 `ScanProductData`，对齐 `GET /products/scan` 响应契约
  - API 封装：`client/src/api/products.ts`
    - 新增 `scanProductApi(barcode)` -> `GET /products/scan?barcode=...`
  - 页面集成：`client/src/pages/ProductsPage.vue`
    - 新增“扫码查询 / 快速建档”卡片
    - 已存在商品：提示命中、自动按条码过滤列表，且在有写权限时自动进入编辑态
    - 未存在商品（`4040`）：自动回填条码到创建表单，便于快速建档
    - 无建档权限角色：给出明确提示（仅 `OWNER/PURCHASER`）

- **客户端商品权限细化（P0）已落地（对齐 API 7.1）**：
  - 文件：`client/src/pages/ProductsPage.vue`
  - 角色约束增强：
    - `SALES` 角色不展示 `cost_price` 列（敏感成本字段）
    - `SALES` 角色隐藏“创建商品属性 / 编辑商品属性”区块
    - 商品列表操作列在无写权限时展示“只读”状态
  - `OWNER/PURCHASER` 保持完整写操作能力（创建、编辑、删除）

- **客户端扫码作业能力继续增强（P0）**：
  - 入库页：`client/src/pages/InboundPage.vue`
    - 新增“扫码入库（快速累加）”区域
    - 调用 `scanProductApi` 按条码识别商品并自动回填 `product_id/barcode`
    - 同一商品连续扫码自动将入库数量 `qty` 累加（+1）
    - 已选择商品时拦截“跨商品混扫”，避免误入库
    - 未建档条码（`4040`）提示先到商品管理建档
  - 出库页：`client/src/pages/OutboundPage.vue`
    - 新增“扫码出库（快速累加）”区域
    - 条码命中后自动写入出库明细；重复扫码同商品自动累加数量
    - 扫码区支持可选销售单价（为空时回落商品零售价）
    - 角色与 API 权限一致：仅 `OWNER/SALES` 可执行出库扫码提交
  - 销售单状态流页：`client/src/pages/SalesOrdersPage.vue`
    - 新增“扫码选品（创建销售单）”区域
    - 条码命中后自动写入 `createForm.items`，重复扫码同商品自动累加 `qty`
    - 扫码区支持可选销售单价（为空时回落商品零售价）
    - 角色与 API 权限一致：仅 `OWNER/SALES` 可操作；`4040` 提示先建档
  - 采购单状态流页：`client/src/pages/PurchaseOrdersPage.vue`
    - 新增“扫码选品（创建采购单）”区域
    - 条码命中后自动写入 `createForm.items`，重复扫码同商品自动累加 `qty`
    - 扫码区支持可选单次进价（为空时回落商品当前成本价）
    - 角色与 API 权限一致：仅 `OWNER/PURCHASER` 可操作；`4040` 提示先建档
  - 盘点单状态流页：`client/src/pages/StockChecksPage.vue`
    - 新增“扫码选品（创建盘点单）”区域
    - 条码命中后自动追加到盘点创建明细，按 `product_id` 去重
    - 重复商品给出“无需重复添加”提示，避免盘点明细脏数据
    - 角色与 API 权限一致：仅 `OWNER/PURCHASER` 可操作；`4040` 提示先建档

- **客户端权限与查询校验细化（P0，继续对齐 API 安全约束）**：
  - 首页看板：`client/src/pages/DashboardPage.vue`
    - 新增 `canViewGrossProfit` 角色判断，`SALES` 隐藏“毛利润”指标
    - 页面增加角色提示文案，明确数据可见性差异
  - 销售报表：`client/src/pages/SalesReportPage.vue`
    - 新增 `canViewCostMetrics` 角色判断，`SALES` 隐藏“总成本/总毛利”与明细“成本/毛利”列
    - 空数据行 `colspan` 按可见列动态适配，避免表格错位
    - 保持导出权限仅 `OWNER/PURCHASER`（与 API 文档一致）
  - 库存流水：`client/src/pages/StockLogsPage.vue`
    - 新增前端查询参数校验：`product_id` 正整数、`operator_id` UUID、日期成对与区间合法性
    - 校验通过后再发请求，减少无效请求与后端 4000 噪音
  - 审计日志：`client/src/pages/AuditLogsPage.vue`
    - 新增前端查询参数校验：`operator_id` UUID、日期成对与区间合法性
    - 与 API 文档参数校验语义对齐（`4000` 场景前置拦截）

- **客户端库存不足错误信息增强（P0，4001 可操作性提升）**：
  - 类型补齐：`client/src/types/api.ts`
    - 新增 `StockInsufficientErrorData`（`failed_product_id` / `available_stock` / `required_qty`）
  - 销售出库页：`client/src/pages/OutboundPage.vue`
    - 新增 `parseStockInsufficientData()`
    - `formatApiError()` 对 `code=4001` 输出细化提示（商品ID、可用库存、需求数量）
  - 销售单状态流页：`client/src/pages/SalesOrdersPage.vue`
    - 新增 `parseStockInsufficientData()`
    - 创建/确认/作废/退货链路统一复用 4001 细化提示
  - 采购单状态流页：`client/src/pages/PurchaseOrdersPage.vue`
    - 新增 `parseStockInsufficientData()`
    - 创建/确认/作废链路统一复用 4001 细化提示
  - 目标：将原本笼统“库存不足”提示升级为可直接定位问题商品与缺口数量的可执行信息

### 本次验证
- 已执行：`npm run typecheck --prefix client`
- 结果：通过（0 错误）
- 已执行：`npm run build --prefix client`
- 结果：通过（Vite 构建成功，包含 `OutboundPage/SalesOrdersPage/PurchaseOrdersPage` 的 4001 错误提示增强最新产物）

## 2026-03-08（稳定性收口追加）

### Redis 幂等缓存收口（已完成）
- 远端主机：`ubuntu@1.14.45.242`
- 项目目录：`/projects/jxcServer`
- 已执行：
  - 安装并启用 Redis 服务（`redis-server`）
  - 更新服务端环境变量：`REDIS_URL=redis://127.0.0.1:6379`
  - 重启 `jxc-server.service` 并执行健康检查
- 验收结果：
  - `redis-server` 状态：`active`
  - `jxc-server.service` 状态：`active`
  - `/health`：`code=200`
  - 启动日志显示：`redis_enabled=true`
  - 原“未配置 REDIS_URL，降级内存幂等缓存”告警在最新启动中已消失

### 部署验收自动化增强（已完成）
- 新增脚本：`server/scripts/smoke_auth_register.sh`
  - 覆盖链路：`register -> login -> refresh -> logout -> duplicate register(4090)`
  - 默认地址：`http://127.0.0.1:8080/api/v1`
  - 兼容无 `jq` 环境（回退 `python3` 解析 JSON）
- 增强脚本：`server/scripts/deploy_remote.sh`
  - 新增可选开关：`RUN_SMOKE_AUTH_REGISTER=1`
  - 新增可选参数：`SMOKE_BASE_URL`（默认 `http://127.0.0.1:8080/api/v1`）
  - 在远端健康检查后可选执行认证注册冒烟
  - 为兼容 `/bin/sh` 环境，修复大小写转换写法（使用 `tr`，避免 `${var,,}`）

### 本次验收结果
- 本地语法校验：
  - `bash -n server/scripts/smoke_auth_register.sh` 通过
  - `bash -n server/scripts/deploy_remote.sh` 通过
- 远端完整部署验证：
  - `RUN_SMOKE_AUTH_REGISTER=1 bash server/scripts/deploy_remote.sh` 执行成功
  - 远端 `cargo test`：`114 passed`
  - 远端 `cargo build --release`：成功
  - 远端健康检查：`code=200`
  - 远端认证注册冒烟：全部步骤通过

## 2026-03-08（Windows Installer CI 收口追加）

### 客户端 Windows 安装包 CI 工作流增强（已完成）
- 文件：`.github/workflows/client-windows-installer.yml`
- 本轮改动：
  - 触发策略增强：
    - 保留 `workflow_dispatch`
    - 新增 `push` 自动触发（`main/master` 且仅在 `client/**` 或 workflow 文件变更时触发）
  - 运行安全与并发控制：
    - 新增最小权限 `permissions: contents: read`
    - 新增 `concurrency`，同分支新构建会取消旧构建
  - 构建性能优化：
    - 新增 `swatinem/rust-cache@v2` 缓存 `client/src-tauri -> target`
  - 产物完整性增强：
    - 新增 NSIS 产物存在性校验
    - 自动生成 `*.exe.sha256` 并与 NSIS 安装包一并上传
    - `upload-artifact` 增加 `if-no-files-found: error`，缺失工件时快速失败
  - 变量统一：
    - 引入 `TARGET_TRIPLE=x86_64-pc-windows-msvc`，统一构建与打包路径，降低硬编码漂移风险

### 本次验证结果
- 本地 YAML 语法校验通过：
  - `ruby -e "require 'yaml'; YAML.load_file('.github/workflows/client-windows-installer.yml'); puts 'workflow_yaml_ok'"`
  - 输出：`workflow_yaml_ok`
- 受限项说明：
  - 当前目录非完整 GitHub 仓库根（根目录缺少 `.git`，且本机无 `gh` CLI/Token），无法在本地直接触发远端 Actions 运行；
  - 已完成可离线验证项与工作流收口，待仓库接入后可直接触发。

## 2026-03-07 更新

### 最新完成任务
- **集成测试恢复与验证**：在 `server/src/routes/tests.rs` 中成功恢复了 94 个集成测试，并增补了 3 个员工管理相关测试，最终达到了 111 个测试的目标。所有测试均已成功运行 (`ok. 111 passed; 0 failed`)。
- **Windows 客户端编译**：成功为 Windows 平台编译了 `.exe` 可执行文件。文件路径：`client/src-tauri/target/x86_64-pc-windows-gnu/release/app.exe`。
- **服务端远端部署完成**：已按 `.clinerules` 要求部署到 `ubuntu@1.14.45.242:/projects/jxcServer`，远端 `cargo test`（111 通过）与 `cargo build --release` 成功，`jxc-server.service` 运行正常，`/health` 返回 `code=200`。
- **Windows 便携包更新完成**：已使用 `--bundles app` 重新构建并更新 `client/dist-windows`：
  - `app.exe`（新构建）
  - `jxc-windows-x64-portable.zip`
  - `jxc-windows-x64-portable.zip.sha256`
  - 当前 zip SHA256：`185ead8482c0e67911f57d2f6daf9f791e43bd299a8e8a7f28b15925235c8bc1`
- **NSIS 方案澄清与落地**：
  - 本机已安装 `makensis`（Homebrew），但 Tauri CLI 在当前 macOS 环境下 `--bundles` 可选项仅 `ios/app/dmg`，不支持 `nsis`，因此无法本机直接产出 NSIS。
  - 已新增打包脚本：
    - `client/package.json`：`tauri:build:win:portable`、`tauri:build:win:nsis`
  - 已新增 CI 工作流：
    - `.github/workflows/client-windows-installer.yml`
    - 使用 `windows-latest` 构建 NSIS installer，并上传 NSIS + portable 工件。
  - 工作流 YAML 已通过本地语法校验。
- **便携包自动归档流程补齐**：
  - 新增脚本：`client/scripts/package_windows_portable.sh`
    - 自动将 `target/.../app.exe` 复制到 `dist-windows`
    - 自动生成 `jxc-windows-x64-portable.zip`
    - 自动生成 `jxc-windows-x64-portable.zip.sha256`
  - 新增 npm 命令：`tauri:build:win:portable:dist`
    - 一次执行完成“构建 + 归档 + 校验”。
  - 已本地验证通过，最新 SHA256：`897dd5896748de8220fbc440a61f54317e38e81ced900322f667f5120d62284c`

### 遗留或已知问题
- **本机无法直接产出 NSIS**：问题已从“缺少 makensis”演进为“平台能力限制”。当前需通过 Windows 主机或 CI（已提供 workflow）产出 NSIS。

### 下一步计划
- 若仓库启用 GitHub Actions：触发 `client-windows-installer` workflow，下载 NSIS 工件用于分发。
- 若暂未启用 CI：在 Windows 构建机执行 `npm run tauri:build:win:nsis` 产出安装包。
- 对外发布建议默认使用 `npm run tauri:build:win:portable:dist` 产出便携包，并附带 `.sha256` 供校验。
