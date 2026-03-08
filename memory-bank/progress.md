# 项目进度 (Progress)

## 2026-03-08（Android App 启动开发）

- [x] 新建 Android Flutter 工程骨架（`app/`）
  - `pubspec.yaml`
  - `analysis_options.yaml`
  - `.gitignore`
  - `README.md`
  - `lib/main.dart`

- [x] 落地 Android 统一网络层（对齐 PRD/ADD/API）
  - `code/message/data/request_id` 响应解包
  - 自动注入 `x-request-id`、`x-client-type=android`
  - 写接口自动注入 `x-idempotency-key`
  - Bearer Token 自动附带

- [x] 落地 Android 认证与会话闭环（注册/登录/退出）
  - 认证仓储：`/auth/register`、`/auth/login`、`/auth/logout`
  - 会话模型与持久化（`shared_preferences`）
  - 认证状态控制器与登录/注册页面（双模式）

- [x] 落地 Android 经营看板首屏（`GET /reports/dashboard`）
  - 看板模型/仓储/控制器/页面
  - 日期查询与格式校验（`YYYY-MM-DD`）
  - 支持刷新、错误提示、退出登录

- [x] 完成应用启动装配与认证态路由切换
  - 未登录：`AuthPage`
  - 已登录：`DashboardPage`

- [x] 完成当前环境可执行验证并记录限制
  - `flutter --version` -> `command not found`
  - `dart --version` -> `command not found`
  - `dart format` / `flutter analyze` / `flutter run` 当前环境无法执行

**当前状态：**
- Android 端已进入“可联调”的 M1 启动版阶段（认证 + 会话 + 经营看板）。
- 待在具备 Flutter SDK 的机器完成格式化、静态检查与真机/模拟器运行验证。

## 2026-03-07 更新

- [x] 恢复了 `routes.rs` 模块重构中丢失的 94 个集成测试。
- [x] 增补了 3 个针对员工管理（OWNER 操作、非 OWNER 拦截、用户名冲突）的集成测试。
- [x] 成功恢复并运行了全部 111 个集成测试 (`ok. 111 passed; 0 failed`)。
- [x] 清理了临时备份文件 `routes.rs.backup`。
- [x] 成功为 Windows 客户端编译了 `.exe` 可执行文件。文件路径：`client/src-tauri/target/x86_64-pc-windows-gnu/release/app.exe`。
- [x] 按 `.clinerules` 完成服务端远端部署到 `ubuntu@1.14.45.242:/projects/jxcServer`，并通过健康检查（`/health -> code=200`）。
- [x] 远端部署验收通过：`cargo test`（111 passed）+ `cargo build --release` + `systemd` 服务重启成功。
- [x] 更新 Windows 便携包交付物：
  - `client/dist-windows/app.exe`
  - `client/dist-windows/jxc-windows-x64-portable.zip`
  - `client/dist-windows/jxc-windows-x64-portable.zip.sha256`
  - 当前 zip SHA256：`185ead8482c0e67911f57d2f6daf9f791e43bd299a8e8a7f28b15925235c8bc1`
- [x] 完成 Windows 打包策略落地（跨平台约束下）：
  - 新增 npm 脚本：
    - `tauri:build:win:portable`（GNU + app）
    - `tauri:build:win:portable:dist`（GNU + app + 自动归档 zip/sha256）
    - `tauri:build:win:nsis`（MSVC + nsis，需 Windows 主机/CI）
  - 新增便携归档脚本：`client/scripts/package_windows_portable.sh`
  - 新增 GitHub Actions 工作流：`.github/workflows/client-windows-installer.yml`
    - `windows-latest` 构建 NSIS installer
    - 同时产出 portable zip + sha256 并上传 artifacts
  - 本地命令验证：`npm run tauri:build:win:portable:dist` 成功，产物生成正常。
  - 最新 portable 包 SHA256：`897dd5896748de8220fbc440a61f54317e38e81ced900322f667f5120d62284c`

