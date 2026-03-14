# 当前工作焦点 (Active Context)

## 2026-03-13（前端审美升级 v1.2.31：视觉系统与交互动效收尾）

### 背景
- 用户明确要求对前端进行“审美升级 + 体验升级”，并固定 5 个方向：
  1. 升级色彩系统（拒绝默认蓝）
  2. 优化卡片与阴影（建立呼吸感）
  3. 字体与数据的视觉层级
  4. 精细化图标设计
  5. 改进交互动效（骨架屏、数字滚动、转场）
- 已按仓库规则先完成文档先行，PRD/架构/API 已升级至 `v1.2.31`，本轮进入代码收尾与校验。

### 本轮完成
- 全局视觉基线（Design Tokens + 组件样式）
  - `client/src/style.css`
  - 新增品牌色阶、语义色、背景与阴影 token，建立统一视觉语言；
  - 升级按钮体系（含 `btn-success / btn-warning`）与 `btn-icon` 线性图标样式；
  - 新增导航 duotone 图标色分组（dashboard/inbound/outbound/lowstock 等）；
  - 新增骨架屏 shimmer、页面淡入、卡片 hover 浮动等动效；
  - 新增微可视化样式（趋势线、库存占比条）与次级信息弱化类 `.cell-muted`。

- Dashboard 数据层级与反馈增强
  - `client/src/pages/DashboardPage.vue`
  - 增加骨架屏；
  - 指标数字 count-up（`requestAnimationFrame`）；
  - 销售趋势 sparkline（SVG path/area）与大号等宽数字展示。

- 导航图标化
  - `client/src/layouts/MainLayout.vue`
  - 导航项扩展 `iconClass`，统一渲染 `nav-icon`，提升模块辨识度。

- 低库存页微可视化
  - `client/src/pages/LowStockPage.vue`
  - 首次加载骨架屏；
  - SKU/条码信息弱化；
  - 当前库存新增占比进度条（`calcStockRatio`）。

- 高频操作页图标与语义动作强化
  - `client/src/pages/ProductsPage.vue`
  - `client/src/pages/InboundPage.vue`
  - `client/src/pages/OutboundPage.vue`
  - 查询/清空/编辑/删除/提交等按钮补齐 linear SVG 图标；
  - 创建/提交等关键动作采用语义色按钮，保持“入库偏绿、预警偏橙红”的一致体验。

### 验证结果
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
- 输出：`vue-tsc --noEmit -p tsconfig.app.json`，exit code 0。

### 当前结论
- 前端“审美升级 + 体验升级”已完成代码落地与类型校验：
  - 色彩系统、卡片/阴影、字体层级、图标体系、交互动效均已覆盖；
  - 不涉及后端 API 契约、权限或错误码语义变更；
  - 当前可进入业务体验验收阶段。

## 2026-03-13（“需求没生效”收尾：进货价格文案 + 默认值链路双端校验）

### 背景
- 用户反馈两项需求“看起来没生效”：
  1. 新建商品页要将“成本价”改为“进货价格”，并去掉“批量销售价”；
  2. 采购入库页“单次进价”统一为“进货价格”，且默认值应按 `last_inbound_unit_cost ?? cost_price`。
- 前序会话已完成大部分 Flutter/Web 改造，本轮继续做最后收尾与验证。

### 本轮完成
- Web 入库页残留文案修复：
  - `client/src/pages/InboundPage.vue`
  - 结果区文案：`当前成本价` -> `当前进货价格`
  - 批量结果表头：`成本价` -> `进货价格`
- 全局复核（关键词：`wholesale/单次进价/成本价/批发价`）已执行，确认本次目标范围内关键页面已对齐。
- 双端静态检查通过：
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 根因结论（为什么之前“没生效”）
- 不是单点问题，而是“多处残留叠加”：
  1. 前端存在旧字段/旧文案残留（尤其 `wholesale_price`、`成本价`、`单次进价`）；
  2. 入库 `scan_confirm` 路径早期未完整透传默认 `unit_cost`，导致与 `continuous_scan` 行为不一致；
  3. 用户看到的页面恰好命中尚未收尾的展示区域（例如 Web 入库结果区），因此感知为“没生效”。

### 当前结论
- 本轮已完成最后一公里修复与静态校验，双端在“进货价格文案统一 + 默认进价链路”上达到一致预期。

## 2026-03-13（wholesale_price 移除 + last_inbound_unit_cost 全链路迁移收尾）

### 背景
- 用户要求连续完成后端迁移，不中断：
  1. 移除 `wholesale_price` 全链路；
  2. 打通 `last_inbound_unit_cost`；
  3. 入库默认口径改为 `unit_cost ?? last_inbound_unit_cost ?? cost_price`。

### 本轮完成
- Memory 分支入库口径同步：
  - `server/src/routes/inventory.rs`
  - `inbound` / `inbound_batch` 均改为新 fallback 口径；
  - 入库后回写 `product.last_inbound_unit_cost = Some(effective_unit_cost.round_dp(4))`。
- 内存样例商品字段迁移：
  - `server/src/state.rs`
  - `sample_product` 字段由 `wholesale_price` 改为 `last_inbound_unit_cost: Some(...)`。
- 路由测试全量同步去旧字段：
  - `server/src/routes/tests/business_flow.rs`
  - `server/src/routes/tests/idempotency_replay.rs`
  - `server/src/routes/tests/idempotency_payload_conflict.rs`
  - `server/src/routes/tests/idempotency_missing_key.rs`
  - `server/src/routes/tests/auth_request_id.rs`
  - 已移除请求体中的 `wholesale_price`，并将 `business_flow` 中旧 Product 构造字段替换为 `last_inbound_unit_cost`。
- 测试口径断言更新：
  - `server/src/routes/tests/idempotency_replay.rs`
  - 两个“inbound 无 unit_cost”用例改为断言回落到 `last_inbound_unit_cost` 后的加权成本 `2.1320`（并重命名测试名体现新语义）。

### 验证结果
- `cd /Users/admin/Documents/Projects/jxc/server && cargo test -q` ✅
- 结果：`126 passed; 0 failed`。

### 当前结论
- 服务端 `wholesale_price -> last_inbound_unit_cost` 迁移在内存分支、测试与入库口径上已完成收尾并通过全量测试。

## 2026-03-13（入库真实进价展示收尾）

### 背景
- 用户持续强调“展示每次进货的真实价格”，并要求不中断继续完成。
- 前序已完成后端写入链路、`/inventory/logs` DTO/映射、Flutter model 解析（`snapshotInboundUnitCost`）。

### 本轮完成
- Flutter 展示层补齐：
  - `app/lib/src/features/inventory/presentation/inbound_logs_page.dart`
  - 在入库记录卡片新增：`本次进货价：${log.snapshotInboundUnitCost ?? '--'}`。
- 编译/静态检查：
  - `cd server && cargo check` ✅
  - `cd app && flutter analyze` ✅（No issues found）

### 当前结论
- “入库真实进价快照”已完成 Android 端展示闭环：
  - 后端写入 -> `/inventory/logs` 返回 -> Flutter 模型解析 -> 入库记录页展示。
- 空值兜底按需求显示 `--`。

## 2026-03-13（Android“采购记录”迁移为“入库记录”页面：Flutter 端收尾）

### 背景
- 用户明确目标：将 Android 端 Dashboard 的“采购记录”入口改为“入库记录”，并展示库存流水中的入库记录（`biz_type=IN_PURCHASE`）。
- 本轮在既有文档先行（PRD/ADD/API 已更新至 `v1.2.27`）基础上，继续完成 Flutter 代码链路迁移。

### 本轮完成
- Repository 层
  - `app/lib/src/features/inventory/models/inventory_repository.dart`
  - 新增 `fetchInboundLogs(...)`，请求 `GET /inventory/logs`，固定参数 `biz_type=IN_PURCHASE`。

- Application 层
  - 新增 `app/lib/src/features/inventory/application/inbound_logs_controller.dart`
  - 提供入库记录查询控制器：权限校验（`OWNER/PURCHASER`）、日期/分页校验、错误人性化。

- Presentation 层
  - 新增 `app/lib/src/features/inventory/presentation/inbound_logs_page.dart`
  - 页面支持：默认当天、日期范围日历选择、分页、每页条数回落、只读流水卡片展示（`id/biz_type/biz_no/product_id/operator_id/created_at` + `delta_qty/snapshot_stock/snapshot_cost`）。

- Dashboard / App 装配层
  - `app/lib/src/features/dashboard/presentation/dashboard_page.dart`
    - 快捷入口改为“入库记录”；
    - 枚举从 `purchaseOrders` 迁移为 `inboundLogs`；
    - 跳转页改为 `InboundLogsPage`；权限提示文案同步。
  - `app/lib/src/app.dart`
    - 注入控制器由 `PurchaseOrdersController` 切换为 `InboundLogsController`，并完成生命周期管理。

### 验证结果
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 当前结论
- Android 端“采购记录 -> 入库记录”迁移已完成 Flutter 主链路改造并通过静态检查。
- 本轮仅改客户端查询展示与装配，不涉及后端接口新增。

## 2026-03-12（入库提交按钮下沉到待提交明细区：文档+Web+Flutter 兼容修复收尾）

### 背景
- 用户确认按“两个阶段操作”的方向优化采购入库：
  1. 表单区仅负责录入与“加入明细/重置”；
  2. “提交入库”主动作下沉到“待提交入库明细”区域。
- 仓库规则要求先更新 PRD/架构/API 文档后开发；本轮续做重点为补齐 API 文档、落地 Web 页面并完成检查收尾。

### 本轮完成
- API 文档补齐（第三份文档）
  - `API接口定义文档.md`
  - 版本升级：`v1.2.25 -> v1.2.26`。
  - 新增章节：`14. 入库提交按钮位置优化兼容约定（本次新增）`，明确：
    - 表单区不再承担提交动作；
    - 提交入口位于待提交明细区；
    - 明细为空时按钮禁用与提示文案；
    - 不新增后端接口，不改权限/错误码/幂等语义。

- Web 入库页下沉提交按钮
  - `client/src/pages/InboundPage.vue`
  - 关键改动：
    - 表单容器从 `<form @submit.prevent="submit">` 调整为普通容器，移除表单区“提交入库”；
    - 在“待提交入库明细”卡片底部新增主提交按钮；
    - 提交禁用条件统一为：`loading || !canSubmit || getEffectiveDraftItems().length === 0`；
    - 新增状态提示：
      - 空明细：`请先加入明细，再提交入库。`
      - 非空：`请核对明细后提交入库。`

- Flutter 兼容修复（由分析检查暴露）
  - `app/lib/src/core/widgets/brand_ui.dart`
    - `SectionCard` 新增可选参数 `footer`，并在卡片底部渲染。
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
    - 修复 lint：`SectionCard` 调用中将 `child` 调整为最后一个命名参数（`sort_child_properties_last`）。

### 验证结果
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 当前结论
- 入库提交按钮下沉方案已完成“文档 -> Web 实现 -> Flutter 兼容修复 -> 双端检查”全链路收尾。
- 本轮仅调整前端/客户端交互与组件兼容，不涉及后端 API 契约行为变化。

## 2026-03-12（方案A续做：采购记录只读分页查询，后端 + Flutter 落地）

### 背景
- 用户要求继续中断任务并直接执行方案A，硬约束：
  1. 采购记录每页默认 10 条，可调整；
  2. 支持日期范围筛选，默认当天；
  3. 采购订单不提供修改能力（只读查询）；
  4. 采购记录尽可能详细。

