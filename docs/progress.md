# 项目进度 (Progress)

## 2026-03-13（前端审美升级 v1.2.31：视觉系统与交互动效收尾）

- [x] 完成文档先行约束执行
  - 已基于 `v1.2.31` 完成 PRD/架构/API 文档更新后再实施前端改造。
- [x] 完成全局视觉系统升级
  - `client/src/style.css`
  - 落地品牌色阶/语义色/阴影 token，升级按钮体系（含 `btn-success / btn-warning`）与 `btn-icon` 图标样式。
  - 新增导航 duotone 配色、骨架屏 shimmer、页面淡入与卡片 hover 动效。
- [x] 完成经营看板数据层级与反馈增强
  - `client/src/pages/DashboardPage.vue`
  - 增加骨架屏、指标数字 count-up、销售趋势 sparkline（SVG）。
- [x] 完成导航图标化与页面微可视化增强
  - `client/src/layouts/MainLayout.vue`：导航 `iconClass` + `nav-icon` 结构化图标。
  - `client/src/pages/LowStockPage.vue`：库存占比进度条、次级信息弱化、loading skeleton。
- [x] 完成高频业务页图标与语义动作强化
  - `client/src/pages/ProductsPage.vue`
  - `client/src/pages/InboundPage.vue`
  - `client/src/pages/OutboundPage.vue`
  - 查询/清空/编辑/删除/提交等动作补齐线性 SVG 图标，关键操作统一语义色按钮。
- [x] 完成前端类型检查
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
  - 输出：`vue-tsc --noEmit -p tsconfig.app.json`，exit code 0。
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
前端“审美升级 + 体验升级”五大方向（色彩系统、卡片与阴影、字体层级、图标体系、交互动效）已完成落地并通过类型检查，可进入体验验收。

## 2026-03-13（“需求没生效”收尾：进货价格文案与默认值链路双端校验）

- [x] 修复 Web 入库结果区残留文案
  - `client/src/pages/InboundPage.vue`
  - `当前成本价` -> `当前进货价格`
  - 批量结果表头 `成本价` -> `进货价格`
- [x] 完成目标范围残留复核
  - 针对 `wholesale/单次进价/成本价/批发价` 执行全局检索
  - 与本次需求直接相关页面已完成对齐
- [x] 完成静态检查
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- [x] 完成 memory-bank 记录更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
用户反馈“没生效”的两项需求已完成最后收尾：
1) 商品侧“进货价格”替换与批量销售价移除已生效；
2) 入库侧默认进价链路（`last_inbound_unit_cost ?? cost_price`）与文案统一已生效；
且 Web + Flutter 静态检查均通过。

## 2026-03-13（wholesale_price 移除 + last_inbound_unit_cost 全链路迁移完成）

- [x] 完成 Memory 分支入库口径迁移
  - `server/src/routes/inventory.rs`
  - `inbound / inbound_batch` 统一为：`unit_cost ?? last_inbound_unit_cost ?? cost_price`
  - 入库后回写：`product.last_inbound_unit_cost = Some(effective_unit_cost.round_dp(4))`
- [x] 完成内存样例商品字段迁移
  - `server/src/state.rs`
  - `sample_product` 从 `wholesale_price` 改为 `last_inbound_unit_cost: Some(...)`
- [x] 完成路由测试字段同步
  - `server/src/routes/tests/business_flow.rs`
  - `server/src/routes/tests/idempotency_replay.rs`
  - `server/src/routes/tests/idempotency_payload_conflict.rs`
  - `server/src/routes/tests/idempotency_missing_key.rs`
  - `server/src/routes/tests/auth_request_id.rs`
  - 已移除所有请求体 `wholesale_price`，并修正旧 `Product` 构造字段
- [x] 完成口径变更导致的测试断言修复
  - `server/src/routes/tests/idempotency_replay.rs`
  - “inbound 无 unit_cost”用例期望值更新为回落 `last_inbound_unit_cost` 后的加权成本 `2.1320`
- [x] 完成服务端全量回归
  - `cd /Users/admin/Documents/Projects/jxc/server && cargo test -q` ✅
  - 结果：`126 passed; 0 failed`
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
`wholesale_price -> last_inbound_unit_cost` 迁移已在后端内存分支、测试与入库成本口径上全部完成，且全量测试通过。

## 2026-03-13（入库真实进价展示闭环完成）

- [x] 完成入库日志页面“本次进货价”展示
  - `app/lib/src/features/inventory/presentation/inbound_logs_page.dart`
  - 在入库记录卡片新增：`本次进货价：${log.snapshotInboundUnitCost ?? '--'}`
- [x] 完成服务端编译检查
  - `cd /Users/admin/Documents/Projects/jxc/server && cargo check` ✅
- [x] 完成 Flutter 静态检查
  - `cd /Users/admin/Documents/Projects/jxc/app && flutter analyze` ✅（No issues found）
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
“展示每次进货的真实价格”已在 Android 入库记录页完成交付，空值兜底显示 `--`，并通过 server/app 双侧检查。

## 2026-03-13（Android“采购记录”迁移为“入库记录”完成）

- [x] 完成文档约束对齐（前序已完成）
  - PRD/ADD/API 已升级至 `v1.2.27`，明确 Android 入口从“采购记录”切换为“入库记录”，查询口径固定 `biz_type=IN_PURCHASE`。
- [x] 完成 Flutter Repository 查询链路
  - `app/lib/src/features/inventory/models/inventory_repository.dart`
  - 新增 `fetchInboundLogs(...)`，对接 `GET /inventory/logs` 并固定 `biz_type=IN_PURCHASE`。
- [x] 完成 Flutter 应用层迁移
  - 新增 `app/lib/src/features/inventory/application/inbound_logs_controller.dart`
  - 覆盖权限、日期/分页校验、错误提示、加载状态。
- [x] 完成 Flutter 展示层迁移
  - 新增 `app/lib/src/features/inventory/presentation/inbound_logs_page.dart`
  - 提供默认当天、日期范围筛选、分页、只读流水卡片展示。
- [x] 完成 Dashboard 入口与 App 装配切换
  - `app/lib/src/features/dashboard/presentation/dashboard_page.dart`
    - 快捷入口：`采购记录 -> 入库记录`
    - 类型：`purchaseOrders -> inboundLogs`
    - 跳转：`PurchaseOrdersPage -> InboundLogsPage`
  - `app/lib/src/app.dart`
    - 控制器注入：`PurchaseOrdersController -> InboundLogsController`
    - 生命周期管理同步更新。
- [x] 完成静态检查
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
Android 端“采购记录”已完成到“入库记录（IN_PURCHASE）”的查询与展示迁移，入口、控制器、页面、仓储链路均已切换并通过静态检查。

