# 极速云进销存 - Windows 客户端（Tauri + Vue3）

> 对齐文档基线：`产品需求文档.md`、`架构设计文档.md`、`API接口定义文档.md`

## 1. 项目说明

本目录为 JXC Windows 客户端（Tauri + Vue3 + TypeScript）代码。

当前阶段为 **MVP 第一阶段**，目标功能：

- 登录（`/api/v1/auth/login`）
- 经营看板（`/api/v1/reports/dashboard`）
- 商品管理（列表 + 创建，`/api/v1/products`）
- 采购入库（`/api/v1/inventory/inbound`）
- 采购单状态流（`/api/v1/purchase-orders*`）
- 库存盘点状态流（`/api/v1/inventory/stock-checks*`）
- 销售单状态流（`/api/v1/sales-orders*`）
- 销售出库（`/api/v1/inventory/outbound`）
- 低库存预警（`/api/v1/inventory/alerts/low-stock`）
- 销售报表（`/api/v1/reports/sales`、`/api/v1/reports/sales/export`）
- 库存流水（`/api/v1/inventory/logs`）
- 审计日志（`/api/v1/audit/logs`）

## 2. 运行要求

- Node.js: 建议 `>= 20`（当前本地 19 可运行，但会出现 engine warning）
- npm: `>= 9`
- Rust + Cargo（用于 Tauri）

## 3. 环境变量

在 `client/` 下创建 `.env.local`（开发环境）或 `.env.production`（打包环境）：

```bash
VITE_API_BASE_URL=http://1.14.45.242:8080/api/v1
```

说明：
- 若未配置 `VITE_API_BASE_URL`，客户端会默认使用 `http://1.14.45.242:8080/api/v1`。
- 建议在不同环境通过 `.env.*` 显式覆盖，避免误连到本机 `localhost`。

## 4. 开发命令

```bash
# 安装依赖
npm install

# 前端开发模式
npm run dev

# 类型检查
npm run typecheck

# 前端构建
npm run build

# Tauri 桌面开发
npm run tauri:dev

# Tauri 桌面打包
npm run tauri:build

# Windows 便携版（可在 macOS 上交叉构建）
npm run tauri:build:win:portable

# Windows 便携版（构建 + 产物归档 + sha256）
npm run tauri:build:win:portable:dist

# Windows NSIS 安装包（需在 Windows 主机/CI 上执行）
npm run tauri:build:win:nsis
```

## 5. 契约约束（必须遵守）

- 统一响应结构：`code/message/data/request_id`
- 鉴权：`Authorization: Bearer <access_token>`
- 写接口幂等：`X-Idempotency-Key`
- 金额字段使用字符串传输（如 `"12.50"`）

## 6. 目录结构（MVP）

```text
client/
├─ src/
│  ├─ api/            # API SDK 与请求封装
│  ├─ components/     # 复用组件
│  ├─ pages/          # 页面（登录/商品/出库/预警）
│  ├─ router/         # 路由
│  ├─ stores/         # Pinia 状态
│  ├─ types/          # TS 类型定义
│  ├─ App.vue
│  └─ main.ts
├─ src-tauri/         # Tauri Rust 壳工程
└─ package.json
```

## 7. 最新迭代说明（2026-03-06）

- 新增采购入库页面：`client/src/pages/InboundPage.vue`
- 新增入库 API SDK：`inboundApi`（`client/src/api/inventory.ts`）
- 新增入库类型定义：`InboundRequest`、`InboundResponseData`（`client/src/types/api.ts`）
- 路由与导航接线：
  - 路由：`/inbound`
  - 侧边栏菜单：`采购入库`
- 交互规则对齐后端契约：
  - `product_id` / `barcode` 至少填写一个
  - `qty` 必须为正整数
  - `unit_cost` 金额字符串（最多 4 位小数）
  - 支持 `expected_version`（可选）
  - 错误提示展示 `code` 与 `request_id`

## 8. 最新迭代说明（2026-03-06，采购单状态流）

- 新增采购单页面：`client/src/pages/PurchaseOrdersPage.vue`
  - 能力覆盖：创建采购单、按 ID 查询、确认采购单、作废采购单
- 新增采购单 API SDK（`client/src/api/inventory.ts`）：
  - `createPurchaseOrderApi`
  - `getPurchaseOrderApi`
  - `confirmPurchaseOrderApi`
  - `voidPurchaseOrderApi`
- 新增采购单类型定义（`client/src/types/api.ts`）：
  - `PurchaseOrderCreateItemRequest`
  - `PurchaseOrderCreateRequest`
  - `OrderActionRequest`
  - `PurchaseOrderItemData`
  - `PurchaseOrderData`
- 路由与导航接线：
  - 路由：`/purchase-orders`
  - 侧边栏菜单：`采购单状态流`
- 前端规则与提示：
  - 角色限制：仅 `OWNER/PURCHASER` 可执行创建/确认/作废
  - 创建校验：`biz_no` 必填、`items` 至少一行、`product_id/qty` 正整数、`unit_cost` 金额字符串
  - 动作校验：`expected_version`（可选）为 `>=0` 整数
  - 错误展示统一包含：`message + code + request_id`