### 本轮完成
- 后端（本轮之前已完成并通过编译）
  - `server/src/routes/mod.rs`：`/purchase-orders` 增加 `GET` 路由（保留 `POST`）。
  - `server/src/routes/purchase_orders.rs`：新增 `list_purchase_orders`。
    - 权限：`OWNER/PURCHASER`
    - 默认 `page_size=10`（可调，clamp 1~100）
    - `start_date/end_date` 默认当天，且要求成对传参
    - 返回字段：`start_date/end_date/list/total/page/page_size`
  - `server/src/repository.rs`：新增 `list_purchase_orders_by_tenant`（Provider + Postgres 实现）。

- Flutter 数据与查询链路（本轮完成）
  - `app/lib/src/features/inventory/models/inventory_models.dart`
    - 新增：`PurchaseOrderItemData / PurchaseOrderData / PurchaseOrdersPageData`。
  - `app/lib/src/features/inventory/models/inventory_repository.dart`
    - 新增 `fetchPurchaseOrders(...)` 对接 `GET /purchase-orders`。
  - `app/lib/src/features/inventory/application/purchase_orders_controller.dart`
    - 新增控制器：日期/分页校验、加载状态、错误人性化。
  - `app/lib/src/features/inventory/presentation/purchase_orders_page.dart`
    - 新增页面：
      - 默认当天日期范围
      - 每页默认 10，可手动调整
      - 分页上一页/下一页
      - 只读小票式详细展示（单头 + items + 金额 + 状态 + 版本 + 时间 + 备注）。

- Flutter 入口注入（本轮完成）
  - `app/lib/src/features/dashboard/presentation/dashboard_page.dart`
    - 新增快捷入口“采购记录”（`OWNER/PURCHASER`）。
    - 跳转 `PurchaseOrdersPage`，初始参数默认当天 + 每页 10。
  - `app/lib/src/app.dart`
    - 注入并管理 `PurchaseOrdersController` 生命周期（创建/释放/传递）。

### 验证结果
- `dart analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- `cargo check --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml` ✅

### 当前结论
- 采购记录只读分页查询已完成后端与 Flutter 端到端落地，满足方案A四条硬约束。
- 当前改造未新增采购订单修改能力，查询链路仅提供只读展示。

## 2026-03-12（v1.2.24 出/入库明细展示增强：Web + Android 双端落地）

### 背景
- 用户确认按“同商品多次扫码后，编辑应统一修改进价”的方向落地。
- 约束：仅做客户端展示/编排增强，不新增后端接口，不改权限/错误码/幂等语义。

### 本轮完成
- Web 入库页：`client/src/pages/InboundPage.vue`
  - 待提交明细新增“商品名称”列展示 `product_name`；
  - 明细行 `unit_cost` 改为可编辑输入框（行级编辑）；
  - 空行判定纳入 `product_name`；
  - 保持提交 payload 与后端契约不变（`unit_cost` 仍按原字段透传，可空）。

- Android 出库控制器：`app/lib/src/features/inventory/application/outbound_controller.dart`
  - `OutboundFormItemInput` 新增 `productName` 字段与 `copyWith` 支持；
  - 扫码新增/合并时持久化商品名称；
  - 有效行过滤纳入 `productName`。

- Android 出库页：`app/lib/src/features/inventory/presentation/outbound_page.dart`
  - 明细卡新增“商品名称”输入/展示；
  - 扫码确认弹窗收敛为“数量 + 销售单价”，移除行级“版本”输入；
  - 提交组包时行级 `expectedVersion` 固定空，保留单据级版本输入；
  - 扫码合并时同步补齐 `productName`。

- Android 入库页：`app/lib/src/features/inventory/presentation/inbound_page.dart`
  - 待提交明细展示 `productName`；
  - 新增行级 `unitCost` 编辑入口（卡片内输入框，提交回车写回）；
  - 增加 `onUpdateUnitCost` 回调与金额格式校验。

### 验证结果
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 当前结论
- v1.2.24 约束下的“出库明细商品名称展示 + 入库待提交商品名称与进价行级编辑”已在 Web/Android 双端完成落地，且静态检查通过。

## 2026-03-12（入库多商品明细池 + batch 提交、出库弹窗校验收尾）

### 背景
- 用户已确认从“排查”进入“实施”阶段，目标是双端收敛：
  1. 入库支持多商品明细暂存并按条数走单条/批量提交；
  2. 出库 `scan_confirm` 必须是弹窗交互（不保留内联确认区）。

### 本轮完成
- Android 入库页完成“明细池 + 编排提交”落地：
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
  - 新增明细池数据结构与 UI（待提交明细列表、删除能力）；
  - 扫码确认/连续扫码不再直接覆盖单表单，而是写入明细池并合并同商品数量；
  - 提交策略落地：`>=2` 条走 `submitBatch`，`=1` 条走 `submit`，`=0` 条回退旧单表单提交；
  - 新增批量结果展示（`batchResult`）并保留单条结果展示。
- Android 入库控制器补齐扫码返回商品名用于明细池展示：
  - `app/lib/src/features/inventory/application/inbound_controller.dart`
  - `InboundScanResult` 新增 `productName` 字段，并在 `scanAndAccumulate/applyScanConfirmed` 填充。
- Android 出库页 lint 收尾：
  - `app/lib/src/features/inventory/presentation/outbound_page.dart`
  - 修复 `use_build_context_synchronously`（`await` 后补 `if (!mounted) return;`）。
- Web 入库页类型检查收尾：
  - `client/src/pages/InboundPage.vue`
  - 修复 `effectiveItems[0]` 可能 `undefined` 的 TS 报错（非空断言）。

### 验证结果
- `npm --prefix /Users/admin/Documents/Projects/jxc/client run typecheck` ✅
- `flutter analyze` ✅（No issues found）

### 当前结论
- 本轮目标已完成：
  - App 入库链路已具备多商品明细池与 batch 编排能力；
  - 出库 `scan_confirm` 弹窗化在 Android 端可用且静态检查通过；
  - Web/App 双端检查已通过。

## 2026-03-12（扫码模式收敛为两种：Android 收尾完成）

### 背景
- 用户确认将扫码模式从三种收敛为两种：
  - 保留：`scan_confirm`、`continuous_scan`
  - 移除：`quick_accumulate`
- 兼容要求：读取到历史值 `quick_accumulate` 时，客户端自动映射为 `continuous_scan`，并建议写回本地偏好。

### 本轮完成
- 文档侧复核完成：`产品需求文档.md`、`架构设计文档.md`、`API接口定义文档.md` 已包含“两模式 + 历史偏好迁移”约束，本轮无需新增文档改动。
- Android 入库页收敛完成：
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
  - 枚举收敛为 `scanConfirm/continuousScan`；
  - 删除 quick 分支与 UI 入口；
  - `_modeFromStorage` 增加 `quick_accumulate -> continuous_scan` 映射；
  - `_restoreStoredScanMode` 在检测到旧值时自动写回规范值。
- Android 出库页收敛完成：
  - `app/lib/src/features/inventory/presentation/outbound_page.dart`
  - 枚举收敛为 `scanConfirm/continuousScan`；
  - 默认模式调整为 `scanConfirm`；
  - 删除 quick 分支与 UI 入口；
  - `_modeFromStorage` 增加 `quick_accumulate -> continuous_scan` 映射；
  - `_restoreStoredScanMode` 在检测到旧值时自动写回规范值。
- 与前序改动对齐确认：Windows 端 `InboundPage.vue/OutboundPage.vue` 与 API 文档此前已完成两模式改造，本轮未回退。

### 验证结果
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 当前结论
- 扫码模式“两模式收敛（含历史偏好迁移）”已完成 Windows + Android 双端闭环。
- 本轮仅涉及客户端交互与本地偏好迁移，不涉及后端 API、权限、错误码与幂等语义变更。

## 2026-03-12（Android 订单下钻默认每页 10 条：代码收尾完成）

### 背景
- 用户新增需求：**订单下钻页面的每页条数默认设置为 10**。
- 已遵循仓库规则完成文档先行（PRD/ADD/API 已升级至 `v1.2.20`），本轮进入 Flutter 代码收尾与验证。

### 本轮完成
- 入口默认值已固定为 10（前序已完成）：
  - `app/lib/src/features/dashboard/presentation/dashboard_page.dart`
  - 看板点击“今日订单数”进入下钻时，`initialPageSize` 固定传 `10`。
- 下钻页回退逻辑统一到 10（本轮完成）：
  - `app/lib/src/features/dashboard/presentation/dashboard_orders_page.dart`
  - 新增常量：`_defaultOrdersDrilldownPageSize = 10`
  - 新增方法：`_resolvePageSizeOrDefault(String raw)`
    - 输入为空/非法/`<=0` 时回退 `10`
  - `_query`、`_gotoPage`、`build` 中的 `pageSize` 解析全部改为复用该方法，移除 `?? 20` 分散回退。

### 验证结果
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅
- 输出：`No issues found!`

### 当前结论
- Android 看板订单下钻已实现：
  - 首次进入默认每页 `10` 条；
  - 用户手动输入每页条数时，空值/非法值/非正数统一回退 `10`。
- 服务端通用默认分页参数未变（仍保持接口兼容），本次仅调整 Android 客户端交互默认策略。

## 2026-03-12（App 日期输入统一日历选择 + 默认当天：收尾完成）

### 背景
- 用户需求：**app 中所有需要填日期的地方，改为日历选择，默认当天日期**。
- 本仓库规则要求“文档先行”，已先完成 PRD/架构/API 文档更新，再进行 Flutter 改造。

### 本轮完成
- 文档先行已完成（版本统一至 `v1.2.19`）：
  - `产品需求文档.md`：新增 `4.7.5 App 日期输入统一日历选择与默认当天`
  - `架构设计文档.md`：新增 `7.2.J Android 日期输入日历化策略`
  - `API接口定义文档.md`：新增 `5.1.D App 日期输入日历化兼容约定`
- Flutter 日期交互改造已完成：
  - `app/lib/src/features/dashboard/presentation/dashboard_page.dart`
    - 查询日期输入改为只读 + 日历选择；默认当天；点击输入框/图标均可触发 `showDatePicker`。
  - `app/lib/src/features/dashboard/presentation/dashboard_orders_page.dart`
    - 开始/结束日期改为只读 + 日历选择；默认当天；区间边界自动修正（开始晚于结束或结束早于开始时自动同步）。
- 日期格式与接口契约保持不变：
  - 仍按 `YYYY-MM-DD` 传参；不新增字段，不改后端路径/权限/错误码/幂等语义。
- 分析阶段收尾修复：
  - 修复了 `showDatePicker` 参数的 const/非const兼容问题；
  - 修复输入框 `OutlineInputBorder` 的性能提示（`prefer_const_constructors`）。

### 验证结果
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅
- 输出：`No issues found!`

### 当前结论
- App 现有“需要填日期”的入口（看板单日期 + 订单下钻日期范围）已全部日历化并默认当天，任务已完成并通过静态检查。

## 2026-03-12（“仍无明细”线上排查与部署校正）

### 用户反馈
- 用户在 Android 看板订单下钻中仍看到“无明细（聚合行）”。

### 排查结论
- 本地仓库代码已是修复版：`server/src/routes/reports.rs` 对 `OUTBOUND_ONLY` 使用 `items: synthetic_items` 回填。
- 线上服务器当时实际代码仍是旧逻辑：`items: Vec::new()`（已通过 SSH 直连 `/projects/jxcServer/src/routes/reports.rs` 复核）。
- 同时数据库实证存在可回填数据：
  - `stock_logs` 中目标 `biz_no` 有 `OUT_SALE` 记录、`snapshot_sell_price` 非空；
  - 且对应 `sales_orders` 主记录缺失（典型 `OUTBOUND_ONLY` 聚合场景），应返回聚合明细而非空数组。

### 已执行处置
- 执行远端发布脚本：
  - `RUN_SMOKE_AUTH_REGISTER=1 bash /Users/admin/Documents/Projects/jxc/server/scripts/deploy_remote.sh`
- 发布结果：
  - 远端 `cargo test` 124/124 通过；
  - `cargo build --release` 成功；
  - `jxc-server.service` 重启后 `active (running)`；
  - 健康检查与 auth/register 冒烟通过。
- 发布后复核：
  - 线上 `reports.rs` 已更新为 `items: synthetic_items`。

### 当前判断
- “仍无明细”根因是**线上运行版本落后于修复版本**，不是当前主干代码逻辑缺失。
- 现网已完成代码与服务切换，需用户侧重新查询验证结果。

## 2026-03-12（OUTBOUND_ONLY 聚合单明细回填收尾）

### 背景
- 用户持续反馈“订单列表没有显示每种商品的名称、价格、数量”。
- 关键表现为看板订单下钻中的历史聚合行（`OUTBOUND_ONLY`）常出现 `items=[]`，前端只能显示“无明细（聚合行）”。

### 本轮完成
- 后端逻辑复核确认：
  - `server/src/routes/reports.rs` 已按 `(biz_no, product_id, sell_price)` 对聚合行回填 `items`；
  - 有效售价口径为 `snapshot_sell_price` 优先，缺失时使用 `(biz_no, product_id)` 映射兜底。
- 自动化测试补强：
  - `server/src/routes/tests/business_flow.rs`
  - 在 `dashboard_orders_drilldown_should_align_with_dashboard_total_orders_and_support_paging()` 新增 `OUT-LEGACY-DRILL` 断言，确保：
    - `status=OUTBOUND_ONLY`
    - `items` 非空
    - 关键字段（`product_name/qty/sell_price/line_amount/returned_qty`）值正确。
- 契约与示例同步：
  - `API接口定义文档.md`：将 `OUT-LEGACY-DRILL` 示例由 `items: []` 改为带明细示例；
  - `server/openapi.yaml`：
    - `info.version` 升级到 `1.1.2`；
    - `/api/v1/reports/dashboard/orders` 描述补充“聚合行优先回填 items，仅无法聚合时允许空数组”。

### 验证结果
- 目标测试：
  - `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml dashboard_orders_drilldown_should_align_with_dashboard_total_orders_and_support_paging 2>&1` ✅
  - 结果：`1 passed; 0 failed`
- 全量测试：
  - `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml 2>&1` ✅
  - 结果：`124 passed; 0 failed`

### 当前结论
- `OUTBOUND_ONLY` 聚合行在可聚合场景下已能返回并展示商品明细，用户关注的“名称/价格/数量缺失”问题完成后端与契约闭环。
- 语义与回归测试已对齐，风险点已纳入自动化断言覆盖。

## 2026-03-12（Android 订单下钻字段精简展示收尾）

### 背景
- 用户提出 Android 订单下钻页展示应进一步精简：
  - 每个订单仅展示 `商品名称 / 商品数量 / 商品单价`；
  - 时间仅保留 `创建时间`。

### 本轮完成
- 文档先行对齐（`v1.2.17`）：
  - `产品需求文档.md`：新增 `4.13.6 Android 订单下钻字段精简展示`。
  - `架构设计文档.md`：更新 `7.2.I`，明确 Android 不显示 `line_amount` 与 `updated_at`。
  - `API接口定义文档.md`：更新 `5.1.C`，补充 Android 渲染侧字段收敛约定。
- Flutter 展示层落地：
  - `app/lib/src/features/dashboard/presentation/dashboard_orders_page.dart`
  - 明细表头与行渲染从 4 列收敛为 3 列：`product_name / qty / sell_price`；
  - 订单尾时间仅展示 `created_at`，移除 `updated_at` 文本渲染。

### 验证结果
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 当前结论
- Android 看板订单下钻列表已按用户最新要求完成字段精简展示。
- 本轮仅改文档与 Flutter `presentation` 层，后端接口字段保持兼容返回（`line_amount`、`updated_at` 仍可返回供跨端使用）。

## 2026-03-12（Android 看板订单下钻列表小票化修复）

### 背景
- 用户反馈 Android 端“订单下钻列表还是旧样式”，未达到“超市小票”展示效果。

### 本轮完成
- 文档先行对齐（v1.2.16）：
  - `产品需求文档.md`：`4.13.5` 明确订单列表小票化约束同时适用于 Windows + Android。
  - `架构设计文档.md`：新增 `7.2.I Android 订单下钻列表小票化展示策略`。
  - `API接口定义文档.md`：在 `5.1.C` 增补 Android 小票化展示约定。
- Flutter 数据模型增强：
  - `app/lib/src/features/dashboard/models/dashboard_data.dart`
  - `SalesOrderItemData` 新增并解析：`productName`、`lineAmount`；
  - 增加 `line_amount` 缺失兜底（`qty * sell_price` 计算，保留 4 位小数）。
- Flutter 下钻页面小票化渲染：
  - `app/lib/src/features/dashboard/presentation/dashboard_orders_page.dart`
  - 订单列表由摘要 `ListTile` 改为“每单小票卡片”：
    - 订单头：`biz_no`、`id`、`status`
    - 明细区：`product_name`、`qty`、`sell_price`、`line_amount`
    - 订单尾：`created_at`、`updated_at`、`remark`
  - 聚合行兼容：`items=[]` 时展示“无明细（聚合行）”，保留整单信息。

### 验证结果
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 当前结论
- Android 看板订单下钻页已完成小票化展示落地，与 Windows 端口径保持一致。
- 本轮仅涉及文档与 Flutter 展示层，不变更后端接口、权限与错误码语义。

## 2026-03-12（按需重新编译 Android arm64 APK）

### 背景
- 用户最新指令：`编译arm64的apk`。

### 本轮完成
- 执行命令：
  - `cd /Users/admin/Documents/Projects/jxc/app && flutter build apk --release --target-platform android-arm64 --split-per-abi`
- 构建结果：
  - `Running Gradle task 'assembleRelease'...`
  - `✓ Built build/app/outputs/flutter-apk/app-arm64-v8a-release.apk (24.0MB)`

### 产物
- 绝对路径：
  - `/Users/admin/Documents/Projects/jxc/app/build/app/outputs/flutter-apk/app-arm64-v8a-release.apk`

### 当前结论
- arm64 release APK 已成功重新编译，可直接安装验证。

## 2026-03-12（销售单下钻订单列表小票化展示收尾）

### 背景
- 用户补充要求：不仅“最近一次销售单结果”要小票化，**订单下钻列表**也必须按“超市小票”形式展示。
- 已按流程完成文档先行（PRD/架构/API 均已升级到 `v1.2.15` 并补充订单列表小票化约定）。

### 本轮完成
- `client/src/pages/SalesOrdersPage.vue`
  - 将“订单下钻列表”从摘要表格改为“每单小票卡片”渲染：
    - 订单头：`biz_no`、`id`、`status`、`total_amount`
    - 明细区：`product_name`、`qty`、`sell_price`、`line_amount`
    - 订单尾：`created_at`、`updated_at`、`remark`
  - 兼容聚合行：当 `items=[]`（如 `OUTBOUND_ONLY`）时展示“无明细（聚合行）”，但保留头尾与备注信息。
- `client/src/style.css`
  - 新增订单小票卡片样式：
    - `.order-receipt-list/.order-receipt-card`
    - `.order-receipt-head/.order-status-badge/.order-receipt-total`
    - `.order-empty-note/.order-receipt-foot/.order-receipt-empty`
  - 增加移动端对齐适配与小票表格换行策略。

### 验证结果
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅

### 当前结论
- 销售单页下钻列表已完成“小票化展示”落地，且不涉及后端接口、权限、错误码与幂等语义变更。

## 2026-03-12（订单明细商品名称快照口径：后端收尾完成）

### 背景
- 延续“订单明细名称快照固化”任务收尾，目标是确保历史订单展示不受商品改名影响。
- 业务口径：`product_name_snapshot > 商品主数据映射名称 > 商品#<product_id>`。