## 2026-03-13（Android“采购记录”改“入库记录”收尾完成）

- [x] 完成文档先行约束执行
  - 延续已完成的 `v1.2.27` 文档基线（PRD/ADD/API），本轮按既定约束完成 Flutter 代码迁移。
- [x] 完成模型/仓储链路接入
  - `app/lib/src/features/inventory/models/inventory_models.dart`（前序已新增）
    - `StockLogData`、`InboundLogsPageData`
  - `app/lib/src/features/inventory/models/inventory_repository.dart`
    - 新增 `fetchInboundLogs(...)`
    - 请求 `GET /inventory/logs` 并固定 `biz_type=IN_PURCHASE`
- [x] 完成应用层控制器迁移
  - 新增 `app/lib/src/features/inventory/application/inbound_logs_controller.dart`
  - 覆盖权限控制、日期/分页校验、统一错误提示。
- [x] 完成展示层页面迁移
  - 新增 `app/lib/src/features/inventory/presentation/inbound_logs_page.dart`
  - 实现默认当天筛选、日期范围日历选择、分页、每页条数回落（默认 10）、只读流水卡片展示。
- [x] 完成 Dashboard 与 App 装配切换
  - `app/lib/src/features/dashboard/presentation/dashboard_page.dart`
    - 快捷入口由“采购记录”改为“入库记录”
    - 类型从 `purchaseOrders` 改为 `inboundLogs`
    - 跳转页改为 `InboundLogsPage`
  - `app/lib/src/app.dart`
    - 控制器注入由 `PurchaseOrdersController` 切换为 `InboundLogsController`
    - 生命周期（创建/释放）同步完成。
- [x] 完成静态检查
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
Android 端“采购记录入口/页面”已迁移为“入库记录入口/页面”，并按 `IN_PURCHASE` 口径展示库存入库流水，且已通过 Flutter 静态检查。

## 2026-03-12（入库提交按钮下沉到待提交明细区：交付完成）

- [x] 完成文档先行补齐（API 文档）
  - `API接口定义文档.md` 升级至 `v1.2.26`
  - 新增 `14. 入库提交按钮位置优化兼容约定（本次新增）`
  - 明确“两段式交互”约束：表单区仅录入/加明细，提交入口下沉至明细区
  - 明确接口契约不变：不新增 API，不改权限/错误码/幂等语义
- [x] 完成 Web 入库页交互改造
  - `client/src/pages/InboundPage.vue`
  - 表单区移除“提交入库”，保留“加入明细/重置”
  - 在“待提交入库明细”卡片底部新增主提交按钮
  - 提交禁用条件：`loading || !canSubmit || getEffectiveDraftItems().length === 0`
  - 增加状态提示：
    - 空明细：`请先加入明细，再提交入库。`
    - 非空：`请核对明细后提交入库。`
- [x] 完成 Flutter 兼容修复与 lint 收敛
  - `app/lib/src/core/widgets/brand_ui.dart`
    - `SectionCard` 新增可选 `footer` 参数并渲染
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
    - 修复 `sort_child_properties_last`：`SectionCard` 的 `child` 参数调整为最后一个命名参数
- [x] 完成静态检查
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
“入库提交按钮下沉到待提交明细区”的优化已完成文档、实现、兼容修复与检查闭环。当前行为已收敛为两段式交互，且不影响后端 API 契约。

## 2026-03-12（方案A续做：采购记录只读分页查询，后端 + Flutter 落地）

- [x] 完成后端采购记录查询链路（前序已完成）
  - `server/src/routes/mod.rs`
    - `/purchase-orders` 路由增加 `GET`（保留 `POST`）
  - `server/src/routes/purchase_orders.rs`
    - 新增 `list_purchase_orders`
    - 默认 `page_size=10`，支持调整（1~100）
    - 日期范围支持 `start_date/end_date`，默认当天，且成对校验
    - 权限限定 `OWNER/PURCHASER`
  - `server/src/repository.rs`
    - 新增 `list_purchase_orders_by_tenant`（Provider + Postgres 实现）
- [x] 完成 Flutter 数据模型与 repository 封装
  - `app/lib/src/features/inventory/models/inventory_models.dart`
    - 新增 `PurchaseOrderItemData / PurchaseOrderData / PurchaseOrdersPageData`
  - `app/lib/src/features/inventory/models/inventory_repository.dart`
    - 新增 `fetchPurchaseOrders(...)` 对接 `GET /purchase-orders`
- [x] 完成 Flutter controller 与页面
  - `app/lib/src/features/inventory/application/purchase_orders_controller.dart`
    - 新增日期/分页校验、加载状态、错误提示
  - `app/lib/src/features/inventory/presentation/purchase_orders_page.dart`
    - 默认当天筛选
    - 每页默认 10 条，可调整
    - 上一页/下一页分页查询
    - 只读详细展示（单头+明细+金额+状态+版本+时间+备注）
- [x] 完成 Dashboard 入口与 App 注入
  - `app/lib/src/features/dashboard/presentation/dashboard_page.dart`
    - 新增“采购记录”快捷入口（`OWNER/PURCHASER`）
  - `app/lib/src/app.dart`
    - 注入并管理 `PurchaseOrdersController` 生命周期
- [x] 完成必要检查
  - `dart analyze /Users/admin/Documents/Projects/jxc/app` ✅
  - `cargo check --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml` ✅
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
方案A“采购记录只读分页查询”已完成后端 + Flutter 端到端落地，满足四项硬约束：默认每页 10、支持日期范围且默认当天、无修改能力、记录详情充分。

## 2026-03-12（v1.2.24 出/入库明细展示增强：Web + Android 双端落地）

- [x] 完成 Web 入库页待提交明细增强
  - `client/src/pages/InboundPage.vue`
  - 新增“商品名称”列展示 `product_name`
  - `unit_cost` 改为明细行可编辑输入
  - 空行判定纳入 `product_name`
- [x] 完成 Android 出库控制器模型扩展
  - `app/lib/src/features/inventory/application/outbound_controller.dart`
  - `OutboundFormItemInput` 新增 `productName`
  - 扫码新增/合并时持久化 `productName`
  - 有效行过滤纳入 `productName`
- [x] 完成 Android 出库页展示与版本收敛
  - `app/lib/src/features/inventory/presentation/outbound_page.dart`
  - 明细卡新增“商品名称”展示/编辑
  - 扫码确认弹窗移除行级“版本”输入，仅保留数量+销售单价
  - 行级组包固定 `expectedVersion=''`，保留单据级版本输入
- [x] 完成 Android 入库页待提交明细增强
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
  - 待提交明细展示商品名称
  - 新增 `unitCost` 行级编辑入口与校验