**已知问题：**
- 当前 macOS 本机环境下 Tauri CLI `--bundles` 不提供 `nsis`（仅 `ios/app/dmg`），因此 NSIS 安装包需通过 Windows 主机或 CI（已提供 workflow）产出。

## 2026-03-08 更新

- [x] 租户注册 P0 全链路补齐（后端 + 文档 + 前端）
  - 后端路由挂载：`server/src/routes/mod.rs` 增加 `POST /auth/register`
  - 后端测试新增：`server/src/routes/tests.rs`
    - `register_success_returns_owner_tokens_and_tenant_id`
    - `register_with_empty_required_fields_returns_4000`
    - `register_with_duplicate_username_returns_4090`
  - OpenAPI 更新：`server/openapi.yaml`
    - 新增 `/api/v1/auth/register`
    - 新增 `RegisterRequest` schema
  - API 文档更新：`API接口定义文档.md`
    - 新增“2.1 租户注册”章节
    - Header 鉴权说明更新为“除注册/登录/刷新外”
  - 前端接入：
    - `client/src/types/api.ts` 新增 `RegisterRequest/RegisterResponseData`
    - `client/src/api/auth.ts` 新增 `registerApi()`
    - `client/src/pages/LoginPage.vue` 新增登录/注册双模式与注册成功自动登录

- [x] 租户注册链路验证通过
  - 本地后端：`cargo test --manifest-path server/Cargo.toml` -> `114 passed`
  - 本地前端：
    - `npm run typecheck --prefix client` -> 通过
    - `npm run build --prefix client` -> 通过

- [x] 服务端远端部署与健康检查完成（按 `.clinerules`）
  - 执行脚本：`bash server/scripts/deploy_remote.sh`
  - 部署目标：`ubuntu@1.14.45.242:/projects/jxcServer`
  - 远端结果：
    - `cargo test` -> `114 passed`
    - `cargo build --release` -> 成功
    - `jxc-server.service` -> `active (running)`
    - 健康检查：`/health` 返回 `code=200`
  - 备注：`REDIS_URL` 未配置时仍可运行（降级内存态幂等缓存，日志仅 WARN）

- [x] 客户端认证闭环第一阶段完成（对齐 PRD/API 的 P0）
  - 新增认证类型：`RefreshTokenRequest`、`RefreshTokenResponseData`、`LogoutResponseData`
  - 新增认证 API：`POST /auth/refresh`、`POST /auth/logout`
  - auth store 新增 `refreshToken` 计算属性
- [x] HTTP 层新增 401 自动续期与请求重放
  - 响应拦截器识别 `code=4010`
  - 单飞刷新，避免并发重复刷新
  - 原请求仅重放一次，避免无限循环
  - 刷新失败统一清理会话并跳转登录
- [x] 退出登录流程改造
  - 先调用后端 `/auth/logout`
  - 无论接口成功/失败均本地清理会话并回到登录页
  - 按钮增加“退出中”禁用态，避免重复点击
- [x] 客户端类型校验通过
  - 执行命令：`npm run typecheck --prefix client`
  - 结果：0 错误

- [x] 客户端员工管理能力上线（P0）
  - 新增 API：`client/src/api/users.ts`
    - `GET /users`
    - `POST /users`
    - `PATCH /users/{id}/role`
    - `POST /users/{id}/reset-password`
  - 新增页面：`client/src/pages/UsersPage.vue`
    - 员工列表、创建员工、修改角色、重置密码
  - 新增路由：`/users`（仅 `OWNER`）
  - 路由守卫增加 `meta.roles` 角色校验
  - 侧边栏导航按角色可见（OWNER/PURCHASER/SALES）
  - 复验通过：`npm run typecheck --prefix client`