### 本轮完成
- `server/src/routes/common.rs`
  - `to_purchase_order_data` / `to_sales_order_data` 已切换为“快照优先”回退链路。
- `server/src/routes/tests/business_flow.rs`
  - 补齐 6 处 `SalesOrderItem` 初始化的 `product_name_snapshot` 字段，消除编译缺口。
- `server/openapi.yaml`
  - `PurchaseOrderItemData.product_name`、`SalesOrderItemData.product_name` 描述更新为“快照优先 + 映射兜底 + 商品#id 最终兜底”。

### 验证结果
- `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml` ✅
  - 结果：`124 passed; 0 failed`
- 全量结构体缺字段扫描 ✅
  - `SalesOrderItem` / `PurchaseOrderItem` 已无 `product_name_snapshot` 缺失实例。

### 当前结论
- 订单明细名称快照策略已在模型-仓储-路由-响应-契约-测试全链路闭环。
- API 路径、参数、权限、错误码、幂等语义保持不变。

## 2026-03-12（订单小票化收尾：后端 Handler 编译阻塞修复 + 校验通过）

### 背景
- 延续“订单小票化展示增强”收尾阶段，前端展示与 OpenAPI 已基本完成，但在校验阶段出现 Rust/Axum 编译阻塞：
  - `create_purchase_order` / `create_sales_order` 不满足 `Handler<_, _>`。

### 根因
- `create_purchase_order` 与 `create_sales_order` 的 in-memory 分支中，`MutexGuard`（`next_*_id` 与 `*_orders`）跨越了后续 `await`（调用 `to_*_data_with_product_names(...).await`）。
- 这会导致 handler future 非 `Send`，进而触发 Axum `Handler` 约束报错。

### 本轮修复
- `server/src/routes/purchase_orders.rs`
  - 重构 `create_purchase_order` 的 in-memory 分支：
    - 用独立代码块提前完成 ID 分配与订单入表；
    - 确保所有 `MutexGuard` 在进入 `await` 前释放。
- `server/src/routes/sales_orders.rs`
  - 同步重构 `create_sales_order` 的 in-memory 分支，消除 `MutexGuard` 跨 `await`。
- 修复过程中临时加入过 `#[axum::debug_handler]` 仅用于定位，已在最终代码中移除。