- [x] 完成静态检查
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
v1.2.24 要求的“出库明细商品名称展示 + 入库待提交商品名称与进价行级编辑”已在 Web 与 Android 双端落地并通过静态检查，且未变更后端接口契约。

## 2026-03-12（入库多商品明细池 + batch 提交、出库弹窗校验收尾）

- [x] 完成 Android 入库“明细池 + 提交编排”改造
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
  - 新增待提交明细池、扫码写入明细池、明细删除与数量合并逻辑
  - 提交策略：`>=2` 条走 `submitBatch`，`=1` 条走 `submit`，`=0` 条回退单表单
  - 新增批量结果展示（`batchResult`）
- [x] 完成 InboundController 扫码结果补强
  - `app/lib/src/features/inventory/application/inbound_controller.dart`
  - `InboundScanResult` 新增 `productName`，支持页面明细池展示商品名
- [x] 修复 Android 出库页 lint
  - `app/lib/src/features/inventory/presentation/outbound_page.dart`
  - 修复 `use_build_context_synchronously`（`await` 后增加 mounted 检查）
- [x] 修复 Web 入库页 TS 类型告警
  - `client/src/pages/InboundPage.vue`
  - 修复 `effectiveItems[0]` 可能为 `undefined` 的类型报错
- [x] 完成静态检查
  - `npm --prefix /Users/admin/Documents/Projects/jxc/client run typecheck` ✅
  - `flutter analyze` ✅（No issues found）
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
本轮“入库多商品明细池 + batch 编排提交”已在 App 端完成闭环；出库弹窗链路已完成 lint 收尾；Web/App 双端静态检查均通过。

## 2026-03-12（扫码模式收敛为两种：双端闭环完成）

- [x] 完成文档约束复核
  - `产品需求文档.md`、`架构设计文档.md`、`API接口定义文档.md` 已包含“两模式收敛 + 历史偏好迁移”约束
- [x] 完成 Windows 端收敛（前序已完成）
  - `client/src/pages/InboundPage.vue`、`client/src/pages/OutboundPage.vue`
  - 保留 `scan_confirm/continuous_scan`，移除 quick 模式 UI；历史值 `quick_accumulate` 自动映射并写回
- [x] 完成 Android 入库页收敛
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
  - 枚举收敛为双模式，移除 quick 分支与文案；历史值迁移并自动写回
- [x] 完成 Android 出库页收敛
  - `app/lib/src/features/inventory/presentation/outbound_page.dart`
  - 枚举收敛为双模式，默认模式改为 `scanConfirm`；移除 quick 分支与文案；历史值迁移并自动写回
- [x] 完成静态检查
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
扫码模式“`scan_confirm / continuous_scan` 双模式收敛”已在 Windows + Android 双端完成闭环，历史偏好 `quick_accumulate` 兼容迁移生效。本轮仅调整客户端交互与本地偏好，不涉及后端 API 契约变更。

## 2026-03-12（Android 订单下钻默认每页 10 条：交付完成）

- [x] 完成文档先行对齐（前序已完成）
  - `产品需求文档.md`、`架构设计文档.md`、`API接口定义文档.md` 已升级至 `v1.2.20`
  - 明确 Android 订单下钻：首次默认 `page_size=10`，手动输入为空/非法时回退 `10`
- [x] 完成 Flutter 入口默认值固定
  - `app/lib/src/features/dashboard/presentation/dashboard_page.dart`
  - 下钻入口 `initialPageSize` 固定为 `10`
- [x] 完成 Flutter 下钻页回退逻辑统一
  - `app/lib/src/features/dashboard/presentation/dashboard_orders_page.dart`
  - 新增 `_defaultOrdersDrilldownPageSize = 10`
  - 新增 `_resolvePageSizeOrDefault(String raw)`，统一处理空值/非法值/`<=0`
  - `_query`、`_gotoPage`、`build` 的 `pageSize` 解析全部改为复用该方法，移除 `?? 20`
- [x] 完成静态检查验证
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅
  - 结果：`No issues found!`
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
Android 看板订单下钻“每页条数默认 10”已完成交付：首次进入默认 10 条，且用户输入为空/非法/非正数时统一回退 10；服务端接口契约保持兼容不变。

## 2026-03-12（App 日期输入统一日历选择 + 默认当天：交付完成）

- [x] 完成文档先行更新（遵循仓库规则）
  - `产品需求文档.md` 升级至 `v1.2.19`，新增 `4.7.5 App 日期输入统一日历选择与默认当天`
  - `架构设计文档.md` 升级至 `v1.2.19`，新增 `7.2.J Android 日期输入日历化策略`
  - `API接口定义文档.md` 升级至 `v1.2.19`，新增 `5.1.D App 日期输入日历化兼容约定`
- [x] 完成 Flutter 日期输入改造
  - `app/lib/src/features/dashboard/presentation/dashboard_page.dart`
    - 看板查询日期改为只读输入 + 日历选择，默认当天。
  - `app/lib/src/features/dashboard/presentation/dashboard_orders_page.dart`
    - 下钻开始/结束日期改为只读输入 + 日历选择，默认当天，并增加区间边界自动修正。
- [x] 完成分析阶段 lint 收敛
  - 修复 `showDatePicker` 参数的 const/非const兼容问题；
  - 修复输入框 `OutlineInputBorder` 的 `prefer_const_constructors` 提示。
- [x] 完成静态检查验证
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅
  - 结果：`No issues found!`
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
App 现有需要填写日期的入口（看板单日期、订单下钻开始/结束日期）已全部切换为日历选择，默认当天；日期提交格式保持 `YYYY-MM-DD`，且已通过静态检查，任务完成。

## 2026-03-12（“仍无明细”线上根因排查与发布校正）

- [x] 完成问题复核与本地代码对比
  - 本地 `server/src/routes/reports.rs` 已为修复逻辑：`OUTBOUND_ONLY` 聚合单返回 `items: synthetic_items`。
- [x] 完成线上版本核查
  - 远端 `/projects/jxcServer/src/routes/reports.rs` 排查到旧逻辑：`items: Vec::new()`。
  - 结论：用户“仍无明细”来自线上运行版本滞后，而非当前主干代码缺失。
- [x] 完成数据层实证核对
  - `stock_logs` 中目标 `biz_no` 存在 `OUT_SALE` 且 `snapshot_sell_price` 非空；
  - 对应 `sales_orders` 主记录为空（典型 `OUTBOUND_ONLY`），按修复逻辑应能聚合出明细。
- [x] 完成远端部署与校验
  - 执行：`RUN_SMOKE_AUTH_REGISTER=1 bash /Users/admin/Documents/Projects/jxc/server/scripts/deploy_remote.sh`。
  - 结果：远端测试 124/124 通过、release 构建成功、服务重启成功、健康检查与冒烟通过。