## 9. 最新迭代说明（2026-03-06，销售单状态流）

- 新增销售单页面：`client/src/pages/SalesOrdersPage.vue`
  - 能力覆盖：创建销售单、按 ID 查询、确认销售单、作废销售单、销售退货
- 新增销售单 API SDK（`client/src/api/inventory.ts`）：
  - `createSalesOrderApi`
  - `getSalesOrderApi`
  - `confirmSalesOrderApi`
  - `voidSalesOrderApi`
  - `returnSalesOrderApi`
- 新增销售单类型定义（`client/src/types/api.ts`）：
  - `SalesOrderCreateItemRequest`
  - `SalesOrderCreateRequest`
  - `SalesOrderReturnItemRequest`
  - `SalesOrderReturnRequest`
  - `SalesOrderItemData`
  - `SalesOrderData`
- 路由与导航接线：
  - 路由：`/sales-orders`
  - 侧边栏菜单：`销售单状态流`
- 前端规则与提示：
  - 角色限制：仅 `OWNER/SALES` 可执行创建/确认/作废/退货
  - 创建校验：`biz_no` 必填、`items` 至少一行、`product_id/qty` 正整数、`sell_price` 金额字符串
  - 动作校验：`expected_version`（可选）为 `>=0` 整数
  - 退货校验：`items` 至少一行、`product_id/qty` 正整数
  - 错误展示统一包含：`message + code + request_id`

## 10. 最新迭代说明（2026-03-06，经营看板）

- 新增经营看板页面：`client/src/pages/DashboardPage.vue`
  - 能力覆盖：按日期查询经营指标并展示卡片化汇总
- 新增看板 API SDK（`client/src/api/inventory.ts`）：
  - `getDashboardApi`
- 新增看板类型定义（`client/src/types/api.ts`）：
  - `DashboardQuery`
  - `DashboardData`
- 路由与导航接线：
  - 路由：`/dashboard`
  - 默认首页：登录后与根路径均跳转到 `/dashboard`
  - 侧边栏菜单：`经营看板`
- 前端规则与提示：
  - 日期参数校验：`YYYY-MM-DD`（可留空，留空按服务端默认口径）
  - 错误展示统一包含：`message + code + request_id`
  - 指标展示：`total_sales/total_gross_profit/total_orders/low_stock_count/top_selling_item`

## 11. 最新迭代说明（2026-03-06，销售报表）

- 新增销售报表页面：`client/src/pages/SalesReportPage.vue`
  - 能力覆盖：按日期范围查询销售报表、分页查看、汇总展示、CSV/XLSX 导出
- 新增销售报表 API SDK（`client/src/api/inventory.ts`）：
  - `getSalesReportApi`
  - `exportSalesReportApi`
- 新增销售报表类型定义（`client/src/types/api.ts`）：
  - `SalesReportQuery`
  - `SalesReportItemData`
  - `SalesReportSummaryData`
  - `SalesReportData`
  - `SalesReportExportQuery`
  - `SalesReportExportResult`
- 路由与导航接线：
  - 路由：`/sales-report`
  - 侧边栏菜单：`销售报表`
- 前端规则与提示：
  - `group_by` 固定按 `product`
  - `start_date/end_date` 必须成对提供，或同时留空（留空由服务端按当天口径处理）
  - 日期格式校验：`YYYY-MM-DD`
  - 导出权限提示：仅 `OWNER/PURCHASER` 可导出
  - 导出格式支持：`csv/xlsx`
  - 错误展示统一包含：`message + code + request_id`

## 12. 最新迭代说明（2026-03-06，库存盘点状态流）

- 新增库存盘点页面：`client/src/pages/StockChecksPage.vue`
  - 能力覆盖：创建盘点单、按 ID 查询、开始盘点、确认盘点
- 新增库存盘点 API SDK（`client/src/api/inventory.ts`）：
  - `createStockCheckApi`
  - `getStockCheckApi`
  - `startStockCheckApi`
  - `confirmStockCheckApi`
- 新增库存盘点类型定义（`client/src/types/api.ts`）：
  - `StockCheckCreateItemRequest`
  - `StockCheckCreateRequest`
  - `StockCheckConfirmItemRequest`
  - `StockCheckConfirmRequest`
  - `StockCheckItemData`
  - `StockCheckData`
- 路由与导航接线：
  - 路由：`/stock-checks`
  - 侧边栏菜单：`库存盘点状态流`
- 前端规则与提示：
  - 角色限制：仅 `OWNER/PURCHASER` 可执行创建/开始/确认
  - 创建校验：`biz_no` 必填、`items` 至少一行、`product_id` 正整数且不重复
  - 开始校验：`expected_version`（可选）需为 `>=0` 整数，且仅 `DRAFT` 可开始
  - 确认校验：`expected_version`（可选）需为 `>=0` 整数，且仅 `COUNTING` 可确认
  - 确认明细校验：必须与盘点单商品集合一致，`actual_stock` 为 `>=0` 整数
  - 错误展示统一包含：`message + code + request_id`