### 验证
- 后端：`cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml` ✅（124 passed）
- 前端：`npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅

### 当前结论
- 订单小票化收尾阶段的核心阻塞（Axum Handler 编译错误）已解除；
- 前后端校验均通过，当前可进入最终交付总结阶段。

## 2026-03-11（Android 看板下钻“总数有值但列表仍空”深层兼容修复）

### 用户反馈
- 用户在安装 arm64 APK 后反馈“还是看不到订单列表”。

### 根因补充
- 之前仅兼容了 `List/Iterable/Map.values` 场景，但线上运行时返回体在部分路径下存在更深层包裹与字符串化 JSON 片段，导致：
  - `list` 可展示总数 `total`，但订单数组未被正确抽取；
  - 页面出现“区间共 N 单”但列表空。

### 本轮修复
- `app/lib/src/features/dashboard/models/dashboard_data.dart`
  - 新增深层提取与形状识别解析链路：
    - `extractMapsByShape(...)`：递归兼容 `Map/List/Iterable/String(JSON)` 多形态容器；
    - `looksLikeSalesOrderMap(...)` / `looksLikeSalesOrderItemMap(...)`：按字段形状识别订单与明细；
    - `toSalesOrderList(...)` / `toSalesOrderItemList(...)`：统一转换为强类型模型。
  - 下钻 `total` 兜底策略增强：当服务端 `total<=0` 且已解析到列表时，使用 `parsedList.length` 作为兜底展示，避免“有列表但总数显示 0”。

### 验证
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- `flutter build apk --release --target-platform android-arm64 --split-per-abi` ✅
  - 产物：`/Users/admin/Documents/Projects/jxc/app/build/app/outputs/flutter-apk/app-arm64-v8a-release.apk`

### 结论
- Android 看板订单下钻解析链路已进一步增强，对深层包裹与字符串化 JSON 兼容更稳健；
- 已重新产出 arm64 安装包，可直接覆盖安装复测“订单列表可见性”。

## 2026-03-11（Android 看板下钻“当日筛选 total>0 但列表空”兼容性二次修复）

### 用户反馈
- 仍存在偶发不一致：开始/结束日期都为当天时，顶部显示“区间共 N 单”，列表却提示“当前筛选条件下暂无订单”。

### 根因补充
- Flutter 端下钻解析此前已从 `whereType<Map<String,dynamic>>()` 收敛为 `whereType<Map>()`，但仍仅在 `rawList is List` 时才进入解析；
- 在部分运行时路径下（动态 JSON 容器形态不稳定），`list/items` 可能以 `Iterable` 或 Map 包裹值出现，导致解析前被当作空集合处理，最终出现 `total` 有值而 `list` 为空。

### 本轮修复
- `app/lib/src/features/dashboard/models/dashboard_data.dart`
  - 新增 `toRawIterable(Object?)`：统一将 `List / Iterable / Map.values` 归一为可迭代对象；
  - 下钻 `list` 与订单 `items` 解析统一改为基于 `toRawIterable(...)` 再做 `Map<String,dynamic>.from(...)` 转换。

### 验证
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 结论
- Android 看板下钻解析链路对运行时 JSON 容器形态更健壮，`total/list` 展示一致性进一步提升。

## 2026-03-11（Android 看板下钻“total>0 但列表为空”修复）

### 用户反馈
- Android 下钻页出现不一致：顶部显示“区间共 2 单”，但列表区域显示“当前筛选条件下暂无订单”。

### 根因定位
- 后端 `GET /reports/dashboard/orders` 实际返回 `list` 与 `total` 口径一致。
- 问题在 Flutter 端 JSON 解析：
  - `dashboard_data.dart` 使用了 `.whereType<Map<String, dynamic>>()` 过滤 `list/items`；
  - `dio/jsonDecode` 返回的运行时类型常为 `LinkedHashMap<dynamic, dynamic>`，无法命中该类型过滤；
  - 导致 `list` 被错误过滤为空，而 `total` 仍正常解析，出现“total>0 且 list 空”。

### 本轮修复
- `app/lib/src/features/dashboard/models/dashboard_data.dart`
  - 下钻订单列表解析：
    - `whereType<Map<String, dynamic>>()`
    - -> `whereType<Map>() + Map<String,dynamic>.from(item)`
  - 订单明细 items 解析同样改造，确保运行时 Map 兼容。

### 验证
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 结论
- Android 看板下钻“总数有值但列表为空”已修复，列表与总数展示恢复一致。

## 2026-03-11（Android 看板“今日订单数点击无反应”闭环收尾）

### 用户反馈
- 用户反馈 Android 端经营看板点击“今日订单数”仍“没有反应”。

### 本轮处理
- 文档先行补充：
  - `API接口定义文档.md` 升级至 `v1.2.12`。
  - 在 `5.1.C` 补充 **Android 看板跳转联动约定**：
    - 默认参数：`start_date/end_date = dashboard.date`，`page=1`，`page_size=min(max(total_orders,1),100)`；
    - 权限：仅 `OWNER/SALES` 可下钻，`PURCHASER` 点击需即时提示；
    - 日期校验与失败提示需可感知。

- Flutter 下钻链路落地：
  - `app/lib/src/features/dashboard/models/dashboard_data.dart`
    - `DashboardData` 新增 `date` 字段；新增下钻数据模型。
  - `app/lib/src/features/dashboard/models/dashboard_repository.dart`
    - 新增 `fetchDashboardOrdersDrilldown(...)`，对接 `GET /reports/dashboard/orders`。
  - `app/lib/src/features/dashboard/application/dashboard_orders_controller.dart`
    - 新增下钻控制器（日期/分页校验、请求、错误人性化提示）。
  - `app/lib/src/features/dashboard/presentation/dashboard_orders_page.dart`
    - 新增订单下钻页面（筛选、分页、列表）。

- 点击无反应修复与接线收尾：
  - `app/lib/src/features/dashboard/presentation/dashboard_page.dart`
    - “今日订单数”指标卡支持整卡点击（`InkWell`）；
    - 点击后按角色做权限提示并跳转下钻页；
    - 跳转异常时提示“订单下钻跳转失败”；
    - 本轮补充类型安全：`_handleOrdersMetricTap(UserRole, DashboardData)`（移除 `dynamic`）。
  - `app/lib/src/app.dart`
    - 新增 `DashboardOrdersController` 创建、注入 `DashboardPage`、并在 `dispose` 释放。

### 验证
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 结论
- Android 端“今日订单数点击无反应”问题已形成完整闭环：
  - 文档约束（API）已补齐；
  - 点击反馈、权限提示、页面跳转与下钻查询链路均已落地；
  - 静态检查通过，可进入业务验收。

## 2026-03-11（看板“今日订单数点击无反应”交互修复收尾）

### 用户反馈
- 用户反馈点击经营看板“今日订单数”时仍感觉“没有反应”。

### 本轮处理
- `client/src/pages/DashboardPage.vue`
  - 将“订单数”卡片整体设为可点击（不再仅依赖数字文本点击热区）。
  - 保留数字按钮点击，并增加 `@click.stop`，避免事件冒泡重复触发。
  - 下钻跳转增加 `try/catch`，失败时回显“订单下钻跳转失败”具体提示。
  - 对无权限角色点击时明确反馈：`当前角色无销售单下钻权限，仅 OWNER / SALES 可查看`。
  - 卡片提示文案增强：
    - 有权限：`点击查看订单下钻`
    - 无权限：`仅 OWNER / SALES 可查看订单下钻`

### 验证
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅

### 结论
- 本轮为前端交互可用性修复，不涉及后端 API、权限规则与业务口径变更。
- 用户点击“今日订单数”时的反馈与可达性已增强，避免“看起来可点但无感知反馈”。

## 2026-03-11（看板订单下钻聚合行文档同步收尾）

### 用户目标
- 延续“点击今日订单数看不到具体订单列表”修复收尾，补齐尚未同步的文档链路，确保与已落地后端口径一致。

### 本轮完成
- `架构设计文档.md`
  - 版本升级到 `v1.2.10`。
  - 在 `6.9` 明确下钻返回按 `biz_no` 维度统一口径；
  - 明确无 `sales_orders` 主记录时必须返回聚合行（`status=OUTBOUND_ONLY`、`items=[]`、聚合 remark）。
- `API接口定义文档.md`
  - 版本升级到 `v1.2.10`。
  - 在 `5.1.C` 补充聚合行语义（`OUTBOUND_ONLY`）与字段约束；
  - 响应示例新增 `OUT-LEGACY-DRILL` 聚合行样例，并同步 `total` 示例值。
- `server/openapi.yaml`
  - 版本升级 `1.1.1`；
  - 新增 `/api/v1/reports/dashboard/orders` 路径定义，补齐参数与鉴权/错误码声明，语义与代码行为一致。

### 当前结论
- PRD / ADD / API / OpenAPI 已与当前代码实现完成一致性对齐：
  - 看板 `total_orders` 与下钻 `total/list` 同口径；
  - legacy `OUT-*` 无主订单场景通过聚合行可见，避免“有总数无列表”。
- 本轮仅文档与契约补齐，无新增服务端业务逻辑改动。

## 2026-03-11（销售出库看板统计口径方案A收敛）

### 用户目标
- 修复“销售出库后销售额、毛利润、订单数不变化”。
- 按既定方案A落地：`snapshot_sell_price` 优先，历史数据 `(biz_no, product_id)` 映射兜底，订单数按有效 `OUT_SALE` 唯一 `biz_no`（覆盖 `SO-*` + `OUT-*`）。

### 本轮核心改动
- `server/src/routes/reports.rs`
  - 新增 `resolve_effective_sell_price(log, sell_price_map)`：
    - 优先 `log.snapshot_sell_price`
    - 缺失时 fallback `sell_price_map[(biz_no, product_id)]`
  - `get_dashboard_report`
    - `OUT_SALE/RETURN_SALE` 均改为使用“有效售价”计算 `total_sales`
    - `total_cost` 与 `total_qty` 仅在存在有效售价时同口径累计
    - `total_orders` 按有效 `OUT_SALE` 唯一 `biz_no` 统计
  - `get_dashboard_orders_drilldown`
    - 有效订单判定改为“存在有效售价的 `OUT_SALE`”
    - `total` 改为 `sold_order_biz_nos.len()`，确保与 dashboard total_orders 对齐（可包含 legacy `OUT-*`）
  - `get_sales_report` / `export_sales_report_csv`
    - 销售额计算统一改为“快照优先 + fallback”
    - 成本/数量与销售额统一口径（仅有效售价日志参与）

- `server/src/routes/tests/business_flow.rs`
  - `dashboard_report_should_use_stock_log_aligned_metrics`
    - 毛利润断言由 `14.30` 调整为 `10.10`（纳入 `OUT-LEGACY-001` 的快照售价后，与成本同口径计算结果）
    - 注释更新为“legacy 出库有快照售价应计入”。

### 验证结果
- 执行：`cd /Users/admin/Documents/Projects/jxc/server && cargo test -q 2>&1`
- 结果：`124 passed; 0 failed` ✅

### 部署与健康检查
- 执行：`RUN_SMOKE_AUTH_REGISTER=1 bash /Users/admin/Documents/Projects/jxc/server/scripts/deploy_remote.sh`
- 结果：✅
  - 远端 `cargo test`：`124 passed; 0 failed`
  - 远端 `cargo build --release`：完成
  - `jxc-server.service`：`active (running)`
  - 健康检查：`http://127.0.0.1:8080/health` 返回 200
  - auth/register 冒烟：通过

### 当前结论
- 方案A在报表域已完成收敛：
  - 销售额/毛利不再遗漏仅存在 stock_log 快照的销售流水；
  - 看板订单数已覆盖 `SO-*` 与 legacy `OUT-*` 有效销售流水；
  - 看板与下钻 `total` 口径一致。
- 变更已部署到远端服务并通过健康检查与冒烟验证。

## 2026-03-11（入库提交后表单重置策略优化收尾）

### 用户诉求与结论
- 用户反馈“入库提交后默认值又变成 1”，并进一步确认“提交后清空商品 ID/条码是否更合理”。
- 最终策略：
  - 提交成功后清空商品上下文（`product_id/barcode` + 扫码输入）；
  - 数量恢复默认 `1`；
  - 其余可选字段按原规则清空。

### 文档先行（已完成）
- `产品需求文档.md`：已升级 `v1.2.8`，在 `4.3.4` 明确提交后清空商品上下文与数量默认值策略。
- `架构设计文档.md`：已升级 `v1.2.8`，在 `7.1.G / 7.2.G` 明确 Windows/Android 一致的重置策略。
- `API接口定义文档.md`：已升级 `v1.2.8`，在 `4.1.D` 明确该变更仅为客户端状态管理，不改后端契约。