- [x] 完成发布后复核
  - 远端 `reports.rs` 已更新为 `items: synthetic_items`。

**当前状态：**
“订单下钻仍无明细”已定位为线上旧版本导致，现网服务已切换到包含聚合明细回填的版本。当前需用户端重新查询验证，预期 `OUTBOUND_ONLY` 将显示商品名称/数量/单价。

## 2026-03-12（OUTBOUND_ONLY 聚合单明细回填收尾）

- [x] 完成后端聚合明细回填逻辑复核
  - `server/src/routes/reports.rs`
  - 已确认 `OUTBOUND_ONLY` 聚合行按 `(biz_no, product_id, sell_price)` 维度回填 `items`，不再固定 `items=[]`。
  - 已确认售价口径：`snapshot_sell_price` 优先，缺失时按 `(biz_no, product_id)` 映射兜底。
- [x] 完成回归测试断言补强
  - `server/src/routes/tests/business_flow.rs`
  - 在 `dashboard_orders_drilldown_should_align_with_dashboard_total_orders_and_support_paging()` 增加 `OUT-LEGACY-DRILL` 聚合单断言：
    - `status=OUTBOUND_ONLY`
    - `items` 非空
    - `product_name/qty/sell_price/line_amount/returned_qty` 值正确。
- [x] 完成接口文档与契约示例同步
  - `API接口定义文档.md`
    - 将 `OUT-LEGACY-DRILL` 示例从 `items: []` 修正为包含商品明细。
  - `server/openapi.yaml`
    - `info.version` 从 `1.1.1` 升级到 `1.1.2`；
    - `/api/v1/reports/dashboard/orders` 描述补充“聚合行优先回填 items，仅无法聚合时允许空数组”。
- [x] 完成测试验证
  - 目标测试：
    - `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml dashboard_orders_drilldown_should_align_with_dashboard_total_orders_and_support_paging 2>&1` ✅
    - 结果：`1 passed; 0 failed`
  - 全量测试：
    - `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml 2>&1` ✅
    - 结果：`124 passed; 0 failed`
- [x] 完成 memory-bank 收尾记录
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
看板订单下钻中 `OUTBOUND_ONLY` 聚合行的商品明细回填已完成代码、测试、接口文档与 OpenAPI 契约闭环；“订单列表未显示商品名称/价格/数量”问题已完成收尾。

## 2026-03-12（Android 订单下钻字段精简展示收尾）

- [x] 完成文档先行更新（PRD / ADD / API，升级到 `v1.2.17`）
  - `产品需求文档.md`：新增 `4.13.6 Android 订单下钻字段精简展示`。
  - `架构设计文档.md`：更新 `7.2.I`，明确 Android 不展示 `line_amount` 与 `updated_at`。
  - `API接口定义文档.md`：更新 `5.1.C`，补充 Android 渲染字段收敛约定。
- [x] 完成 Flutter 页面字段收敛
  - `app/lib/src/features/dashboard/presentation/dashboard_orders_page.dart`
  - 明细区收敛为：`product_name / qty / sell_price`（移除 `line_amount` 渲染）；
  - 订单尾时间仅展示 `created_at`（移除 `updated_at` 渲染）。
- [x] 完成本地静态检查
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- [x] 完成 memory-bank 收尾记录
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
Android 订单下钻列表已按用户最新要求完成字段精简展示：每个订单仅显示“商品名称/数量/单价”，时间仅显示“创建时间”。本次仅改展示层与文档约束，后端接口字段保持兼容返回。

## 2026-03-12（Android 看板订单下钻列表小票化修复）

- [x] 完成文档先行更新（PRD / ADD / API）
  - `产品需求文档.md` 升级到 `v1.2.16`，明确订单列表小票化约束覆盖 Android。
  - `架构设计文档.md` 升级到 `v1.2.16`，新增 `7.2.I Android 订单下钻列表小票化展示策略`。
  - `API接口定义文档.md` 升级到 `v1.2.16`，补充 Android 下钻页小票化展示约定。
- [x] 完成 Flutter 数据模型对齐
  - `app/lib/src/features/dashboard/models/dashboard_data.dart`
  - 下钻明细新增并解析 `productName`、`lineAmount`，并提供 `line_amount` 缺失兜底计算。
- [x] 完成 Flutter 下钻页小票化改造
  - `app/lib/src/features/dashboard/presentation/dashboard_orders_page.dart`
  - 订单列表由 `ListTile` 摘要改为“订单小票卡片 + 明细表格”展示。
  - `OUTBOUND_ONLY/items=[]` 聚合行保留整单可见，并展示“无明细（聚合行）”。
- [x] 完成静态检查验证
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

**当前状态：**
Android 端“看板订单下钻列表未小票化”问题已修复完成；当前可在移动端直接按小票样式核对“订单头 + 明细行 + 订单尾”，且不影响后端 API 契约与权限语义。

## 2026-03-12（Flutter Android arm64 APK 重新编译）

- [x] 基于当前代码重新编译 Android arm64 release 包
  - 执行：`cd /Users/admin/Documents/Projects/jxc/app && flutter build apk --release --target-platform android-arm64 --split-per-abi` ✅
  - 输出：`✓ Built build/app/outputs/flutter-apk/app-arm64-v8a-release.apk`
- [x] 完成产物存在性复核
  - 执行：`ls -lh /Users/admin/Documents/Projects/jxc/app/build/app/outputs/flutter-apk/app-arm64-v8a-release.apk` ✅
  - 结果：产物存在，大小约 `23MB`，时间戳 `Mar 12 10:11`

**当前状态：**
已成功产出最新 Android arm64 APK，可用于安装验证：
`/Users/admin/Documents/Projects/jxc/app/build/app/outputs/flutter-apk/app-arm64-v8a-release.apk`

## 2026-03-12（销售单下钻订单列表小票化展示完成）

- [x] 完成文档先行约束对齐
  - `产品需求文档.md`、`架构设计文档.md`、`API接口定义文档.md` 已在前序升级到 `v1.2.15`
  - 明确“订单下钻列表也需小票化展示”的契约约束
- [x] 完成销售单下钻列表小票化渲染
  - `client/src/pages/SalesOrdersPage.vue`
  - 从摘要表格改为“每单一张小票卡片”：
    - 订单头：`biz_no / id / status / total_amount`
    - 明细区：`product_name / qty / sell_price / line_amount`
    - 订单尾：`created_at / updated_at / remark`
- [x] 完成聚合行（`OUTBOUND_ONLY`）兼容展示
  - 当 `items=[]` 时显示“无明细（聚合行）”，保留订单头尾与备注，不隐藏整单
- [x] 完成样式落地
  - `client/src/style.css`
  - 新增小票卡片相关样式：`.order-receipt-*` 系列类