- [x] 客户端商品扫码建档流程上线（P0）
  - 类型扩展：`client/src/types/api.ts`
    - 新增 `ScanProductData`，对齐 `GET /products/scan`
  - 商品 API 增强：`client/src/api/products.ts`
    - 新增 `scanProductApi(barcode)`
  - 商品页流程落地：`client/src/pages/ProductsPage.vue`
    - 新增“扫码查询 / 快速建档”区域
    - 扫码命中：自动定位并支持直接进入编辑
    - 扫码未命中（`4040`）：自动回填条码到创建表单
    - 无建档权限时给出角色限制提示
  - 复验通过：
    - `npm run typecheck --prefix client`
    - `npm run build --prefix client`

- [x] 客户端商品模块权限细化（P0，安全口径对齐）
  - 文件：`client/src/pages/ProductsPage.vue`
  - 已落地：
    - `SALES` 角色不展示 `cost_price`（对齐 API 文档“SALES 不可查看成本价”）
    - `SALES` 角色隐藏商品创建/编辑区块
    - 列表操作列对无写权限角色显示“只读”
    - `OWNER/PURCHASER` 保持完整写能力
  - 复验通过：
    - `npm run typecheck --prefix client`
    - `npm run build --prefix client`

- [x] 客户端扫码作业主链路补齐（P0）
  - 入库扫码能力：`client/src/pages/InboundPage.vue`
    - 新增“扫码入库（快速累加）”区域
    - 条码命中后自动回填商品并将 `qty` 累加
    - 拦截跨商品混扫，降低误操作风险
    - `4040` 未建档条码提示跳转商品建档
  - 出库扫码能力：`client/src/pages/OutboundPage.vue`
    - 新增“扫码出库（快速累加）”区域
    - 条码命中后自动新增/累加出库明细
    - 扫码区支持可选销售价（为空回落零售价）
    - 角色限制对齐：仅 `OWNER/SALES` 可提交出库
  - 复验通过：
    - `npm run typecheck --prefix client`
    - `npm run build --prefix client`

- [x] 客户端单据状态流扫码能力补齐（P0）
  - 销售单创建扫码：`client/src/pages/SalesOrdersPage.vue`
    - 新增“扫码选品（创建销售单）”区域
    - 条码命中后自动新增/累加销售明细（同商品 `qty +1`）
    - 支持可选销售单价（为空回落 `retail_price`）
    - 角色与权限对齐：仅 `OWNER/SALES`；`4040` 未建档提示
  - 采购单创建扫码：`client/src/pages/PurchaseOrdersPage.vue`
    - 新增“扫码选品（创建采购单）”区域
    - 条码命中后自动新增/累加采购明细（同商品 `qty +1`）
    - 支持可选单次进价（为空回落 `cost_price`）
    - 角色与权限对齐：仅 `OWNER/PURCHASER`；`4040` 未建档提示
  - 盘点单创建扫码：`client/src/pages/StockChecksPage.vue`
    - 新增“扫码选品（创建盘点单）”区域
    - 条码命中后自动追加盘点明细，并按 `product_id` 去重
    - 重复扫码同商品提示“无需重复添加”
    - 角色与权限对齐：仅 `OWNER/PURCHASER`；`4040` 未建档提示
  - 复验通过：
    - `npm run typecheck --prefix client`
    - `npm run build --prefix client`

- [x] 客户端权限可见性与查询参数校验补齐（P0）
  - 看板权限细化：`client/src/pages/DashboardPage.vue`
    - `SALES` 角色隐藏毛利润指标（对齐最小权限与敏感指标收敛）
  - 销售报表权限细化：`client/src/pages/SalesReportPage.vue`
    - `SALES` 角色隐藏成本/毛利相关指标与列，避免敏感经营数据暴露
    - 导出权限继续保持 `OWNER/PURCHASER`
  - 库存流水查询校验：`client/src/pages/StockLogsPage.vue`
    - 新增 `product_id`、`operator_id`、日期区间前端校验，拦截无效请求
  - 审计日志查询校验：`client/src/pages/AuditLogsPage.vue`
    - 新增 `operator_id` 与日期区间前端校验，前置拦截 `4000` 场景
  - 复验通过：
    - `npm run typecheck --prefix client`
    - `npm run build --prefix client`