### 代码落地（已完成）
- Windows（Vue）
  - `client/src/pages/InboundPage.vue`
  - `submit()` 成功后：
    - 清空 `form.product_id / form.barcode`
    - 重置 `form.qty='1'`
    - 清空 `unit_cost/expected_version/remark`
    - 调用 `resetScanForm()` 清空扫码会话上下文
- Android（Flutter）
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
  - `_submit()` 成功后：
    - `clear()` 商品ID与条码控制器
    - `_qtyController.text = '1'`
    - 清空 `unitCost/expectedVersion/remark`
    - 调用 `_resetScan()` 清空扫码会话上下文

### 验证结果
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 当前结论
- “提交后清空商品 ID/条码并重置数量为 1”的策略已在 Windows + Android 双端完成落地，且通过静态检查。
- 本轮不涉及后端 API、错误码、权限与幂等语义变更。

## 2026-03-11（看板订单下钻首屏可见性修复收尾）

### 用户反馈
- 用户反馈“点击今日订单数后，似乎还是无法查看今日所有订单”。

### 根因结论
- 后端口径与分页实现正常（`/reports/dashboard/orders` 默认 `page_size=20`，最大 `100`）。
- 问题出在 Windows 看板跳转参数：此前固定传 `page_size=20`，导致当日订单数大于 20 时首屏只展示 20 条，产生“看不到全部”的体验偏差。

### 代码修复（已完成）
- `client/src/pages/DashboardPage.vue`
  - `goToOrdersDrilldown()` 改为动态计算首屏页大小：
    - `page_size = min(max(total_orders, 1), 100)`
  - 跳转仍保持：`start_date=end_date=dashboard.date`，`page=1`。

### 文档同步（已完成）
- `产品需求文档.md`
  - 已更新到 `v1.2.7`，在 `4.7.3` 补充“看板下钻首屏 page_size 动态化（1~100）”。
- `架构设计文档.md`
  - 已更新到 `v1.2.7`；
  - 在 `6.9` 增补 Windows 联动约束：按 `total_orders` 动态设置首屏 `page_size`；
  - 新增 `7.1.H Windows 看板订单下钻首屏可见性策略`。
- `API接口定义文档.md`
  - 已更新到 `v1.2.7`；
  - 在 `5.1.C` 增补“Windows 看板跳转联动约定”：`page_size=min(max(total_orders,1),100)`，超 `100` 走分页。

### 验证与结论
- 前序已完成前端类型检查：`npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅。
- 结论：问题属前端首屏分页参数策略，已修复并完成 PRD/ADD/API 与 memory-bank 同步。

## 2026-03-11（入库 scan_confirm 确认写入弹窗化：Windows + Android 双端落地）

### 用户目标
- 将入库页 `scan_confirm` 从“页面内联确认区”改为“弹窗确认”。
- 保持“确认后自动续扫”，减少重复点击“按条码处理”。
- Windows 支持 `Enter` 确认、`Esc` 取消；Android 支持回车快速确认。

### 文档基线
- 已沿用本轮前序完成的 `v1.2.6` 文档约束：
  - `产品需求文档.md`：`4.3.4 入库确认写入弹窗化`
  - `架构设计文档.md`：`7.1.G / 7.2.G`
  - `API接口定义文档.md`：`4.1.D`
- 结论：本次仅改客户端 Presentation 交互，不改后端 API 契约。

### 本轮代码改造（已完成）
- Windows（Vue）
  - `client/src/pages/InboundPage.vue`
    - 新增弹窗确认状态与方法：`openScanConfirmDialog / cancelScanConfirmDialog / clearScanConfirmDraft`。
    - `scan_confirm` 命中商品后改为打开弹窗，不再渲染内联确认区。
    - 数量输入默认 `1`，弹窗打开后自动聚焦并选中。
    - 键盘支持：`Enter` 确认写入，`Esc` 取消确认。
    - 确认成功后保持确认续扫会话、计数递增、清空条码并回焦扫码输入框。
  - `client/src/style.css`
    - 新增 `modal-backdrop / modal-card` 弹窗样式。

- Android（Flutter）
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
    - 删除 `_showScanConfirm` 内联确认区块与 `_ScanCard` 对应参数。
    - 新增 `_showScanConfirmDialog()`，用 `AlertDialog` 承载确认输入。
    - 弹窗数量输入支持 `onSubmitted` 回车确认；确认按钮复用 `_applyScanConfirm()`。
    - 取消时关闭弹窗并回焦扫码框，提示“可继续扫码”。
    - 确认成功后保持确认续扫会话、计数递增、清空扫码输入并回焦。

### 验证结果
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 当前结论
- 入库 `scan_confirm` 弹窗化已在 Windows + Android 双端完成落地，并保持“确认后自动续扫”会话语义。
- 本轮未修改后端接口、错误码、权限与幂等语义。

## 2026-03-11（扫码入库首扫数量异常 +1：Web/Android 同步修复）

### 用户反馈
- 入库扫码时，某商品只扫 1 次，数量却直接变成 `2`。

### 根因定位
- Web 与 Android 的入库扫码累加逻辑都采用：
  - 读取表单当前数量（默认初始值为 `1`）
  - 再执行 `当前数量 + 本次扫码数量(1)`
- 因此“首扫”在默认数量为 `1` 的情况下被算成了 `2`。

### 修复方案（已完成）
- 统一改为“首扫重置基数”策略：
  - 若当前表单尚未绑定商品（`product_id/barcode` 均为空），首扫以 `0` 为基数再累加。
  - 若已绑定同一商品，仍按原逻辑继续累加。
- 代码变更：
  - `client/src/pages/InboundPage.vue`
    - `applyScannedInbound`：新增 `baseQty`，首扫时 `baseQty=0`。
  - `app/lib/src/features/inventory/application/inbound_controller.dart`
    - `_applyScannedToForm`：新增 `baseQty`，首扫时 `baseQty=0`。

### 验证结果
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 当前结论
- 入库扫码“首扫数量变 2”问题已在 Web 与 Android 双端修复；当前行为为：首扫=1，后续同商品扫码继续递增。

## 2026-03-11（经营看板订单数下钻：联调收尾 + 远端部署完成）

### 用户目标
- 将“今日订单数可点击下钻，默认当天销售订单，并支持日期范围筛选”完整落地并可上线。

### 本轮实现状态（已完成）
- 后端路由挂载
  - `server/src/routes/mod.rs`
  - 新增：`GET /reports/dashboard/orders` -> `reports::get_dashboard_orders_drilldown`
- 后端口径与分页测试补齐
  - `server/src/routes/tests/business_flow.rs`
  - 新增：`dashboard_orders_drilldown_should_align_with_dashboard_total_orders_and_support_paging`
  - 覆盖点：
    - 看板 `total_orders` 与下钻 `total` 同口径；
    - 仅统计可映射销售明细的有效 `OUT_SALE`；
    - 无有效流水销售单不计入；
    - `page/page_size` 分页有效。
- 前端（Windows）下钻链路联通
  - `client/src/types/api.ts`：新增下钻 Query/Data 类型；
  - `client/src/api/inventory.ts`：新增 `getDashboardOrdersDrilldownApi`；
  - `client/src/pages/DashboardPage.vue`：订单数可点击并携带当天日期跳转销售单页；
  - `client/src/pages/SalesOrdersPage.vue`：新增下钻区块、日期范围筛选、分页与 query 初始化逻辑。

### 验证与部署
- 本地验证
  - `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml routes::tests::business_flow -- --nocapture` ✅
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
- 远端部署（`server/scripts/deploy_remote.sh`）✅
  - 远端 `cargo test`：`124 passed; 0 failed`
  - 远端 `cargo build --release`：完成
  - `systemd` 服务重启：`jxc-server.service active (running)`
  - 健康检查：`http://127.0.0.1:8080/health` 返回 200

### 当前结论
- “经营看板订单数下钻（默认当天 + 日期范围筛选）”已完成文档对齐、代码落地、测试通过与远端部署，可进入业务验收。

## 2026-03-11（经营看板全指标口径修复收尾）

### 用户目标
- 用户反馈“除今日订单数外，其它看板指标也可能不准”，要求对 `GET /reports/dashboard` 全指标做口径排查与修复。

### 本轮后端实现状态（已完成）
- `server/src/routes/reports.rs`
  - 看板统计统一基于当日销售流水（`OUT_SALE/RETURN_SALE`）计算销售额、毛利、热销。
  - 新增“销售单明细可映射”约束：仅当 `(biz_no, product_id)` 能命中 `sales_orders.items` 的 `sell_price` 才计入统计。
  - `total_orders` 改为“当日有效 `OUT_SALE` 且可映射流水”的唯一 `biz_no` 数量，不再按 `confirmed_at` 直接计数。
- `server/src/routes/tests/business_flow.rs`
  - 新增 `dashboard_report_should_use_stock_log_aligned_metrics`，覆盖：退货冲减、低库存、热销、已确认无流水订单不计数、遗留流水忽略。

### 文档对齐（已完成）
- `产品需求文档.md` 升级至 `v1.2.4`，新增 `4.7.2 经营看板统计口径统一`。
- `架构设计文档.md` 升级至 `v1.2.4`，新增 `6.8 经营看板统计口径约束`。
- `API接口定义文档.md` 升级至 `v1.2.4`，新增 `5.1.B 经营看板统计口径统一约定`，并补充响应 `data.date` 字段示例。

### 验证结果
- `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml routes::tests` ✅（109 passed）
- `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml` ✅（123 passed）

### 当前结论
- 经营看板 `total_sales / total_gross_profit / total_orders / low_stock_count / top_selling_item` 口径已统一并通过集成测试与全量测试回归验证。

## 2026-03-11（扫码模式 Tab 化 + 主按钮智能摄像头触发：双端收尾）

### 用户目标
- 落地“扫码模式从下拉改为并排 Tab/分段切换”的交互优化。
- 保持文案语义一致：主按钮是“按条码处理”，摄像头入口才使用“扫码”。
- Android 端补充主按钮智能触发：
  - 在 `quick_accumulate / scan_confirm` 下，若条码输入为空，点击主按钮应直接拉起摄像头；
  - `continuous_scan` 保持原“显式开启会话”行为。

### 本轮代码完成
- Web（Windows / Vue）
  - `client/src/pages/InboundPage.vue`：扫码模式已由下拉改为并排 Tab 按钮。
  - `client/src/pages/OutboundPage.vue`：扫码模式已由下拉改为并排 Tab 按钮。
  - `client/src/style.css`：新增 `.scan-mode-tabs / .scan-mode-tab / .scan-mode-tab.is-active` 统一样式。

- Android（Flutter）
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
    - 扫码模式控件 `DropdownButtonFormField` -> `ChoiceChip + Wrap`。
    - 新增 `_onPrimaryScanPressed()`：非连续模式且条码为空时优先调用 `_scanWithCamera()`。
    - 扫码处理入口增加 `trim()`，减少空白输入干扰。
  - `app/lib/src/features/inventory/presentation/outbound_page.dart`
    - 扫码模式控件 `DropdownButtonFormField` -> `ChoiceChip + Wrap`。
    - 抽取 `_onScanModeChanged()` 统一模式切换状态收敛与持久化。
    - 新增 `_onPrimaryScanPressed()`：非连续模式空条码时优先拉起摄像头。
    - 扫码处理入口增加 `trim()`。

### 一致性与边界
- 扫码模式存储值保持不变：`quick_accumulate / scan_confirm / continuous_scan`。
- 仅变更前端/客户端交互与展示，不改后端 API、错误码、权限、幂等语义。