- [x] 完成前端类型检查
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅

**当前状态：**
销售单页“订单下钻列表”已按小票样式完成落地，满足“每种商品：名称/数量/单价/小计；订单级总金额”的展示要求。该改造仅涉及前端展示层，不变更后端 API 路径、字段、权限与错误码语义。

## 2026-03-12（订单明细商品名称快照口径：后端闭环完成）

- [x] 完成响应口径收敛（快照优先）
  - `server/src/routes/common.rs`
  - `to_purchase_order_data` / `to_sales_order_data` 已改为：
    - `product_name_snapshot` 优先
    - 其次 `product_name_map`
    - 最终兜底 `商品#<product_id>`
- [x] 完成测试编译缺口修复
  - `server/src/routes/tests/business_flow.rs`
  - 补齐 6 处 `SalesOrderItem` 初始化的 `product_name_snapshot` 字段。
- [x] 完成 OpenAPI 契约描述同步
  - `server/openapi.yaml`
  - 更新 `PurchaseOrderItemData.product_name`、`SalesOrderItemData.product_name` 描述为“快照优先 + 映射兜底 + 商品#id 兜底”。
- [x] 完成后端全量回归
  - `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml` ✅
  - 结果：`124 passed; 0 failed`
- [x] 完成结构体字段完整性复核
  - 扫描 `server/src/**/*.rs` 中 `SalesOrderItem`/`PurchaseOrderItem` 初始化块；
  - 结果：无 `product_name_snapshot` 缺失实例。

**当前状态：**
订单明细商品名称快照策略已在后端实现与契约层完成闭环，历史单据名称展示不再受商品改名影响，且未改变 API 路径、参数、权限、错误码与幂等语义。

## 2026-03-12（订单小票化收尾：Handler 编译阻塞修复）

- [x] 完成后端阻塞问题定位
  - 问题：`create_purchase_order/create_sales_order` 不满足 Axum `Handler`。
  - 根因：in-memory 分支中 `MutexGuard` 跨 `await`，导致 future 非 `Send`。
- [x] 完成后端修复
  - `server/src/routes/purchase_orders.rs`
  - `server/src/routes/sales_orders.rs`
  - 重构创建单据 in-memory 分支，确保锁在 `await` 前释放。
- [x] 完成后端回归验证
  - `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml` ✅
  - 结果：`124 passed; 0 failed`
- [x] 完成前端类型校验
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅

**当前状态：**
订单小票化收尾阶段的后端编译阻塞已解除，前后端校验均通过，可进入最终交付。

## 2026-03-11（Flutter Android arm64 APK 编译）

- [x] 完成 Android arm64 release 包编译
  - 执行：`cd app && flutter build apk --release --target-platform android-arm64 --split-per-abi` ✅
  - 产物：`app/build/app/outputs/flutter-apk/app-arm64-v8a-release.apk` （24.0MB）

**当前状态：**
已成功编译 Flutter Android arm64 release 版本的 APK。

## 2026-03-11（Dashboard 订单下钻筛选逻辑修复 + 服务端部署）

- [x] 完成问题定位与修复
  - 现象：点击 Dashboard "今日订单数"进入订单列表，开始/结束日期都为今天时看不到订单。
  - 根因：`get_dashboard_orders_drilldown` 函数基于库存流水（stock_logs）的 `OUT_SALE` 类型记录进行筛选，与 Dashboard 订单统计口径保持一致。
  - 修复：`server/src/routes/reports.rs` 恢复正确的筛选逻辑，基于库存流水时间而非销售订单创建时间。
- [x] 完成本地测试验证
  - `cargo test dashboard_orders_drilldown_should_align_with_dashboard_total_orders_and_support_paging` ✅
  - `cargo test` 全量回归：`124 passed; 0 failed` ✅
- [x] 完成服务端部署
  - 远端测试：`124 passed; 0 failed` ✅
  - release 构建完成，`jxc-server.service` 重启并 `active (running)` ✅
  - `http://127.0.0.1:8080/health` 返回 200 ✅
- [x] 完成 memory-bank 更新
  - `memory-bank/progress.md`

**当前状态：**
Dashboard 订单下钻筛选逻辑已修复并部署到服务器。现在点击"今日订单数"进入订单列表时，将正确显示与 Dashboard 统计口径一致的订单（基于 OUT_SALE 库存流水记录）。

## 2026-03-11（Android 看板下钻“列表仍空”深层兼容修复 + arm64 重打包）

- [x] 完成问题复核与链路排查
  - 用户反馈：安装新包后仍“看不到订单列表”。
  - 复核范围：`dashboard_repository -> dashboard_data -> dashboard_orders_page`。
- [x] 完成下钻解析链路增强
  - `app/lib/src/features/dashboard/models/dashboard_data.dart`
  - 新增深层容器提取能力：`extractMapsByShape(...)`
    - 兼容 `Map/List/Iterable/String(JSON)` 多形态响应包裹；
    - 支持通过字段形状识别订单/明细：
      - `looksLikeSalesOrderMap(...)`
      - `looksLikeSalesOrderItemMap(...)`
  - 下钻 `total` 增加兜底：`total<=0` 且已解析出列表时，回落到 `parsedList.length`。
- [x] 完成静态检查
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- [x] 完成 arm64 APK 重打包
  - `flutter build apk --release --target-platform android-arm64 --split-per-abi` ✅
  - 产物：`/Users/admin/Documents/Projects/jxc/app/build/app/outputs/flutter-apk/app-arm64-v8a-release.apk`
  - 大小：约 `23MB`
  - SHA256：`d1b5fa4381d56f8a2805915db9514cbce544d3a021837c60049d02b9f49e1cff`

**当前状态：**
Android 看板订单下钻对深层包裹与字符串化 JSON 场景的兼容性已增强，并已输出新的 arm64 安装包供覆盖安装复测。

## 2026-03-11（Android 看板下钻“当日筛选 total>0 但列表空”兼容性二次修复）

- [x] 完成问题复核与路径确认
  - 现象：开始/结束日期同为当天时，顶部统计 `total>0`，列表偶发显示空。
  - 范围：Android 下钻页解析层（`dashboard_data.dart`）兼容性。
- [x] 完成解析兼容性修复
  - `app/lib/src/features/dashboard/models/dashboard_data.dart`
  - 新增 `toRawIterable(Object?)`，统一兼容 `List / Iterable / Map.values`。
  - 下钻 `list` 与订单 `items` 解析改为统一走 `toRawIterable(...)` 后再 `Map<String,dynamic>.from(...)`。
- [x] 完成静态检查
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
Android 看板订单下钻在“total/list 一致性”方面进一步增强，对运行时 JSON 容器形态的兼容性更稳健，用户反馈的“同日筛选 total 有值但列表为空”问题已完成修复与自检。