## 13. 最新迭代说明（2026-03-06，商品管理：创建商品属性）

- 已在商品页面补齐“创建商品属性”入口：`client/src/pages/ProductsPage.vue`
  - 页面能力：创建商品 + 查询商品列表（分页）
- 新增商品创建 API SDK（`client/src/api/products.ts`）：
  - `createProductApi`
- 新增商品创建类型定义（`client/src/types/api.ts`）：
  - `CreateProductRequest`
- 创建字段与后端契约对齐（`server/src/routes.rs::CreateProductRequest`）：
  - `sku?`、`barcode`、`name`、`unit`
  - `retail_price`、`wholesale_price`
  - `init_stock?`、`min_stock_limit?`、`cost_price?`
- 前端校验与提示：
  - 必填：`barcode/name/unit/retail_price/wholesale_price`
  - 金额格式：最多 4 位小数（与全局金额规范一致）
  - 库存/阈值：`>=0` 整数
  - 权限提示：仅 `OWNER/PURCHASER` 可创建
  - 错误展示统一包含：`message + code + request_id`
  - 创建成功后自动刷新商品列表

## 14. 最新迭代说明（2026-03-06，商品管理：编辑/删除）

- 已在商品页面补齐“编辑商品属性 + 删除商品”能力：`client/src/pages/ProductsPage.vue`
  - 页面能力：列表查询 + 创建 + 编辑 + 删除（同页闭环）
- 新增商品编辑/删除 API SDK（`client/src/api/products.ts`）：
  - `updateProductApi(id, payload)` -> `PUT /products/{id}`
  - `deleteProductApi(id, query?)` -> `DELETE /products/{id}`
- 编辑字段与后端契约对齐（`server/src/routes.rs::UpdateProductRequest`）：
  - `sku/barcode/name/unit/retail_price/wholesale_price/min_stock_limit`
  - `expected_version`（用于乐观锁）
- 删除字段与后端契约对齐（`server/src/routes.rs::DeleteProductQuery`）：
  - `expected_version`（用于乐观锁）
- 前端交互与错误语义：
  - 操作列新增：`编辑`、`删除`
  - 编辑校验：必填字段、金额格式、阈值整数、`expected_version >= 0`
  - 删除默认携带当前行 `version` 作为 `expected_version`
  - 错误展示统一包含：`message + code + request_id`
  - 针对关键冲突给出可读提示并自动刷新列表：
    - `4091`：版本冲突（提示当前版本并引导刷新）
    - `4090`：有库存不可删（展示 `current_stock`）
    - `4030`：无权限操作

## 15. 最新迭代说明（2026-03-06，库存追溯闭环）

- 补齐了服务端库存流水查询 API：`GET /api/v1/inventory/logs`。
- 新增审计日志页面：`client/src/pages/AuditLogsPage.vue`。
  - 支持按动作、目标、操作人、Request ID 与日期范围过滤。
  - 权限：仅 `OWNER` 可见。
- 新增库存流水页面：`client/src/pages/StockLogsPage.vue`。
  - 支持按业务类型、单号、商品ID、操作人与日期范围过滤。
  - 权限：`OWNER` 与 `PURCHASER` 可见。
- 更新了主菜单导航，接入两个新页面。
- 统一了错误展示格式：`message + code + request_id`。

## 16. 最新迭代说明（2026-03-07，Windows 打包策略）

- 已确认当前开发机（macOS）下 Tauri CLI 的 `--bundles` 可选项不包含 `nsis`，因此**无法在本机直接产出 NSIS 安装包**。
- 已补充 npm 脚本：
  - `tauri:build:win:portable`：产出 Windows 可执行文件（`app.exe`）。
  - `tauri:build:win:portable:dist`：在上述基础上自动生成 `dist-windows` 便携包与 `sha256`。
  - `tauri:build:win:nsis`：用于 Windows 主机/CI 产出 NSIS 安装包。
- 新增便携包脚本：`client/scripts/package_windows_portable.sh`
  - 输入：`src-tauri/target/x86_64-pc-windows-gnu/release/app.exe`
  - 输出：`dist-windows/app.exe`、`dist-windows/jxc-windows-x64-portable.zip`、`dist-windows/jxc-windows-x64-portable.zip.sha256`
- 已补充 CI 工作流：`.github/workflows/client-windows-installer.yml`
  - 在 `windows-latest` 执行构建；
  - 产出 NSIS installer 与 portable 包（zip + sha256）。

### 16.1 本机（macOS）可执行命令

```bash
cd /Users/admin/Documents/Projects/jxc/client
npm run tauri:build:win:portable:dist
```

产物路径：

- `client/src-tauri/target/x86_64-pc-windows-gnu/release/app.exe`

### 16.2 Windows 主机/CI 执行命令（NSIS）

```bash
cd client
npm ci
npm run tauri:build:win:nsis
```

典型产物路径（Windows）：

- `client/src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/*.exe`