### 验证结果
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 当前结论
- “扫码模式 Tab 化 + Android 主按钮空输入智能拉起摄像头”已完成双端落地并通过静态检查。

## 2026-03-11（扫码模式记忆 + 确认后自动续扫：Android 收尾完成）

### 用户目标
- 在已完成 Windows 端落地基础上，继续完成 Android 端两项优化闭环：
  1. 记住用户上次选择的扫码模式（按用户作用域 + 页面隔离）。
  2. `scan_confirm` 模式下确认后默认继续扫码（会话化，无需重复点击“按条码处理”）。

### 本轮代码完成
- `app/lib/src/features/inventory/presentation/inbound_page.dart`
  - 接入 `SessionStorage` 扫码模式持久化：
    - 页面初始化读取 `readScanMode(scope, 'inbound')`
    - 模式切换写入 `writeScanMode(scope, 'inbound', mode)`
  - 新增确认续扫会话状态：
    - `_scanConfirmSessionActive`
    - `_scanConfirmProcessedCount`
    - `结束确认续扫` 操作
  - `scan_confirm` 确认后自动续扫：
    - 确认成功后清空扫码输入
    - 计数递增并提示“会话进行中”
    - 聚焦扫码输入框继续下一次扫码
  - 新增扫码枪回车触发处理（非连续模式）：`onSubmitted -> _scanByCurrentMode()`

- `app/lib/src/features/inventory/presentation/outbound_page.dart`
  - 接入 `SessionStorage` 扫码模式持久化：
    - 页面初始化读取 `readScanMode(scope, 'outbound')`
    - 模式切换写入 `writeScanMode(scope, 'outbound', mode)`
  - 新增确认续扫会话状态与“结束确认续扫”动作。
  - `scan_confirm` 确认写入明细后自动续扫：
    - 清空扫码输入、计数累加、提示会话状态
    - 自动聚焦扫码输入框
  - 新增扫码枪回车触发处理（非连续模式）。

- `app/lib/src/features/dashboard/presentation/dashboard_page.dart`
  - 为入库/出库页面注入：
    - `sessionStorage`
    - `scanPreferenceScope = "tenantId:userId"`

- `app/lib/src/app.dart`
  - 为 `DashboardPage` 注入 `sessionStorage`，打通依赖链。

### 一致性说明
- Flutter 与 Windows 端都采用相同模式值：
  - `quick_accumulate`
  - `scan_confirm`
  - `continuous_scan`
- 作用域一致：`tenantId:userId`。
- 本次仅改客户端本地存储与交互行为，不变更后端 API 契约。

### 验证结果
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅

### 当前结论
- “扫码模式记忆 + 确认后自动续扫”已在 Windows + Android 双端全部落地并通过静态检查。

## 2026-03-11（扫码文案与行为一致性收敛：Windows + Android）

### 用户目标
- 用户反馈“很多按钮写着『扫码并xx』，但实际并不会拉起扫码”，要求统一 review 并修正文案，保证“按钮名称”和“实际行为”一致。

### 文档约束（已对齐）
- 本轮代码改造遵循已补齐的文档规范：
  - `产品需求文档.md`：`4.11 扫码文案与行为一致性规范`
  - `架构设计文档.md`：`6.6 扫码文案与行为一致性约束`
  - `API接口定义文档.md`：`11. 扫码文案与行为一致性 API 兼容约定`
- 约束要点：仅摄像头入口使用“扫码”动词；处理输入框条码的动作改为“按条码…”。

### 本轮完成（Android / Flutter）
- `app/lib/src/features/inventory/presentation/inbound_page.dart`
  - `title: 扫码入库 -> 条码入库`
  - `subtitle: 扫码确认 -> 确认写入`
  - 输入框图标 `tooltip: 扫码并累加 -> 摄像头扫码`
  - 主按钮 `扫码并处理 -> 按条码处理`
- `app/lib/src/features/inventory/presentation/outbound_page.dart`
  - `title: 扫码出库 -> 条码出库`
  - `subtitle: 扫码确认 -> 确认写入`
  - 输入框图标 `tooltip: 扫码出库 -> 摄像头扫码`
  - 主按钮 `扫码并处理 -> 按条码处理`
- `app/lib/src/features/inventory/presentation/stock_check_page.dart`
  - `title: 扫码选品（创建盘点单） -> 条码选品（创建盘点单）`
  - `subtitle: 扫码命中后... -> 按条码命中后...`
  - 输入框图标 `tooltip: 扫码并加入明细 -> 摄像头扫码加入明细`
  - 主按钮 `扫码并加入明细 -> 按条码加入明细`

### 本轮复核与验证
- 残留检索：
  - `search_files(app/lib/src, regex="扫码并")` -> `0`
  - `search_files(client/src, regex="扫码并")` -> `0`
- 静态检查：
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 当前结论
- Windows 与 Android 两端“扫码并xx”误导文案已完成收敛；
- 仅做 Presentation 文案层改造，不涉及 API 契约、参数、错误码或权限变更。

## 2026-03-11（扫码入库未命中直达新建商品：Windows + Android 落地收尾）

### 用户目标
- 将“入库扫码未命中商品时可直接新建商品”的方案正式落地到 Windows 与 Android 双端。
- 严格遵循“文档先行”：先更新 PRD/架构/API 文档，再实施代码。

### 文档变更（已完成）
- `产品需求文档.md`：升级至 `v1.2.1`，新增“扫码未命中直达新建商品”需求条目。
- `架构设计文档.md`：升级至 `v1.2.1`，新增 Windows/Android 端流程编排策略。
- `API接口定义文档.md`：升级至 `v1.2.1`，明确沿用既有 `4040` 语义，不新增接口。

### 代码落地（已完成）
- Windows（Vue）
  - `client/src/pages/InboundPage.vue`
    - 扫码命中 `4040` 时可弹确认并跳转商品管理新建。
    - 支持从商品创建成功后回流入库页并自动回填 `product_id/barcode`。
  - `client/src/pages/ProductsPage.vue`
    - 支持接收入库来源参数（`from=inbound` + `barcode`）并预填创建表单。
    - 创建成功后携带 `created_product_id/created_barcode` 回跳入库页。

- Android（Flutter）
  - `app/lib/src/features/inventory/application/inbound_controller.dart`
    - 新增未命中条码暂存能力（`pendingMissingBarcode`），用于页面引导建档。
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
    - 未命中后弹窗引导新建商品；调用 `CreateProductSheet` 完成建档并回填入库表单。
    - 适配 `CreateProductSheet` 新参数 `moneyPattern`。
  - `app/lib/src/features/products/presentation/products_page.dart`
    - `CreateProductSheet` 对外可复用，新增 `key` 参数；取消时返回 `null` 以匹配 `ProductData?`。
  - `app/lib/src/features/dashboard/presentation/dashboard_page.dart`
    - 入库页面装配 `productController`，保证入库页可直接调用商品创建弹层。

### 本轮修复与验证
- 修复 Flutter 分析错误：`CreateProductSheet` 调用缺少必填 `moneyPattern`。
- 修复 BottomSheet 返回值类型不一致（取消时不再 `pop(false)`）。
- 验证通过：
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 当前结论
- “扫码入库未命中 -> 引导新建 -> 回填并继续入库”双端链路已完成并通过静态检查。
- 本次仅改前端/客户端编排，后端契约与错误码语义保持不变。

## 2026-03-11（第三方条码查询 + 缓存落库闭环收尾）

### 本轮目标
- 在已完成后端 `GET /products/barcode-lookup` 主链路（Cache-Aside + 负缓存 + 降级）的基础上，补齐剩余闭环：
  - 路由测试模块缺失文件修复并补测试；
  - Windows 前端接入 lookup 能力，实现新建商品自动带出名称；
  - 完成后端/前端回归验证；
  - 更新 Memory Bank 记录。

### 本轮完成
- 后端测试补齐：
  - 新增 `server/src/routes/tests/barcode_lookup.rs`，覆盖 3 个关键场景：
    1. 第三方未配置且无缓存 -> `DEGRADED`
    2. 有有效缓存 -> `CACHE`
    3. 仅有过期缓存且回源失败 -> `CACHE_STALE`
- 后端回归：
  - 执行 `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml`。
  - 结果：`122 passed; 0 failed`。
- 前端类型与 API 封装：
  - `client/src/types/api.ts` 新增 `BarcodeLookupData`。
  - `client/src/api/products.ts` 新增 `barcodeLookupProductNameApi(barcode)`。
- 前端页面接入：
  - `client/src/pages/ProductsPage.vue` 在 `scanBarcode()` 的 404 分支接入 lookup：
    - 先回填 `createForm.barcode`；
    - 调用 `/products/barcode-lookup`；
    - 若返回 `suggested_name`，自动回填 `createForm.name`；
    - 按 `source` 给出提示；
    - lookup 异常时保持原有兜底提示。
- 前端回归：
  - 执行 `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client`。
  - 结果：通过（`vue-tsc --noEmit`）。

### 当前结论
- “新建商品扫码未命中时，自动通过第三方条码查询带出建议名称”已完成代码闭环。
- 本轮未新增后端接口契约变更，仅完成既有 v1.2.0 方案的实现补齐与验证。

## 2026-03-11（经营看板日期筛选：默认当天 + 日历选择器）

### 用户目标
- 评估并落地经营看板按日期查询的交互优化：
  - 默认展示当天数据；
  - 支持通过日历控件筛选日期；
  - 保持后端接口契约不变。

### 文档先行更新（已完成）
- `产品需求文档.md`
  - 版本更新为 `v1.1.9`，新增 **4.7.1 经营看板日期筛选交互补充**。
- `架构设计文档.md`
  - 版本更新为 `v1.1.9`，新增 **7.1.C 经营看板日期筛选交互策略**。
- `API接口定义文档.md`
  - 版本更新为 `v1.1.9`，新增 **5.1.A 经营看板日期筛选交互约定**。

### 代码落地（已完成）
- `client/src/pages/DashboardPage.vue`
  - 日期输入控件由文本输入改为 `input[type=date]`（日历选择器）。
  - 保持页面初始化默认当天（`YYYY-MM-DD`）并自动查询。
  - 增加更严格的日期解析与合法性校验（防止无效日期）。
  - `重置为今天` 仍保留，回填当天并刷新查询。

### 回归结果
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅

### 当前结论
- “默认当天 + 日历筛选”已在 Windows 看板页面完成落地。
- 本次仅调整前端交互层，未变更 `GET /reports/dashboard` 契约、权限与错误码语义。

## 2026-03-11（入库/出库连续扫码会话模式双端落地收尾）

### 用户目标
- 将前面评审通过的三模式扫码方案（`quick_accumulate / scan_confirm / continuous_scan`）正式落地到：
  - Windows 端（Vue）：入库 + 出库
  - Android 端（Flutter）：入库 + 出库
- 且连续扫码模式需支持：会话持续、短时去重、成功反馈（页面提示 + 语音/声音提示）、手动结束。

### 文档状态
- PRD/ADD/API 已于本轮前置阶段升级到 `v1.1.8`，并明确连续扫码会话模式约束（不改后端契约）。

### 本轮完成（代码）
- Windows（Vue）
  - `client/src/pages/OutboundPage.vue`
    - 修复收敛问题：`OutboundScanMode` 补齐 `continuous_scan`；表单提交入口切换到 `scanByCurrentMode`。
    - 连续扫码会话可正常启动/持续扫码/结束统计，保留短时去重与语音提示。
  - `client/src/pages/InboundPage.vue`
    - 新增三模式切换（默认 `scan_confirm`）。
    - 新增确认区（数量确认后写入）、连续扫码会话、短时去重、语音提示与会话计数。