## 2026-03-11（Android arm64 APK 重新打包交付）

- [x] 基于最新修复代码重新构建 Android arm64 release 包
  - 执行：`flutter build apk --release --target-platform android-arm64 --split-per-abi` ✅
  - 结果：构建成功（`assembleRelease` 完成）
- [x] 产物路径确认
  - `/Users/admin/Documents/Projects/jxc/app/build/app/outputs/flutter-apk/app-arm64-v8a-release.apk`
  - 产物大小：约 `24.0MB`

**当前状态：**
已产出包含“看板订单下钻 total/list 展示不一致修复”的最新 Android arm64 APK，可直接安装复测。

## 2026-03-11（Android 看板下钻 total/list 展示不一致修复）

- [x] 完成问题定位
  - 现象：下钻页出现“区间共 N 单”但列表显示“暂无订单”。
  - 根因：`dashboard_data.dart` 使用 `whereType<Map<String, dynamic>>()` 过滤运行时 `LinkedHashMap<dynamic,dynamic>`，导致 `list/items` 被误过滤为空。
- [x] 完成代码修复
  - `app/lib/src/features/dashboard/models/dashboard_data.dart`
  - 下钻 `list/items` 解析改为：`whereType<Map>() + Map<String,dynamic>.from(...)`，兼容运行时 Map 类型。
- [x] 完成静态检查
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）

**当前状态：**
Android 看板下钻“total 有值但列表为空”问题已修复，列表与总数口径展示恢复一致。

## 2026-03-11（Android 看板“今日订单数点击无反应”闭环收尾）

- [x] 完成 API 文档先行补齐
  - `API接口定义文档.md` 升级至 `v1.2.12`
  - 在 `5.1.C` 新增 Android 看板跳转联动约定（默认参数、权限提示、日期校验、失败可感知）
- [x] 完成 Flutter 下钻链路落地
  - `app/lib/src/features/dashboard/models/dashboard_data.dart`
  - `app/lib/src/features/dashboard/models/dashboard_repository.dart`
  - `app/lib/src/features/dashboard/application/dashboard_orders_controller.dart`
  - `app/lib/src/features/dashboard/presentation/dashboard_orders_page.dart`
- [x] 完成看板点击入口可达与反馈修复
  - `app/lib/src/features/dashboard/presentation/dashboard_page.dart`
  - “今日订单数”支持整卡点击（非仅数字热区）
  - 无权限点击即时提示；跳转异常可感知提示
  - 下钻参数联动：`start_date=end_date=dashboard.date`，`page=1`，`page_size=min(max(total_orders,1),100)`
- [x] 完成应用层接线收尾
  - `app/lib/src/app.dart`
  - 新增 `DashboardOrdersController` 创建、注入 `DashboardPage` 与 `dispose` 释放
- [x] 完成类型安全收敛
  - `dashboard_page.dart` 中 `_handleOrdersMetricTap` 参数由 `dynamic` 收敛为 `DashboardData`
- [x] 完成静态检查
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
Android 端“今日订单数点击无反应”问题已闭环：文档约束已补齐，点击反馈与下钻链路已完整接通，应用层依赖注入已完成，且 Flutter 静态检查通过，可进入业务验收。

## 2026-03-11（看板“今日订单数点击无反应”交互修复文档链路收尾）

- [x] 完成 Windows 端交互可感知性修复落地
  - `client/src/pages/DashboardPage.vue`
  - 订单数下钻入口支持整卡片点击（不再仅依赖数字文本热区）
  - 无权限点击给出显式提示：`仅 OWNER / SALES 可查看订单下钻`
  - 路由跳转异常捕获并回显失败提示，避免“点击无反应”
- [x] 完成需求文档同步
  - `产品需求文档.md` 升级至 `v1.2.11`
  - `4.7.3` 补充“点击反馈可感知”要求（热区、权限提示）
- [x] 完成架构文档同步
  - `架构设计文档.md` 升级至 `v1.2.11`
  - `6.9` 增补看板下钻点击反馈约束（整卡片点击、无权限提示、跳转失败提示）
  - 新增 `7.1.I Windows 看板订单下钻点击反馈可感知策略`
- [x] 完成 API 文档同步
  - `API接口定义文档.md` 升级至 `v1.2.11`
  - `5.1.C`“Windows 看板跳转联动约定”补充点击反馈可感知条款
- [x] 完成前端类型检查
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅

**当前状态：**
“今日订单数点击无反应”问题已从交互实现与文档契约两侧完成闭环：用户点击可感知、失败可见、权限可解释；PRD / ADD / API 已与当前代码行为一致。

## 2026-03-11（看板订单下钻聚合行文档同步收尾）

- [x] 完成文档口径补齐（ADD / API）
  - `架构设计文档.md`
    - 版本升级至 `v1.2.10`
    - `6.9` 明确下钻按 `biz_no` 聚合输出
    - 补充“无 sales_orders 主记录时返回聚合行”约束：`status=OUTBOUND_ONLY`、`items=[]`、聚合 `remark`
  - `API接口定义文档.md`
    - 版本升级至 `v1.2.10`
    - `5.1.C` 增补聚合行语义与字段约束
    - 响应示例新增 `OUT-LEGACY-DRILL` 聚合行，并同步 `total` 示例
- [x] 完成 OpenAPI 对齐
  - `server/openapi.yaml`
    - 版本升级至 `1.1.1`
    - 新增 `/api/v1/reports/dashboard/orders` 路径定义（鉴权、查询参数、错误码）
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
“点击今日订单数看不到具体列表”问题相关文档链路已全部补齐并与现有后端实现对齐；`total/list` 同口径且支持 legacy `OUT-*` 无主订单聚合行展示，交付闭环完成。

## 2026-03-11（销售出库看板统计口径方案A收敛）

- [x] 完成 reports 口径改造（方案A）
  - `server/src/routes/reports.rs`
  - 新增 `resolve_effective_sell_price`：`snapshot_sell_price` 优先，缺失时 fallback 到 `(biz_no, product_id)` 的销售单明细映射。
  - `get_dashboard_report`：销售额/毛利/热销与订单统计统一使用有效售价口径。
  - `get_dashboard_orders_drilldown`：有效 `OUT_SALE` 判定改为“存在有效售价”；`total` 与 dashboard `total_orders` 对齐为有效 `biz_no` 去重数。
  - `get_sales_report`、`export_sales_report_csv`：销售额改为快照优先，成本/数量与销售额同口径。
- [x] 完成相关测试收敛
  - `server/src/routes/tests/business_flow.rs`
  - `dashboard_report_should_use_stock_log_aligned_metrics` 的毛利润断言调整为 `10.10`，与“计入 legacy 快照售价后”的实际口径一致。