- [x] 客户端库存不足错误提示增强（P0，4001 细化）
  - 类型补齐：`client/src/types/api.ts`
    - 新增 `StockInsufficientErrorData`（`failed_product_id` / `available_stock` / `required_qty`）
  - 出库页增强：`client/src/pages/OutboundPage.vue`
    - 新增 `parseStockInsufficientData()`
    - `formatApiError()` 对 `code=4001` 输出“商品ID/可用库存/需求数量”
  - 销售单状态流增强：`client/src/pages/SalesOrdersPage.vue`
    - 新增 `parseStockInsufficientData()`
    - 在创建/确认/作废/退货链路统一输出 4001 细化提示
  - 采购单状态流增强：`client/src/pages/PurchaseOrdersPage.vue`
    - 新增 `parseStockInsufficientData()`
    - 在创建/确认/作废链路统一输出 4001 细化提示
  - 复验通过：
    - `npm run typecheck --prefix client`
    - `npm run build --prefix client`

- [x] 服务端 Redis 幂等缓存收口完成（线上稳定性）
  - 远端安装并启用 `redis-server`
  - 更新 `/projects/jxcServer/.env`：`REDIS_URL=redis://127.0.0.1:6379`
  - 重启 `jxc-server.service` 后验证通过：
    - `redis-server=active`
    - `jxc-server=active`
    - `/health -> code=200`
    - 启动日志：`redis_enabled=true`（不再降级到内存幂等缓存）

- [x] 部署后认证注册冒烟能力落地（自动化验收）
  - 新增脚本：`server/scripts/smoke_auth_register.sh`
    - 校验链路：注册 -> 登录 -> 刷新 -> 退出 -> 重复注册冲突（4090）
    - 支持无 `jq` 环境（自动回退 `python3`）
  - 部署脚本增强：`server/scripts/deploy_remote.sh`
    - 新增 `RUN_SMOKE_AUTH_REGISTER` 开关（默认关闭）
    - 新增 `SMOKE_BASE_URL` 可配置冒烟地址
    - 兼容性修复：大小写转换改为 `tr`，避免 `${var,,}` 在非 bash shell 报错
  - 验证结果：
    - `bash -n` 语法校验通过
    - `RUN_SMOKE_AUTH_REGISTER=1 bash server/scripts/deploy_remote.sh` 远端执行通过
    - 远端 `cargo test=114 passed`、`cargo build --release` 成功、healthcheck 通过、冒烟链路全通过

- [x] Windows Installer CI 工作流收口增强（发布链路）
  - 变更文件：`.github/workflows/client-windows-installer.yml`
  - 已落地能力：
    - 触发策略：保留 `workflow_dispatch`，新增 `push`（`main/master` + `client/**`/workflow 文件变更）
    - 并发控制：新增 `concurrency`，同分支新任务自动取消旧任务
    - 最小权限：新增 `permissions: contents: read`
    - 构建优化：新增 `swatinem/rust-cache@v2`（`client/src-tauri -> target`）
    - 产物校验：新增 NSIS 安装包存在性检查与 `*.exe.sha256` 生成
    - 上传严谨性：NSIS artifact 上传新增 `if-no-files-found: error`
    - 路径统一：引入 `TARGET_TRIPLE=x86_64-pc-windows-msvc`，减少硬编码路径漂移
  - 本地验证：
    - `ruby -e "require 'yaml'; YAML.load_file('.github/workflows/client-windows-installer.yml'); puts 'workflow_yaml_ok'"` 通过
  - 受限项：
    - 当前工作目录非 GitHub 仓库根（缺少根 `.git`）且本机无 `gh` CLI/Token，无法直接在本地触发远端 Actions；
    - 工作流已完成可离线校验，待接入仓库后可直接触发执行。