- Android（Flutter）
  - `app/lib/src/core/widgets/barcode_scanner_sheet.dart`
    - 新增 `scanContinuous(...)` 连续扫码底部弹层，支持会话内去重窗口与处理计数展示。
  - `app/lib/src/features/inventory/application/inbound_controller.dart`
    - 新增 `InboundScanPreview`、`scanForConfirm(...)`、`applyScanConfirmed(...)`。
    - 抽取 `_applyScannedToForm(...)` 统一扫码写入逻辑，兼容快速累加与确认写入。
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
    - 新增三模式（默认扫码后确认）与确认区 UI。
    - 新增连续扫码会话流程、处理计数、会话结束提示。
    - 成功反馈使用 `SystemSound.play(SystemSoundType.click)`。
  - `app/lib/src/features/inventory/presentation/outbound_page.dart`
    - 扩展为三模式，新增连续扫码会话模式。
    - 摄像头连续识别后自动累加明细并计数，结束会话后弹提示。
    - 成功反馈使用 `SystemSound.play(SystemSoundType.click)`。

### 回归结果
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

### 当前结论
- 入库/出库连续扫码会话模式已在 Windows + Android 双端完成落地；
- 保持后端 API 契约不变，满足 v1.1.8 文档约束，可进入业务验收阶段。

## 2026-03-11（采购入库 422 非标准 JSON 错误统一修复收尾）

### 用户问题
- 采购入库提交异常时（请求体解析失败）返回了 Axum 默认错误格式，Flutter 端识别为“非标准 JSON（code=9001）”。

### 本轮完成
- 服务端统一提取器：`server/src/extractors.rs`
  - `AppJson<T>` 封装 `Json<T>` 提取失败。
  - 将 `JsonRejection` 映射为 `AppError`，统一业务码 `4000`，并回显/生成 `request_id`。
- 路由入参提取统一替换为 `AppJson<...>`（auth / inventory / products / purchase_orders / sales_orders / stock_checks / users）。
- 新增测试模块：`server/src/routes/tests/json_extractor.rs`
  - 覆盖 inbound 三类场景：
    - JSON 语法错误
    - 字段类型错误
    - Content-Type 不匹配
  - 断言统一响应结构：`code/message/data/request_id`，且 `code=4000`，header/body 均有 `x-request-id/request_id`。
- 修复 `架构设计文档.md` 6.1 代码块排版污染（约束说明移出 JSON code block）。

### 验证与部署
- 本地：
  - `cargo test --manifest-path server/Cargo.toml routes::tests::json_extractor` ✅
  - `cargo test --manifest-path server/Cargo.toml routes::tests` ✅
  - `cargo test --manifest-path server/Cargo.toml` ✅（119 passed）
- 远端部署：
  - `RUN_SMOKE_AUTH_REGISTER=1 bash server/scripts/deploy_remote.sh` ✅
  - 远端测试通过、release 构建通过、systemd 服务重启成功、`/health` 检查通过、auth/register 冒烟通过。

### 当前结论
- “请求体解析失败返回非标准 JSON”问题已闭环修复并覆盖测试。
- 入库等写接口在解析失败时已统一返回标准错误响应，不再透传框架默认纯文本。

## 2026-03-10（出库流程模式化 + 订单明细默认只读策略收尾）

### 用户目标（本轮）
- 出库流程模式化：默认“扫码快速累加”，可切换“扫码后确认”。
- 订单列表明细编辑策略：默认只读，点击“编辑”后可修改，降低误触风险。
- 修复“默认空首行导致提交失败”问题：采用**页面初始空状态（不预置空明细）**。

### 文档先行（已完成）
- `产品需求文档.md` 升级至 `v1.1.6`：新增出库流程模式化、明细默认只读、初始空状态约束。
- `架构设计文档.md` 升级至 `v1.1.6`：补充前端交互策略（不变更后端 API 契约）。
- `API接口定义文档.md` 升级至 `v1.1.6`：补充前端模式约定与提交前空白行忽略规则。

### 代码落地（已完成）
- Windows（Vue）
  - `client/src/pages/OutboundPage.vue`
    - 新增 `quick_accumulate / scan_confirm` 模式切换。
    - 明细初始空数组，提交前过滤空白行。
    - 明细行新增 `editable`，默认只读，支持“编辑/完成”切换。
  - `client/src/pages/PurchaseOrdersPage.vue`
    - 创建明细改为初始空数组；明细默认只读 + 编辑切换；空白行过滤。
  - `client/src/pages/SalesOrdersPage.vue`
    - 创建/退货明细改为初始空数组；默认只读 + 编辑切换；空白行过滤。

- Flutter
  - `app/lib/src/features/inventory/application/outbound_controller.dart`
    - 新增 `scanForConfirm(...)` 及 `OutboundScanPreview`。
    - `scanAndAccumulate / validateForm / submit` 全链路接入空白行过滤。
  - `app/lib/src/features/inventory/presentation/outbound_page.dart`
    - 新增扫码模式枚举与确认区 UI。
    - 初始不预置空明细；空状态提示。
    - 明细默认只读，支持“编辑/完成”。
    - 提交前 `_compactBlankRows()` 清理空行。
    - 兼容 Flutter 新版 API：`DropdownButtonFormField.value` -> `initialValue`。