- [x] 完成后端全量回归
  - `cd /Users/admin/Documents/Projects/jxc/server && cargo test -q`
  - 结果：`124 passed; 0 failed`。
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`
- [ ] 待完成远端部署与健康检查

**当前状态：**
“销售出库后销售额、毛利润、订单数不变化”问题在报表域已按方案A完成收敛，且本地后端测试全绿。下一步为部署到远端并完成健康检查验收。

## 2026-03-11（入库提交后表单重置策略优化收尾）

- [x] 完成需求澄清与策略确认
  - 结论：入库提交成功后应清空商品上下文（`product_id/barcode` 与扫码输入），数量恢复默认 `1`。
- [x] 完成文档先行更新（PRD / ADD / API）
  - `产品需求文档.md` 升级到 `v1.2.8`，补充 `4.3.4` 提交后重置策略。
  - `架构设计文档.md` 升级到 `v1.2.8`，补充 `7.1.G / 7.2.G` 双端策略一致性。
  - `API接口定义文档.md` 升级到 `v1.2.8`，补充 `4.1.D` 客户端状态约定。
- [x] 完成 Windows（Vue）实现
  - `client/src/pages/InboundPage.vue`
  - `submit()` 成功后清空 `product_id/barcode`，重置 `qty=1`，清空可选字段并 `resetScanForm()`。
- [x] 完成 Android（Flutter）实现
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
  - `_submit()` 成功后清空商品ID/条码，重置数量 `1`，清空可选字段并 `_resetScan()`。
- [x] 完成静态检查
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- [x] 完成 memory-bank 记录更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
入库提交成功后的表单重置策略已在 Windows + Android 双端完成统一落地：清空商品上下文、数量回到默认 `1`，并通过双端静态检查。该改造仅影响客户端交互层，不涉及后端 API 契约、错误码、权限与幂等语义变更。

## 2026-03-11（看板订单下钻首屏可见性修复收尾）

- [x] 完成问题定位
  - 现象：点击看板“今日订单数”后，下钻页首屏看不到当日全部订单（当订单数 > 20 时尤其明显）。
  - 根因：`DashboardPage.vue` 跳转下钻时固定传 `page_size=20`，并非后端漏数。
- [x] 完成前端修复
  - `client/src/pages/DashboardPage.vue`
  - 下钻首屏分页参数改为动态：`page_size = min(max(total_orders, 1), 100)`。
  - 跳转参数保持：`start_date=end_date=dashboard.date`，`page=1`。
- [x] 完成文档同步（按文档先行要求）
  - `产品需求文档.md`：已为 `v1.2.7`，补充 `4.7.3` 默认筛选策略（首屏 page_size 动态化）。
  - `架构设计文档.md`：升级 `v1.2.7`，补充 `6.9` 联动约束并新增 `7.1.H`。
  - `API接口定义文档.md`：升级 `v1.2.7`，在 `5.1.C` 补充 Windows 看板跳转联动约定。
- [x] 完成 memory-bank 记录更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`
- [x] 完成验证结论沉淀
  - 前端类型检查已通过：`npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅

**当前状态：**
“看板订单下钻看不到今日全部订单”问题已修复并完成文档/记忆库闭环。当前行为为：从看板下钻时首屏尽可能展示全部当日订单（上限 100），超过上限通过分页继续查看。

## 2026-03-11（入库 scan_confirm 确认写入弹窗化：Windows + Android 双端落地）

- [x] 完成文档约束对齐确认
  - 已按 `v1.2.6` 基线执行：
    - `产品需求文档.md` 4.3.4
    - `架构设计文档.md` 7.1.G / 7.2.G
    - `API接口定义文档.md` 4.1.D
- [x] 完成 Windows（Vue）入库确认弹窗化
  - `client/src/pages/InboundPage.vue`
    - `scan_confirm` 命中后改为弹窗确认（替代内联确认区）
    - 数量输入默认 `1`，弹窗打开自动聚焦并选中
    - `Enter` 确认写入、`Esc` 取消
    - 确认成功后自动续扫：清空扫码输入 + 回焦扫码框 + 会话计数递增
  - `client/src/style.css`
    - 新增 `modal-backdrop / modal-card` 样式
- [x] 完成 Android（Flutter）入库确认弹窗化
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
    - 删除 `_showScanConfirm` 内联确认区及 `_ScanCard` 相关参数
    - 新增 `_showScanConfirmDialog()`（`AlertDialog`）承载确认输入
    - 数量输入支持回车确认（`onSubmitted`）
    - 取消后关闭弹窗并回焦扫码输入，支持继续扫码
    - 确认成功后自动续扫：清空扫码输入 + 回焦扫码框 + 会话计数递增
- [x] 完成静态检查
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- [x] 完成 memory-bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
入库 `scan_confirm` 的“确认写入”已在 Windows + Android 双端从内联区改为弹窗，并保持“确认后自动续扫”；本轮未改动后端 API、错误码、权限和幂等语义。

## 2026-03-11（扫码入库首扫数量异常 +1：Web/Android 同步修复）

- [x] 完成问题定位
  - 现象：首扫 1 次数量显示为 2
  - 根因：表单默认 `qty=1`，首扫仍按“当前值 + 1”累加
- [x] 完成 Web 修复
  - `client/src/pages/InboundPage.vue`
  - `applyScannedInbound`：首扫（当前未绑定商品）使用 `baseQty=0` 后再加本次扫码数量
- [x] 完成 Android 修复
  - `app/lib/src/features/inventory/application/inbound_controller.dart`
  - `_applyScannedToForm`：首扫（当前未绑定商品）使用 `baseQty=0` 后再加本次扫码数量
- [x] 完成回归验证
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- [x] 完成 Memory Bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
扫码入库“首扫变 2”问题已在 Web + Android 双端修复，当前行为为首扫=1、同商品后续扫码正常递增。

## 2026-03-11（经营看板订单数下钻：默认当天 + 日期范围筛选）

- [x] 完成后端路由挂载
  - `server/src/routes/mod.rs`
  - 新增 `GET /reports/dashboard/orders`
- [x] 完成前端类型与 API 封装
  - `client/src/types/api.ts`：新增下钻 Query/Data 类型
  - `client/src/api/inventory.ts`：新增 `getDashboardOrdersDrilldownApi`
- [x] 完成看板入口交互
  - `client/src/pages/DashboardPage.vue`
  - 订单数支持点击跳转销售单页，并默认携带当天 `start_date/end_date`
- [x] 完成销售单页下钻能力
  - `client/src/pages/SalesOrdersPage.vue`
  - 新增下钻查询区、日期范围筛选、分页查询、路由 query 初始化
- [x] 完成后端口径一致性与分页测试
  - `server/src/routes/tests/business_flow.rs`
  - 新增 `dashboard_orders_drilldown_should_align_with_dashboard_total_orders_and_support_paging`
- [x] 完成本地验证
  - `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml routes::tests::business_flow -- --nocapture` ✅
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
- [x] 完成远端部署与健康检查
  - `bash /Users/admin/Documents/Projects/jxc/server/scripts/deploy_remote.sh` ✅
  - 远端测试：`124 passed; 0 failed`
  - release 构建完成，`jxc-server.service` 重启并 `active (running)`
  - `http://127.0.0.1:8080/health` 返回 200
- [x] 完成 Memory Bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
经营看板“订单数下钻（默认当天 + 日期范围筛选）”已完成代码、测试、部署和文档沉淀，可进入业务验收。

## 2026-03-11（经营看板全指标口径修复收尾）

- [x] 完成看板统计实现修复
  - `server/src/routes/reports.rs`
  - 订单数改为“当日有效 `OUT_SALE` 且可映射销售明细”的唯一 `biz_no` 计数
  - 销售额/毛利/热销统计增加 `(biz_no, product_id) -> sell_price` 映射过滤，忽略无法映射的遗留流水
- [x] 完成看板口径专项测试补齐
  - `server/src/routes/tests/business_flow.rs`
  - 新增 `dashboard_report_should_use_stock_log_aligned_metrics`
  - 覆盖：退货冲减、低库存、热销、已确认无流水订单不计数、遗留流水忽略
- [x] 完成回归验证
  - `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml routes::tests` ✅（109 passed）
  - `cargo test --manifest-path /Users/admin/Documents/Projects/jxc/server/Cargo.toml` ✅（123 passed）
- [x] 完成文档口径同步（按“文档先行”）
  - `产品需求文档.md` 升级到 `v1.2.4`，新增 `4.7.2 经营看板统计口径统一`
  - `架构设计文档.md` 升级到 `v1.2.4`，新增 `6.8 经营看板统计口径约束`
  - `API接口定义文档.md` 升级到 `v1.2.4`，新增 `5.1.B 经营看板统计口径统一约定`
- [x] 完成 Memory Bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
经营看板 `total_sales / total_gross_profit / total_orders / low_stock_count / top_selling_item` 统计口径已统一，代码、测试与文档已对齐并完成回归验证。

## 2026-03-11（扫码模式 Tab 化 + 主按钮智能摄像头触发：双端收尾）

- [x] 完成文档先行约束下的实现收尾（延续已更新到 v1.2.3 的 PRD/架构/API 文档）
- [x] 完成 Web 扫码模式 Tab 化
  - `client/src/pages/InboundPage.vue`：扫码模式下拉改为并排 Tab 按钮
  - `client/src/pages/OutboundPage.vue`：扫码模式下拉改为并排 Tab 按钮
  - `client/src/style.css`：新增统一样式 `.scan-mode-tabs / .scan-mode-tab / .scan-mode-tab.is-active`
- [x] 完成 Flutter 入库页扫码模式 Tab 化与智能触发
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
  - `DropdownButtonFormField` 改为 `ChoiceChip + Wrap`
  - 新增 `_onPrimaryScanPressed()`：非连续模式且条码为空时，主按钮直接拉起摄像头
- [x] 完成 Flutter 出库页扫码模式 Tab 化与智能触发
  - `app/lib/src/features/inventory/presentation/outbound_page.dart`
  - `DropdownButtonFormField` 改为 `ChoiceChip + Wrap`
  - 新增 `_onPrimaryScanPressed()` 与 `_onScanModeChanged()`，收敛模式切换与智能触发逻辑
- [x] 完成静态检查
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
- [x] 完成 Memory Bank 更新
  - `memory-bank/activeContext.md`
  - `memory-bank/progress.md`

**当前状态：**
“扫码模式 Tab 化 + Android 主按钮空输入智能拉起摄像头”已在 Windows + Android 双端完成落地并通过静态检查；不涉及后端 API 契约变更。

## 2026-03-11（扫码模式记忆 + 确认后自动续扫：Android 收尾完成）

- [x] 完成 Android 入库页扫码模式记忆接入
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
  - 初始化读取 `readScanMode(scope,'inbound')`
  - 模式切换持久化 `writeScanMode(scope,'inbound', mode)`
- [x] 完成 Android 出库页扫码模式记忆接入
  - `app/lib/src/features/inventory/presentation/outbound_page.dart`
  - 初始化读取 `readScanMode(scope,'outbound')`
  - 模式切换持久化 `writeScanMode(scope,'outbound', mode)`
- [x] 完成 Android 入库/出库 `scan_confirm` 确认后自动续扫会话
  - 新增会话状态：进行中标识 + 已处理计数
  - 新增“结束确认续扫”操作
  - 确认成功后自动清空条码输入并聚焦，支持扫码枪连续回车
- [x] 完成依赖注入链路打通
  - `app/lib/src/features/dashboard/presentation/dashboard_page.dart` 注入 `sessionStorage` 与 `scanPreferenceScope`
  - `app/lib/src/app.dart` 向 `DashboardPage` 注入 `sessionStorage`
- [x] 完成静态检查
  - `flutter analyze /Users/admin/Documents/Projects/jxc/app` ✅（No issues found）
  - `npm run typecheck --prefix /Users/admin/Documents/Projects/jxc/client` ✅

**当前状态：**
“扫码模式记忆 + 扫码后确认自动续扫”已在 Windows + Android 双端全部落地并通过静态检查，可进入验收。

## 2026-03-11（扫码文案与行为一致性收敛：Windows + Android）

- [x] 完成文档约束对齐并按规范执行（仅文案层改造）
  - 已遵循 `产品需求文档.md` 4.11、`架构设计文档.md` 6.6、`API接口定义文档.md` 11 的统一规则：
    - 仅摄像头入口使用“扫码”；
    - 输入框条码处理动作统一为“按条码…”。
- [x] 完成 Android（Flutter）库存三页误导文案修正
  - `app/lib/src/features/inventory/presentation/inbound_page.dart`
    - `扫码入库` -> `条码入库`
    - `扫码并处理` -> `按条码处理`
    - tooltip `扫码并累加` -> `摄像头扫码`
  - `app/lib/src/features/inventory/presentation/outbound_page.dart`
    - `扫码出库` -> `条码出库`
    - `扫码并处理` -> `按条码处理`
    - tooltip `扫码出库` -> `摄像头扫码`
  - `app/lib/src/features/inventory/presentation/stock_check_page.dart`
    - `扫码选品` -> `条码选品`
    - `扫码并加入明细` -> `按条码加入明细`
    - tooltip `扫码并加入明细` -> `摄像头扫码加入明细`
- [x] 完成双端残留检索
  - `search_files(app/lib/src, regex=