### 验证结果
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅ 通过。
- `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅ 通过（已修复 outbound_page 的 deprecated 告警）。

### 当前结论
- 本轮“出库流程模式化 + 明细默认只读 + 初始空状态”已完成文档与多端同步；
- Flutter 侧收尾告警已清理，当前可进入后续联调/体验验收阶段。

## 2026-03-10（Android 条码输入交互收敛：按钮改输入框尾部扫码图标）

### 需求与结论
- 用户提出：取消独立“扫码条形码按钮”，改为输入框后面的扫码图标是否更合理。
- 结论：**更合理**。该方案更贴近“输入来源切换”心智（手输/扫码都围绕同一输入框），可减少按钮噪音并缩短操作路径。

### 本轮文档更新（先文档后开发）
- `产品需求文档.md`
  - 版本升级至 `v1.1.5`。
  - 在 `4.6.3 交互与验收标准` 新增约束：Android 条码输入位默认采用“输入框尾部扫码图标”，取消独立扫码按钮。
- `架构设计文档.md`
  - 版本升级至 `v1.1.5`。
  - 在 Android UI/UX 演进原则中新增：条码录入优先使用输入框尾部扫码图标。
- `API接口定义文档.md`
  - 版本升级至 `v1.1.5`。
  - 在 Android UI/UX 兼容约定中新增：扫码图标交互不影响 API 参数与错误码语义。

### 本轮代码改造（Flutter）
- `app/lib/src/features/inventory/presentation/inbound_page.dart`
  - 扫码区条码输入框新增 `suffixIcon` 扫码图标，移除独立“手机摄像头扫码”按钮。
  - 入库表单 `barcode` 输入框新增 `suffixIcon` 扫码图标。
- `app/lib/src/features/inventory/presentation/outbound_page.dart`
  - 扫码区条码输入框新增 `suffixIcon` 扫码图标，移除独立“手机摄像头扫码”按钮。
- `app/lib/src/features/inventory/presentation/stock_check_page.dart`
  - 扫码区条码输入框新增 `suffixIcon` 扫码图标，移除独立“手机摄像头扫码”按钮。
- `app/lib/src/features/products/presentation/products_page.dart`
  - 筛选条码输入框新增 `suffixIcon` 扫码图标。
  - 新建商品 BottomSheet 的条码输入框新增 `suffixIcon` 扫码图标，移除独立扫码按钮。
- `app/lib/src/features/products/presentation/products_edit_page.dart`
  - 编辑商品页条码输入框新增 `suffixIcon` 扫码图标，移除独立扫码按钮。

### 验证结果
- 搜索校验：Flutter 代码中已无“手机摄像头扫码填入条码/手机摄像头扫码”独立按钮文案残留。
- 静态检查：`flutter analyze /Users/admin/Documents/Projects/jxc/app` 通过，`No issues found!`。

## 2026-03-10（Android 高配软装 UI 方案落地收尾）

### 本轮目标
- 延续“高配软装”实施任务，将 Flutter 端核心业务页统一升级为品牌化骨架：
  - 商品管理（Products）
  - 采购入库（Inbound）
  - 销售出库（Outbound）
  - 库存盘点（Stock Check）
- 完成静态检查收敛，确保可编译、可分析通过。

### 本轮实际完成（代码）
- `app/lib/src/features/products/presentation/products_page.dart`
  - 引入 `BrandHeroBanner` 顶部品牌区。
  - 筛选区由原 `Card` 升级为 `SectionCard`。
  - 权限提醒/成功失败消息改为统一 `StatusNotice`。
  - 移除页面内局部 `_MessageCard/_WarningCard`，统一复用品牌组件。

- `app/lib/src/features/inventory/presentation/inbound_page.dart`
  - 新增品牌头图（入库作业语义）。
  - 扫码区、表单区、结果区统一迁移到 `SectionCard`。
  - 错误与成功提示统一为 `StatusNotice`。

- `app/lib/src/features/inventory/presentation/outbound_page.dart`
  - 新增品牌头图（出库作业语义）。
  - 扫码区/表单区/明细区/结果区统一迁移 `SectionCard`。
  - 明细卡片增加 `action`（删除）以保持结构一致。
  - 提示信息统一为 `StatusNotice`。

- `app/lib/src/features/inventory/presentation/stock_check_page.dart`
  - 新增品牌头图（盘点三步流程语义）。
  - “扫码选品 / 创建盘点单 / 查询开始 / 确认盘点 / 结果”五段统一 `SectionCard`。
  - 顶层与分段状态反馈统一为 `StatusNotice`。

- 稳定性与兼容性修复：
  - `app/lib/src/features/auth/presentation/auth_page.dart`
    - 修复列表闭合符错误（`children` 闭合），消除语法报错。
  - `app/lib/src/core/theme/app_theme.dart`
  - `app/lib/src/core/widgets/brand_ui.dart`
    - 将 `withOpacity(...)` 调整为 `withValues(alpha: ...)`，消除 Flutter 新版弃用告警。

### 本轮验证结果
- 已执行：`flutter analyze /Users/admin/Documents/Projects/jxc/app`
- 结果：`No issues found!`

### 当前结论
- Android 端“高配软装”在核心业务页面已完成统一落地。
- 页面视觉语言、组件形态、状态反馈方式已形成一致基线，可继续按同一模式扩展到后续新页面。

## 2026-03-10（续做收尾：商品成本口径与文案统一补齐）

### 本轮背景
- 延续上一轮任务，按“以代码真实状态为准”继续补齐尚未完成的前端改造与回归验证。
- 目标不变：
  - 新建商品 `cost_price` 必填；
  - 入库 `unit_cost` 可选（空值/不传触发服务端回落）；
  - 多端“批发价”统一改为“批量销售价”。

### 本轮实际完成（代码）
- Flutter：
  - `app/lib/src/features/products/presentation/products_page.dart`
    - 创建表单中 `成本价` 改为必填（校验 + 文案）。
    - 创建请求 `CreateProductRequest` 改为始终传入 `costPrice`（对齐模型 required）。
    - 文案统一：`批发价` → `批量销售价`（列表/创建错误提示）。
  - `app/lib/src/features/products/presentation/products_edit_page.dart`
    - 文案统一：`批发价 *` / `批发价格式错误` → `批量销售价 *` / `批量销售价格式错误`。

- Windows（Vue3/TS）：
  - `client/src/types/api.ts`
    - `CreateProductRequest.cost_price`：`string`（必填）。
    - `InboundRequest.unit_cost`：`string?`（可选）。
  - `client/src/pages/InboundPage.vue`
    - `unit_cost` 仅在非空时校验格式；
    - payload 仅在非空时携带 `unit_cost`；
    - 文案去星号：`单次进价 *` → `单次进价`，占位改为可选。
  - `client/src/pages/ProductsPage.vue`
    - 创建商品：`cost_price` 改必填（空值报“成本价不能为空”）。
    - 创建 payload 固定传 `cost_price`。
    - 全页面文案统一：`批发价` / `批发价格式错误` → `批量销售价` / `批量销售价格式错误`。

### 本轮验证结果
- `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml routes::tests`
  - 结果：`102 passed; 0 failed`（含 inbound 回落相关测试通过）。
- `flutter analyze /Users/admin/Documents/Projects/jxc/app`
  - 结果：`No issues found!`
- `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client`
  - 结果：通过（`vue-tsc --noEmit` 无报错）。

### 当前结论
- 本轮续做范围内的 Flutter 与 Windows 改造已补齐，并通过既定回归检查。
- 现有代码与本需求（`cost_price` 必填、`unit_cost` 可选回落、文案统一）保持一致。

## 2026-03-10（成本价/入库单价口径统一与多端同步）

### 背景
- 用户要求对商品管理模块的 `cost_price`（成本价）和采购入库模块的 `unit_cost`（入库单次进价）进行口径统一与多端同步改造。
  - 新建商品时 `cost_price` 必填。
  - 采购入库时 `unit_cost` 可选，若不提供则回落到商品当前 `cost_price`。
  - 统一 Flutter 和 Windows 端文案“批发价”为“批量销售价”。

### 本轮已完成文档更新（严格遵循“文档先行”原则）
- `产品需求文档.md`
  - 更新 `4.2 商品管理模块`：明确新建商品时 `cost_price` 必填，且 `retail_price/wholesale_price/cost_price` 均可配置。
  - 更新 `4.3 采购入库模块`：明确 `unit_cost` 可选，不填时回落商品当前成本价。
  - 新增 `9.研发对齐要求` 中第 5 条：商品创建时 `cost_price` 必填。
- `架构设计文档.md`
  - 更新 `6.2 通用规则`：明确 `unit_cost` 可选及回落逻辑，`cost_price` 创建必填。
  - 更新 `7.1 Windows 端（Tauri + Vue3）` 和 `7.2.B Android 创建流程单号交互约束（本次新增）` 中关于 biz_no 的说明，虽然是旧任务，但一并校对。
- `API接口定义文档.md`
  - 更新 `3.2 新建商品`：字段说明将 `cost_price` 改为必填。
  - 更新 `3.3 商品列表` 和 `3.5 更新商品`：文案“批发价”更新为“批量销售价”。
  - 更新 `4.1 采购入库`：字段说明将 `unit_cost` 改为可选，并补充回落逻辑。
- `server/openapi.yaml`
  - 更新 `ProductCreateRequest` schema：`required` 列表中新增 `cost_price`。
  - 更新 `InboundRequest` schema：`required` 列表中移除 `unit_cost`，并在 `unit_cost` 属性中添加描述“入库单次进价，可选，不传时回落商品当前成本价”。
  - 更新 `ProductUpdateRequest` schema 中的 `retail_price` 和 `wholesale_price` 描述。

### 本轮已完成服务端代码改造
- `server/src/routes/common.rs`
  - `CreateProductRequest` 中的 `cost_price` 从 `Option<String>` 修改为 `String`（必填）。
  - `InboundRequest` 中的 `unit_cost` 从 `String` 修改为 `Option<String>`（可选）。
- `server/src/routes/products.rs`
  - `create_product` 路由处理函数中，移除了 `cost_price` 的 `match` 语句，直接使用 `parse_decimal(&req.cost_price, "cost_price", &request_id)?` 确保必填校验。
- `server/src/routes/inventory.rs`
  - `inbound` 路由处理函数中，修改了 `unit_cost` 的解析逻辑：如果 `unit_cost` 未提供或为空字符串，则通过 `state.repository.find_product_by_id` 获取商品的当前 `cost_price` 作为回落值。
- `server/src/repository.rs`
  - `inbound` 方法的签名中 `unit_cost` 参数从 `Decimal` 修改为 `Option<Decimal>`。
  - `inbound` 方法的内部逻辑：如果 `unit_cost` 为 `None`，则从数据库查询商品的当前 `cost_price` 作为实际入库成本。

### 本轮已完成服务端测试用例同步
- `server/src/routes/tests.rs`
  - 更新 `create_product` 相关的测试用例，在 `ProductCreateRequest` payload 中添加 `"cost_price": "5.00"` 字段，以符合新的必填要求。
  - 新增 3 个 `inbound` 相关的测试用例，分别测试 `unit_cost` 提供了值、`unit_cost` 为 None（回落）、`unit_cost` 为空字符串（回落）的场景，确保回落逻辑正确。
  - **验证结果**: `cargo test --manifest-path server/Cargo.toml routes::tests` 运行通过，`114 passed; 0 failed`。

### 本轮已完成 Flutter 端同步
- `app/lib/src/features/inventory/models/inventory_models.dart`
  - `InboundRequest` 中的 `unitCost` 字段类型从 `String` 修改为 `String?`，并在 `toJson` 方法中添加条件判断 `if (unitCost != null && unitCost!.trim().isNotEmpty)` 以便可选。
- `app/lib/src/features/inventory/application/inbound_controller.dart`
  - `validateForm` 方法中 `unitCost` 的校验逻辑调整为：当 `unitCost` 非空时才进行价格格式校验。同时，在 `submit` 方法中构建 `InboundRequest` 时，如果 `unitCost` 为空字符串，则将其赋值为 `null`，以便服务端进行回落处理。
- `app/lib/src/features/inventory/presentation/inbound_page.dart`
  - 将“单次进价”的 `label` 从 `单次进价 *` 修改为 `单次进价`，表示其不再是必填项。
- `app/lib/src/features/products/models/product_models.dart`
  - `CreateProductRequest` 中的 `costPrice` 从 `String?` 修改为 `String`，并在 `toJson` 方法中移除了 `costPrice` 的空值判断，以符合必填要求。
- `app/lib/src/features/products/presentation/products_page.dart`
  - 商品列表中的“批发价”文本修改为“批量销售价”。
  - “新建商品”表单中，“成本价”的 `label` 从 `成本价（可选）` 修改为 `成本价 *`，并在 `_validateForm` 方法中移除了对 `costPriceRaw` 的 `isNotEmpty` 判断，直接进行价格格式校验，确保必填。
- `app/lib/src/features/products/presentation/products_edit_page.dart`
  - “编辑商品”表单中，“批发价”的 `label` 从 `批发价 *` 修改为 `批量销售价 *`。
  - **验证结果**: `flutter analyze app` 运行通过，`No issues found!`。

### 本轮已完成 Windows 端同步
- `client/src/types/api.ts`
  - `CreateProductRequest` 中的 `cost_price` 字段类型从 `string?` 修改为 `string`，并添加注释 `// 必填`。
  - `InboundRequest` 中的 `unit_cost` 字段类型从 `string` 修改为 `string?`，并添加注释 `// 可选，不传时回落商品当前成本价`。
- `client/src/pages/InboundPage.vue`
  - 修改 `validateForm` 函数：当 `form.unit_cost` 非空时才校验格式，若为空则要求 `product_id` 或 `barcode` 至少一个存在，以便后端回落成本价。
  - 修改 `buildPayload` 函数：当 `form.unit_cost` 非空时才添加到 payload 中。
  - 修改模板中“单次进价”的 `span` 文本，移除 `*`，并在 `placeholder` 中添加 `(可选)`。
- `client/src/pages/ProductsPage.vue`
  - 修改 `validateCreateForm` 函数：`createForm.wholesale_price` 的格式错误提示文本修改为“批量销售价格式错误”。`createForm.cost_price` 的校验逻辑调整为：首先判断是否为空，如果为空则返回“成本价不能为空”的错误；如果非空，则继续校验其价格格式。
  - 修改模板中商品列表的 `批发价` 为 `批量销售价`。
  - 修改模板中“创建商品属性”部分的 `批发价 *` 为 `批量销售价 *`。
  - 修改模板中“创建商品属性”部分的 `成本价` 为 `成本价 *`。
  - 修改模板中商品列表 `current_stock` 的展示，新增 `{{ item.unit }}`，使其变为 `库存 {{ item.current_stock }} {{ item.unit }}`。
- `client/src/pages/ProductsEditPage.vue`
  - 修改模板中“编辑商品属性”部分的 `批发价 *` 为 `批量销售价 *`。
- `client/src/pages/OutboundPage.vue`
  - 修改扫码选品区域“销售单价”的 `span` 文本，将 `(可选)` 修改为 `(选填)`。
- `client/src/pages/PurchaseOrdersPage.vue`
  - 修改扫码选品区域“单次进价”的 `span` 文本，将 `(可选)` 修改为 `(选填)`。
- `client/src/pages/SalesOrdersPage.vue`
  - 修改扫码选品区域“销售单价”的 `span` 文本，将 `(可选)` 修改为 `(选填)`。
- `client/src/pages/StockChecksPage.vue`
  - 修改 `h3` 标签文本，将 `2）盘点开始 / 确认` 修改为 `2）盘点开始 / 确认 / 作废（尚未实现）`。
  - **验证结果**: `npm run typecheck --prefix client` 运行通过。

### 已执行回归检查并修复问题
- `app/test/widget_test.dart` 中，将 `await tester.pumpWidget(const MyApp());` 修改为 `await tester.pumpWidget(const JxcApp());` 以匹配 `main.dart` 中的 `runApp(const JxcApp());`。
  - **验证结果**: `flutter analyze app` 运行通过，`No issues found!`。

### 远端部署
- 执行命令：`RUN_SMOKE_AUTH_REGISTER=1 bash server/scripts/deploy_remote.sh`。
- **验收结果**：部署成功，健康检查通过，认证注册冒烟测试通过。

### 当前结论
所有文档、服务端、Flutter 端和 Windows 端都已根据需求完成更新和同步，并且通过了各自的静态检查和测试。服务端也已成功部署到远程服务器。

## 2026-03-10（routes 测试文件拆分）

### 背景
- 用户反馈 `server/src/routes/tests.rs` 体量过大（约 5000+ 行），希望进行拆分以提升可维护性与可读性。

### 本轮改造
- 将原单文件 `server/src/routes/tests.rs` 拆分为目录模块：`server/src/routes/tests/`。
- 保留共享测试辅助方法在 `server/src/routes/tests/mod.rs`（如 `make_test_state`、`login_token`、`response_json`、幂等断言辅助）。
- 按测试主题拆分为 6 个子模块：
  - `auth_request_id.rs`
  - `idempotency_missing_key.rs`
  - `idempotency_payload_conflict.rs`
  - `idempotency_replay.rs`
  - `business_flow.rs`
  - `user_role_management.rs`
- `server/src/routes/mod.rs` 维持 `#[cfg(test)] mod tests;` 不变，通过 Rust 模块同名目录机制自动加载 `tests/mod.rs`。

### 验证
- 执行：`cargo test --manifest-path server/Cargo.toml routes::tests`
- 结果：`100 passed; 0 failed`。

### 当前结论
- 测试行为保持一致，功能无变更，仅完成测试代码结构化重构，后续定位与维护成本明显下降。
