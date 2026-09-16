# JXC Server (M3)

基于 Rust + Axum 的进销存后端（M3 收口版，默认 Postgres 主链路）。

## 当前已实现

- 统一响应结构：`code/message/data/request_id`
- 基础链路：
  - `GET /health`
  - `POST /api/v1/auth/login`
  - `POST /api/v1/auth/refresh`
  - `POST /api/v1/auth/logout`
  - `GET /api/v1/products/scan?barcode=...`
  - `POST /api/v1/products`
  - `GET /api/v1/products`
  - `GET /api/v1/products/:id`
  - `PUT /api/v1/products/:id`
  - `DELETE /api/v1/products/:id`
  - `POST /api/v1/purchase-orders`
  - `GET /api/v1/purchase-orders/:id`
  - `POST /api/v1/purchase-orders/:id/confirm`
  - `POST /api/v1/purchase-orders/:id/void`
  - `POST /api/v1/sales-orders`
  - `GET /api/v1/sales-orders/:id`
  - `POST /api/v1/sales-orders/:id/confirm`
  - `POST /api/v1/sales-orders/:id/void`
  - `POST /api/v1/sales-orders/:id/return`
  - `POST /api/v1/inventory/stock-checks`
  - `GET /api/v1/inventory/stock-checks/:id`
  - `POST /api/v1/inventory/stock-checks/:id/start`
  - `POST /api/v1/inventory/stock-checks/:id/confirm`
  - `GET /api/v1/inventory/alerts/low-stock`
  - `GET /api/v1/reports/dashboard`
  - `GET /api/v1/reports/sales`
  - `GET /api/v1/reports/sales/export`
  - `GET /api/v1/audit/logs`
  - `POST /api/v1/inventory/inbound`
  - `POST /api/v1/inventory/outbound`
- JWT 鉴权（Access/Refresh）
- 多租户上下文校验（`tenant_id`）
- 角色权限（M1）：
  - 入库：`OWNER` / `PURCHASER`
  - 出库：`OWNER` / `SALES`
  - 采购单（创建/确认/作废）：`OWNER` / `PURCHASER`
  - 销售单（创建/确认/作废/退货）：`OWNER` / `SALES`
  - 盘点单（创建/开始/确认）：`OWNER` / `PURCHASER`
  - 审计日志查询：`OWNER`
  - 销售报表导出：`OWNER` / `PURCHASER`
  - 扫码查品：`SALES` 隐藏 `cost_price`
- 幂等支持：`X-Idempotency-Key`
  - 重放：返回首次结果 + `X-Idempotent-Replay: true`
  - 同 key 异体：`4092`
- 版本冲突：`4091`（包含最新快照）
- 商品规则（M1.1）：
  - 条码租户内唯一，重复返回 `4002`
  - 商品软删除（`is_deleted`），已删除商品对外不可见
  - 有库存商品不可删除（返回 `4090`）
  - 商品修改/删除支持 `expected_version` 乐观并发校验（`4091`）
- 单据状态机与可逆规则（M1.2）：
  - 采购单：`DRAFT -> CONFIRMED -> VOIDED`
  - 销售单：`DRAFT -> CONFIRMED -> RETURNED_PARTIAL/RETURNED_FULL`，草稿可作废为 `VOIDED`
  - 采购确认：写 `IN_PURCHASE` 流水并更新加权成本
  - 采购作废（已确认）：写 `VOID_PURCHASE` 反向流水
  - 销售确认：写 `OUT_SALE` 流水
  - 销售退货：写 `RETURN_SALE` 流水并更新退货状态
  - 单据动作支持 `expected_version` 冲突校验（`4091`）
- 盘点状态机与差异入账（M2.1）：
  - 盘点单：`DRAFT -> COUNTING -> CONFIRMED`
  - 开始/确认动作支持 `expected_version` 冲突校验（`4091`）
  - 盘点确认校验“盘点基准库存（book_stock）未漂移”，漂移返回 `4091`
  - 盘点确认按差异写库存流水：`ADJ_CHECK`（`delta_qty != 0`）
- 库存预警（M2.2）：
  - `GET /api/v1/inventory/alerts/low-stock`
  - 支持 `page/page_size/keyword/only_active` 查询参数
  - 默认 `only_active=true`，仅返回 `current_stock < min_stock_limit` 的商品
  - 返回聚合字段：`active_low_stock_count`
- 审计增强（M1.2）：
  - 单据关键动作记录 `before_data/after_data + request_id` 到 `audit_logs`

## 存储后端与单机模式（v1.7.0 新增）

服务端支持三种可插拔存储后端，由 `STORAGE_BACKEND` 选择：

| 后端 | 用途 | 说明 |
| --- | --- | --- |
| `postgres` | 多租户 SaaS（默认） | 主链路存储，配合 Redis 做幂等与缓存 |
| `sqlite` | 单机版（本地模式） | 桌面端内嵌服务端使用，数据落在本地文件，无需服务器 |
| `memory` | 本地冒烟 / 测试 | 进程内存储，重启后数据丢失 |

单机模式相关配置：

| 变量 | 默认值 | 说明 |
| --- | --- | --- |
| `STORAGE_BACKEND` | `postgres` | 设为 `sqlite` 启用单机模式 |
| `SQLITE_PATH` | `jxc.db` | SQLite 数据文件路径 |

单机模式启动示例：

```bash
cd server
STORAGE_BACKEND=sqlite SQLITE_PATH=jxc.db cargo run
```

启动时行为差异：

1. 自动应用 `migrations/sqlite/*.sql`（与 PostgreSQL 版一一等价的迁移脚本）。
2. 首次启动自动创建单机租户（租户码 `local`）与管理员账号，并在日志中给出初始账号提示。
3. 登录时租户码可选，未提供时回落 `local`；`postgres` 模式仍强制要求租户码。

单机版整包（内嵌服务端 + 前端）由仓库根目录 `scripts/build-local.sh` 构建。

## 快速启动

```bash
cd /Users/admin/Documents/Projects/jxc/server
cp .env.example .env
cargo run
```

默认监听：`http://0.0.0.0:8080`

## 示例调用

### 1) 健康检查

```bash
curl -s http://127.0.0.1:8080/health
```

### 2) 登录

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}'
```

### 3) 扫码查询商品

```bash
curl -s "http://127.0.0.1:8080/api/v1/products/scan?barcode=690123456789" \
  -H "Authorization: Bearer <access_token>"
```

### 4) 入库（幂等）

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/inventory/inbound \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-in-001" \
  -d '{"product_id":1001,"qty":5,"unit_cost":"2.20","expected_version":1,"biz_no":"PO-TEST-001"}'
```

### 5) 新建商品（幂等）

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/products \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-prod-create-001" \
  -d '{"barcode":"699000000001","name":"测试商品","unit":"件","retail_price":"9.90","wholesale_price":"8.80","init_stock":0,"min_stock_limit":2}'
```

### 6) 商品列表

```bash
curl -s "http://127.0.0.1:8080/api/v1/products?page=1&page_size=20&keyword=测试" \
  -H "Authorization: Bearer <access_token>"
```

### 7) 更新商品

```bash
curl -s -X PUT "http://127.0.0.1:8080/api/v1/products/2001" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-prod-update-001" \
  -d '{"name":"测试商品-更新","expected_version":1}'
```

### 8) 删除商品（软删除）

```bash
curl -s -X DELETE "http://127.0.0.1:8080/api/v1/products/2001?expected_version=2" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-prod-delete-001"
```

### 9) 出库

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/inventory/outbound \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-out-001" \
  -d '{"biz_no":"SO-TEST-001","expected_version":2,"items":[{"product_id":1001,"qty":2,"sell_price":"3.50"}]}'
```

### 10) 创建采购单

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/purchase-orders \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-po-create-001" \
  -d '{"biz_no":"PO-TEST-001","items":[{"product_id":1001,"qty":5,"unit_cost":"2.20"}],"remark":"测试采购"}'
```

### 11) 确认采购单

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/purchase-orders/3001/confirm \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-po-confirm-001" \
  -d '{"expected_version":1}'
```

### 12) 作废采购单（已确认会生成反向流水）

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/purchase-orders/3001/void \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-po-void-001" \
  -d '{"expected_version":2}'
```

### 13) 创建销售单

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/sales-orders \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-so-create-001" \
  -d '{"biz_no":"SO-TEST-002","items":[{"product_id":1001,"qty":3,"sell_price":"3.50"}]}'
```

### 14) 确认销售单

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/sales-orders/4001/confirm \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-so-confirm-001" \
  -d '{"expected_version":1}'
```

### 15) 销售退货（部分/全部）

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/sales-orders/4001/return \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-so-return-001" \
  -d '{"expected_version":2,"items":[{"product_id":1001,"qty":1}],"remark":"测试退货"}'
```

### 16) 作废销售草稿

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/sales-orders/4002/void \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-so-void-001" \
  -d '{"expected_version":1}'
```

### 17) 创建盘点单

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/inventory/stock-checks \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-sc-create-001" \
  -d '{"biz_no":"SC-TEST-001","items":[{"product_id":1001}],"remark":"月末盘点"}'
```

### 18) 开始盘点（DRAFT -> COUNTING）

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/inventory/stock-checks/5001/start \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-sc-start-001" \
  -d '{"expected_version":1}'
```

### 19) 确认盘点（COUNTING -> CONFIRMED，写 ADJ_CHECK）

```bash
curl -s -X POST http://127.0.0.1:8080/api/v1/inventory/stock-checks/5001/confirm \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <access_token>" \
  -H "X-Idempotency-Key: idem-sc-confirm-001" \
  -d '{"expected_version":2,"items":[{"product_id":1001,"actual_stock":103}],"remark":"复核完成"}'
```

### 20) 查询盘点单

```bash
curl -s "http://127.0.0.1:8080/api/v1/inventory/stock-checks/5001" \
  -H "Authorization: Bearer <access_token>"
```

### 21) 查询低库存预警

```bash
curl -s "http://127.0.0.1:8080/api/v1/inventory/alerts/low-stock?page=1&page_size=20&only_active=true&keyword=可乐" \
  -H "Authorization: Bearer <access_token>"
```

返回字段示例：

- `list`: 预警商品列表（含 `shortage_qty`）
- `total`: 当前查询条件下总数
- `active_low_stock_count`: 全量预警商品数（`shortage_qty > 0`）
- `page/page_size`: 分页信息

### 22) 查询经营看板

```bash
curl -s "http://127.0.0.1:8080/api/v1/reports/dashboard?date=2026-03-02" \
  -H "Authorization: Bearer <access_token>"
```

返回字段示例：

- `date`: 报表日期（`YYYY-MM-DD`）
- `total_sales`: 当日净销售额（销售额-退货冲减）
- `total_gross_profit`: 当日毛利（`total_sales - 销售成本`，含退货冲减）
- `total_orders`: 当日已确认销售单数量
- `low_stock_count`: 当前低库存商品数（`current_stock < min_stock_limit`）
- `top_selling_item`: 当日净销量最高商品名称（无数据时为 `暂无`）

### 23) 查询销售报表

```bash
curl -s "http://127.0.0.1:8080/api/v1/reports/sales?start_date=2026-03-03&end_date=2026-03-03&group_by=product&page=1&page_size=20" \
  -H "Authorization: Bearer <access_token>"
```

返回字段示例：

- `group_by`: 当前分组维度（现仅支持 `product`）
- `start_date/end_date`: 报表时间范围（`YYYY-MM-DD`）
- `list`: 报表明细（`product_id/product_name/total_qty/total_sales/total_cost/gross_profit`）
- `summary`: 汇总（`total_sales/total_cost/total_gross_profit/total_qty`）
- `total/page/page_size`: 分页信息

### 24) 查询审计日志

```bash
curl -s "http://127.0.0.1:8080/api/v1/audit/logs?page=1&page_size=20&action=STOCK_CHECK_START&target_type=stock_check&start_date=2026-03-03&end_date=2026-03-03" \
  -H "Authorization: Bearer <access_token>"
```

查询参数：

- `page/page_size`: 分页（`page_size` 最大 100）
- `action`: 按动作过滤（如 `STOCK_CHECK_START`）
- `target_type`: 按目标类型过滤（如 `stock_check`）
- `operator_id`: 按操作人 UUID 过滤
- `request_id`: 按请求链路 ID 精确过滤
- `start_date/end_date`: 日期范围过滤（`YYYY-MM-DD`，需同时提供）

返回字段：

- `list`: 审计列表（`id/operator_id/action/target_type/target_id/before_data/after_data/request_id/created_at`）
- `total/page/page_size`: 分页信息

### 25) 导出销售报表（CSV/XLSX）

```bash
# 导出 XLSX
curl -sS -D /tmp/export_headers_xlsx.txt -o /tmp/sales_report.xlsx \
  "http://127.0.0.1:8080/api/v1/reports/sales/export?start_date=2026-03-03&end_date=2026-03-03&group_by=product&format=xlsx" \
  -H "Authorization: Bearer <access_token>"

# 导出 CSV
curl -sS -D /tmp/export_headers_csv.txt -o /tmp/sales_report.csv \
  "http://127.0.0.1:8080/api/v1/reports/sales/export?start_date=2026-03-03&end_date=2026-03-03&group_by=product&format=csv" \
  -H "Authorization: Bearer <access_token>"
```

查询参数：

- `start_date/end_date`: 日期范围（`YYYY-MM-DD`，需同时提供；不提供则默认当天）
- `group_by`: 当前仅支持 `product`
- `format`: 支持 `csv` / `xlsx`（默认 `csv`）

响应说明：

- HTTP `200` 返回文件流
- 当 `format=csv`：
  - `Content-Type: text/csv; charset=utf-8`
  - `Content-Disposition: attachment; filename="sales_report_<start>_<end>.csv"`
  - 文件为 UTF-8 BOM 的 CSV
  - CSV 列：`product_id,product_name,total_qty,total_sales,total_cost,gross_profit`
  - 尾行 `SUMMARY` 为汇总：`SUMMARY,,total_qty,total_sales,total_cost,total_gross_profit`
- 当 `format=xlsx`：
  - `Content-Type: application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`
  - `Content-Disposition: attachment; filename="sales_report_<start>_<end>.xlsx"`
  - Sheet1 包含与 CSV 同口径字段与 `SUMMARY` 汇总行

## M2.1 本地冒烟结果（2026-03-02）

- 创建盘点单：`200`，状态 `DRAFT`
- 开始盘点：`200`，状态 `COUNTING`
- 确认盘点：`200`，状态 `CONFIRMED`
- 库存变化：`100 -> 103`，盘点差异 `delta=3`
- 幂等重放头：`x-idempotent-replay: true`（start / confirm）
- 冲突码验证：`4091`（版本冲突）、`4092`（幂等请求体冲突）

## M2.2 本地冒烟结果（2026-03-02）

- 低库存测试商品创建：`create_code=200`，`product_id=2001`
- 默认查询（`only_active=true`）：`code=200 total=1 active_low_stock_count=1`
- 包含非预警商品（`only_active=false`）：`code=200 total=2 active_low_stock_count=1`
- 关键字过滤（按条码）：`code=200 total=1` 且命中测试商品
- 分页验证（`page_size=1`）：返回条数 `list_len=1`

## M2.2（第二步）本地冒烟结果（2026-03-02）

- 经营看板接口：`GET /api/v1/reports/dashboard?date=2026-03-02`
- 返回：`code=200`
- 指标结果：
  - `total_sales=7.00`
  - `total_gross_profit=2.80`
  - `total_orders=1`
  - `low_stock_count=1`
  - `top_selling_item=可口可乐 330ml`

## M2.2（第三步）本地冒烟结果（2026-03-03）

- 销售报表接口：`GET /api/v1/reports/sales`
- 返回：`code=200 total=1`
- 报表明细（按商品聚合）：
  - `total_qty=2`（销售 3、退货 1，净销量 2）
  - `total_sales=7.00`
  - `total_cost=4.20`
  - `gross_profit=2.80`
- 汇总：
  - `summary.total_qty=2`
  - `summary.total_sales=7.00`
  - `summary.total_cost=4.20`
  - `summary.total_gross_profit=2.80`
- 分页验证：`page_size=1` 时 `page1_list_len=1`、`page2_list_len=0`
- 时间范围验证：昨日查询 `code=200 total=0`
- 参数校验：
  - `group_by=customer` -> `4000`（仅支持 product）
  - 仅传 `start_date` 不传 `end_date` -> `4000`

## M2.2（第四步）本地冒烟结果（2026-03-03）

- 审计查询接口：`GET /api/v1/audit/logs`
- 前置动作：创建并执行盘点流程（start + confirm）生成 2 条审计日志
- OWNER 查询：
  - `code=200 total=2`
  - 倒序首条 `action=STOCK_CHECK_CONFIRM`
- 过滤验证：
  - `action=STOCK_CHECK_START` -> `code=200 total=1`
  - `target_type=stock_check` -> `code=200` 且类型唯一 `stock_check`
  - `operator_id=<admin_user_id>` -> `code=200 total=2`
  - `request_id=req-audit-start-001` -> `code=200 total=1 action=STOCK_CHECK_START`
  - `start_date=today&end_date=today` -> `code=200 total=2`
- 分页验证：`page_size=1` 时 `page1_len=1`、`page2_len=1`
- 参数校验：
  - 单传 `start_date` -> `4000`
  - `start_date > end_date` -> `4000`
  - `operator_id=abc` -> `4000`
- 权限校验：非 OWNER 访问 -> `4030`

## M2.2（第五步）本地冒烟结果（2026-03-03）

- 销售报表导出接口：`GET /api/v1/reports/sales/export`
- OWNER 导出成功：
  - `http=200`
  - `content_type=text/csv; charset=utf-8`
  - `content_disposition=attachment; filename="sales_report_2026-03-03_2026-03-03.csv"`
- PURCHASER 导出成功：`http=200`
- SALES 无权限：`http=403`，业务码 `4030`
- 参数校验：
  - `format=xlsx` -> `4000`
  - `group_by=customer` -> `4000`
  - 单传 `start_date` -> `4000`
  - `start_date > end_date` -> `4000`
- CSV 内容验证：
  - 商品名转义生效：`"测试,商品""X"`
  - 明细行示例：`2001,"测试,商品""X",2,19.80,0,19.80`
  - 汇总行：`SUMMARY,,2,19.80,0,19.80`

## M2.3（第一步）本地冒烟结果（2026-03-03）

- 销售报表导出接口已扩展支持 `format=xlsx`
- 编译验证：`cargo check` 通过
- 权限验证：
  - OWNER 导出 xlsx：`http=200`
  - PURCHASER 导出 xlsx：`http=200`
  - SALES 导出 xlsx：`http=403`，业务码 `4030`
- 参数校验：
  - `format=pdf` -> `4000`
  - `group_by=customer` -> `4000`
  - 单传 `start_date` -> `4000`
  - `start_date > end_date` -> `4000`
- Header 校验：
  - `content-type=application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`
  - `content-disposition=attachment; filename="sales_report_2026-03-03_2026-03-03.xlsx"`
- xlsx 内容结构校验：
  - `xlsx_zip_ok=yes`
  - `xlsx_row_count_ge_3=yes`（表头 + 明细 + SUMMARY）
  - `xlsx_has_header_product_id=yes`
  - `xlsx_has_header_product_name=yes`
  - `xlsx_has_summary=yes`
- 兼容性回归：`format=csv` 仍返回 `http=200`，且 `SUMMARY` 行正确

## M2.3（第二步）本地冒烟结果（2026-03-03）

- 销售报表导出 xlsx 显示增强已落地（样式/列宽/冻结/筛选/数值格式）：
  - 表头样式：加粗 + 居中 + 背景色 + 边框（header fill 命中 `D9E1F2`）
  - 汇总样式：加粗 + 背景色 + 边框（summary fill 命中 `FCE4D6`）
  - 冻结表头：`freeze_panes=yes`
  - 自动筛选：`autofilter=yes`（明细区）
  - 列宽：导出文件包含列宽配置（`column_width_count=4`，合并区间写法）
  - 数值写入：明细与 SUMMARY 的数量/金额单元格均为数值类型（`*_numeric=yes`）
- 接口验收：
  - 权限：`OWNER=200`、`PURCHASER=200`、`SALES=4030`
  - 参数校验：`format/group_by/日期成对/日期倒挂` 均返回 `4000`
  - Header：
    - xlsx `content-type=application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`
    - xlsx `content-disposition=attachment; filename="sales_report_<start>_<end>.xlsx"`
    - csv `content-type=text/csv; charset=utf-8`
    - csv `content-disposition=attachment; filename="sales_report_<start>_<end>.csv"`
  - 内容口径：CSV 明细与 `SUMMARY`（示例：`SUMMARY,,2,7.00,4.20,2.80`）与 xlsx 一致

## 默认测试账号

- 用户名：`admin`
- 密码：`admin123`

## 当前限制

- 默认使用 PostgreSQL 持久化；`STORAGE_BACKEND=sqlite` 时使用本地 SQLite（单机版），`STORAGE_BACKEND=memory` 时使用内存模式（重启后数据会丢失）。
- 报表已实现经营看板（`/reports/dashboard`）、销售报表（`/reports/sales`，`group_by=product`）与销售报表导出（`/reports/sales/export`，`format=csv/xlsx`）。
- 审计查询已实现（`/audit/logs`，仅 `OWNER`，支持过滤与分页）。
- 报表导出已支持 CSV/XLSX；xlsx 已支持基础样式、列宽、冻结表头、自动筛选、数值格式化与 SUMMARY 强调。
- 已提供 `scripts + systemd` 运维闭环；CI 自动发布仍待后续接入。

## OpenAPI 3.0（M3 第一阶段）

- 已新增接口契约文件：`server/openapi.yaml`
- 覆盖范围：健康检查、认证、商品、采购/销售单、盘点、库存、报表、审计与导出接口。
- 导出接口（`/api/v1/reports/sales/export`）已声明双格式文件流：
  - `text/csv`
  - `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`
- 建议使用 Swagger Editor（Import File）或 Redoc 打开 `openapi.yaml` 进行联调查看。

## M3（第三阶段）第一步：持久化骨架（2026-03-04）

已完成“可插拔存储 + 初始化接线”的第一步落地，当前仍保持业务逻辑以内存态为主，确保无回归：

- 配置扩展（`src/config.rs`）：
  - 新增 `STORAGE_BACKEND`（`memory`/`postgres`）
  - 新增 `DATABASE_URL`、`REDIS_URL`
  - 新增 `PG_MAX_CONNECTIONS`
- 持久化初始化骨架（`src/persistence.rs`）：
  - `initialize_persistence` 按配置初始化 PostgreSQL 连接池（启动时真实建连）与 Redis 客户端（可选）
  - 启动自动执行 `migrations/0001_init.sql`、`0002_core_tables.sql`、`0003_business_tables.sql`
  - 当启用 `postgres` 但未配置 `REDIS_URL` 时给出告警并回退内存幂等缓存
- Repository 抽象占位（`src/repository.rs`）：
  - 定义 `Repository` trait
  - 提供 `MemoryRepository` / `PostgresRepository` 占位实现
- 启动接线（`src/main.rs` / `src/state.rs`）：
  - 启动时初始化 persistence + repository，并注入 `AppState`
  - 保留原有 `AppState::new` 兼容测试与内存态运行
- migration 占位（`migrations/0001_init.sql`）：
  - 新增初始迁移占位，后续逐步补齐业务表 DDL

### 新增环境变量

见 `.env.example`：

- `STORAGE_BACKEND=postgres`
- `DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/jxc`
- `REDIS_URL=redis://127.0.0.1:6379`
- `PG_MAX_CONNECTIONS=10`

### 运行说明

- 默认 `STORAGE_BACKEND=postgres`：未配置 `DATABASE_URL` 会在启动阶段快速失败。
- 若需临时切回内存模式做快速联调，可显式设置 `STORAGE_BACKEND=memory`。
- PostgreSQL 模式启动会自动执行内置迁移脚本（幂等）。

示例：

```bash
cd /Users/admin/Documents/Projects/jxc/server
cp .env.example .env
cargo run
```

## M3 运维闭环（部署 / 巡检 / 备份）

项目已提供标准化运维资产：

- `deploy/jxc-server.service`：systemd 服务单元
- `scripts/install_systemd.sh`：安装并启用 systemd 服务
- `scripts/deploy_remote.sh`：一键同步代码 + 远端构建 + 重启服务 + 健康检查
- `scripts/healthcheck.sh`：健康巡检脚本
- `scripts/backup_postgres.sh`：PostgreSQL 备份与过期清理脚本

### 1) 安装 systemd（首次）

```bash
cd /Users/admin/Documents/Projects/jxc/server
bash scripts/install_systemd.sh
```

### 2) 部署到远端

```bash
cd /Users/admin/Documents/Projects/jxc/server
bash scripts/deploy_remote.sh
```

### 3) 健康巡检

```bash
cd /Users/admin/Documents/Projects/jxc/server
bash scripts/healthcheck.sh http://127.0.0.1:8080/health
```

### 4) 备份 PostgreSQL

```bash
cd /Users/admin/Documents/Projects/jxc/server
DATABASE_URL='postgres://postgres:postgres@127.0.0.1:5432/jxc' \
BACKUP_DIR=/projects/jxcServer/backups \
BACKUP_RETENTION_DAYS=7 \
bash scripts/backup_postgres.sh
```

## M3（第三阶段）第二步：核心表迁移与最小闭环补齐（2026-03-04）

本轮已完成“登录/商品/幂等”最小持久化闭环所需的核心 DDL 与回归验证：

- 新增迁移脚本：`migrations/0002_core_tables.sql`
  - 新增 `users` 表（含角色约束与索引）
  - 新增 `products` 表（含租户维度唯一索引：条码/SKU，仅针对未删除数据）
  - 新增 `idempotency_records` 表（JSONB 请求/响应体与更新时间索引）
  - 写入初始数据：`admin/purchaser/sales` 三个账号、示例商品 `1001`
  - 迁移版本登记：`schema_migrations.version='0002_core_tables'`
- 修复编译阻塞（Axum `Handler` non-Send）：
  - `create_stock_check` / `inbound` / `outbound` 三处改为同步幂等写入 helper，避免 `MutexGuard` 跨 `await`。
- 商品查询链路补齐 Postgres 分支（`src/routes.rs`）：
  - `GET /api/v1/products` 在 `repository.is_postgres()` 下走 `list_products_by_tenant(...)`。
  - `GET /api/v1/products/:id` 在 `repository.is_postgres()` 下走 `find_product_by_id(...)`。
  - `SALES` 角色成本价脱敏与分页/筛选行为在 Memory/Postgres 双分支保持一致。
- 本地质量验证：
  - `cargo fmt` 通过
  - `cargo check` 通过
  - `cargo test` 通过（**10 passed; 0 failed**）
- 远端验收（`ssh ubuntu@1.14.45.242:/projects/jxcServer`）：
  - 已 `rsync` 同步
  - 远端 `cargo test` 通过（**10 passed; 0 failed**）

## M3（第三阶段）第三步（第一部分）：业务核心表 DDL 迁移（2026-03-04）

本轮已完成业务核心表迁移脚本：`migrations/0003_business_tables.sql`，用于为后续 Repository 事务化持久化改造提供数据库基础。

- 新增业务表：
  - 采购：`purchase_orders`、`purchase_order_items`
  - 销售：`sales_orders`、`sales_order_items`
  - 盘点：`stock_checks`、`stock_check_items`
  - 流水：`stock_logs`（不可变流水）
  - 审计：`audit_logs`（`before_data/after_data + request_id`）
- 新增关键约束与索引：
  - 状态机 `CHECK` 约束（采购/销售/盘点状态值与模型一致）
  - 数量/金额范围校验（如 `qty > 0`、`unit_cost >= 0`）
  - 主单与明细 `FK`（明细 `ON DELETE CASCADE`）
  - 租户维度业务单号唯一索引（`tenant_id + biz_no`）
  - 高频查询索引（租户、状态、业务单号、时间倒序）
- 迁移版本登记：
  - `schema_migrations.version='0003_business_tables'`

质量与验收结果：

- 本地：`cargo fmt && cargo check && cargo test` 通过（**10 passed; 0 failed**）
- 远端（`ssh ubuntu@1.14.45.242:/projects/jxcServer`）：
  - 已 `rsync` 同步代码
  - `cargo test` 通过（**10 passed; 0 failed**）

> 说明：当前该步骤完成的是 DDL 落地；业务路由到 Postgres 的完整事务读写改造将在后续步骤继续推进。

## M3（第三阶段）第三步（第二部分）：采购路由 Postgres 接线（2026-03-04）

本轮已完成采购单最小闭环的路由层 Postgres 接线（Repository + Routes），并保留 Memory fallback：

- 已接线路由：
  - `POST /api/v1/purchase-orders`
  - `GET /api/v1/purchase-orders/:id`
  - `POST /api/v1/purchase-orders/:id/confirm`
  - `POST /api/v1/purchase-orders/:id/void`
- 已接通仓储方法：
  - `next_purchase_order_id`
  - `is_purchase_order_biz_no_taken`
  - `create_purchase_order`
  - `find_purchase_order_by_id`
  - `confirm_purchase_order`
  - `void_purchase_order`
- 幂等处理：
  - Postgres 分支使用异步 `save_idempotency_record(...)`
  - 内存分支保持同步 `save_idempotency_record_sync(...)`

本地验证结果：

- `cargo fmt` 通过
- `cargo check` 通过
- `cargo test` 通过（**10 passed; 0 failed**）

> 说明：采购单 PG 路由已打通；后续将继续推进销售/盘点/库存流水/审计等路由的 Postgres 主链路迁移。

## M3（第二阶段）自动化测试（阶段通过）

当前已大幅扩展后端自动化测试（`server/src/routes.rs` 内 `#[cfg(test)]`）：

- **基础校验**：登录成功、未鉴权拦截、销售角色权限拦截。
- **状态机流转**：采购单、销售单、盘点单全链路库存及状态断言。
- **业务规约**：4091 版本冲突返回快照、4092 幂等冲突、幂等重放头校验。
- **数据一致性**：CSV 导出明细与汇总行一致性断言。

执行方式：

```bash
cd /Users/admin/Documents/Projects/jxc/server
cargo test
```

本地实测结果（2026-03-04）：

- `cargo test`：通过（**10 passed; 0 failed**）
- 覆盖用例：
  - `login_success_returns_200_and_token`
  - `protected_route_without_token_returns_401`
  - `create_product_idempotent_replay_header_should_be_true_on_second_call`
  - `sales_role_cannot_export_sales_report`
  - `purchase_order_state_machine_should_confirm_and_void_with_stock_roundtrip`
  - `sales_order_state_machine_should_reach_returned_full_and_restore_stock`
  - `stock_check_state_machine_should_move_to_confirmed_and_apply_delta`
  - `expected_version_conflict_should_return_4091_with_latest_snapshot`
  - `idempotency_payload_conflict_should_return_4092`
  - `sales_report_export_csv_should_match_sales_report_summary_and_detail`
- 测试修复记录：
  - 增加 dev 依赖：`tower = { version = "0.5", features = ["util"] }`
  - `ServiceExt` 导入修正为：`tower::util::ServiceExt`
  - 修复测试中 `Router.state()` 不可用问题：改为在 `build_router(state)` 前直接修改 `state.users`

远端实测结果（2026-03-04，`/projects/jxcServer`）：

- 执行：`export PATH=/home/ubuntu/.cargo/bin:$PATH && cargo test`
- 结果：**10 passed; 0 failed**（本地同步并通过验收）

## M3（第二阶段）质量收尾（2026-03-04）

- 代码清理：
  - 已修复 `server/src/routes.rs` 中 5 处 `clippy::explicit_counter_loop` 警告。
  - 显式循环计数器改为 `(start_log_id..).zip(iter)` 迭代器写法。
- Clippy 收口：
  - 业务保留函数 `append_audit_log` 增加 `#[allow(clippy::too_many_arguments)]`。
- 代码格式：
  - 已执行 `cargo fmt` 统一风格。
- 本地验证：
  - `cargo clippy --all-targets --all-features`：通过（0 warning）。
  - `cargo test`：通过（**10 passed; 0 failed**）。
- 远端验收（`ssh ubuntu@1.14.45.242:/projects/jxcServer`）：
  - 已 `rsync` 同步最新代码。
  - 远端执行 `export PATH=/home/ubuntu/.cargo/bin:$PATH && cargo test`：**10 passed; 0 failed**。

## M3（第三阶段）第三步（第三部分）：销售路由 Postgres 接线（2026-03-04）

本轮已完成销售单路由层的 Postgres 主链路补齐，保持与采购路由一致的“双分支”策略（Postgres 主链路 + Memory fallback）：

- 已接线路由（销售 5/5）：
  - `POST /api/v1/sales-orders`
  - `GET /api/v1/sales-orders/:id`
  - `POST /api/v1/sales-orders/:id/confirm`
  - `POST /api/v1/sales-orders/:id/void`
  - `POST /api/v1/sales-orders/:id/return`
- 本次补齐重点（新增 PG 分支）：
  - `confirm_sales_order`
  - `void_sales_order`
  - `return_sales_order`
- 角色权限与租户隔离：
  - `OWNER/SALES` 可操作销售单；
  - `SALES` 在 PG 分支下仅可操作本人创建单据（`created_by == auth.user_id`）；
  - 全链路按 `tenant_id` 隔离。
- 幂等与并发语义保持一致：
  - `X-Idempotency-Key` 命中重放返回首次响应；
  - `expected_version` 冲突返回 `4091`；
  - 业务状态冲突返回 `4090`。

本地验证结果：

- `cargo fmt`：通过
- `cargo check`：通过
- `cargo test`：通过（**10 passed; 0 failed**）

## M3（第三阶段）第三步（第四部分）：盘点路由 Postgres 接线（2026-03-04）

本轮已完成盘点模块路由层与仓储层的 Postgres 主链路接线（Repository + Routes），并保持与既有模块一致的双分支策略（Postgres 主链路 + Memory fallback）：

- 已接线路由（盘点 4/4）：
  - `POST /api/v1/inventory/stock-checks`
  - `GET /api/v1/inventory/stock-checks/:id`
  - `POST /api/v1/inventory/stock-checks/:id/start`
  - `POST /api/v1/inventory/stock-checks/:id/confirm`
- 已接通仓储方法：
  - `next_stock_check_id`
  - `is_stock_check_biz_no_taken`
  - `create_stock_check`
  - `find_stock_check_by_id`
  - `start_stock_check`
  - `confirm_stock_check`
- 关键行为保持一致：
  - 幂等语义保持不变（`X-Idempotency-Key` / `4092` / `X-Idempotent-Replay`）；
  - 乐观并发保持不变（`expected_version` 冲突返回 `4091` + 最新快照）；
  - 业务状态冲突保持不变（`4090`）；
  - 盘点确认继续执行“基准库存漂移校验 + 差异入账（`ADJ_CHECK`）+ 审计落库（`STOCK_CHECK_START`/`STOCK_CHECK_CONFIRM`）”。

本地验证结果：

- `cargo fmt`：通过
- `cargo check`：通过
- `cargo test`：通过（**10 passed; 0 failed**）

## M3（第三阶段）第三步（第五部分）：库存入/出库路由 Postgres 接线（2026-03-04）

本轮已完成库存入库/出库路由层与仓储层的 Postgres 主链路接线（Repository + Routes），并保持与现有模块一致的双分支策略（Postgres 主链路 + Memory fallback）：

- 已接线路由（库存 2/2）：
  - `POST /api/v1/inventory/inbound`
  - `POST /api/v1/inventory/outbound`
- 已接通仓储方法：
  - `inbound`
  - `outbound`
- 关键行为保持一致：
  - 幂等语义保持不变（`X-Idempotency-Key` / `4092` / `X-Idempotent-Replay`）；
  - 乐观并发保持不变（`expected_version` 冲突返回 `4091` + 最新快照）；
  - 库存不足保持返回业务码 `4001`；
  - 出库支持“明细级 expected_version 优先，单据级 expected_version 兜底”；
  - 入库 PG 分支支持 `product_id`/`barcode` 二选一解析（`barcode` 通过仓储查询）。

本地验证结果：

- `cargo fmt`：通过
- `cargo check`：通过
- `cargo test`：通过（**10 passed; 0 failed**）

## M3（第三阶段）第三步（第六部分）：报表/审计/预警路由 Postgres 接线（2026-03-04）

本轮已完成报表/审计/预警相关接口的 Postgres 主链路接线（Repository + Routes），并保持与现有模块一致的双分支策略（Postgres 主链路 + Memory fallback）：

- 已接线路由（5/5）：
  - `GET /api/v1/reports/dashboard`
  - `GET /api/v1/reports/sales`
  - `GET /api/v1/reports/sales/export`
  - `GET /api/v1/audit/logs`
  - `GET /api/v1/inventory/alerts/low-stock`
- 已接通仓储读取方法：
  - `list_products_by_tenant_all`
  - `list_sales_orders_by_tenant`
  - `list_stock_logs_by_tenant`
  - `list_audit_logs_by_tenant`
- 关键行为保持一致：
  - 报表口径不变：销售额/成本/毛利继续基于 `stock_logs`（`OUT_SALE` / `RETURN_SALE`）与销售单明细售价聚合；
  - Dashboard 指标口径不变：`total_sales`、`total_gross_profit`、`total_orders`、`low_stock_count`、`top_selling_item`；
  - 导出行为不变：`csv/xlsx` 双格式、汇总行与明细排序规则保持一致；
  - 审计日志筛选与分页不变：`action/target_type/operator_id/request_id/date range` 过滤 + `id desc`；
  - 低库存预警行为不变：`only_active`、`keyword`、短缺量排序与分页逻辑保持一致。

本地验证结果：

- `cargo fmt`：通过
- `cargo check`：通过
- `cargo test`：通过（**10 passed; 0 failed**）

## M3（第三阶段）第三步（第七部分）：认证 refresh 路由 Postgres 接线（2026-03-04）

本轮继续推进认证链路，完成 `POST /api/v1/auth/refresh` 的 Postgres 主链路接线，并保持 Memory fallback：

- 路由改造：`refresh_token`
  - Postgres 模式下，基于 `claims.tenant_id + claims.user_id` 从仓储读取用户，而非直接依赖内存 `state.users`；
  - Memory 模式保持原有内存查找行为。
- 仓储能力补齐：
  - `RepositoryProvider::find_user_by_id(pool, tenant_id, user_id)`
  - `PostgresRepository::find_user_by_id(...)`（按 `id + tenant_id` 查询 `users`）
- 行为保持一致：
  - 继续校验 `refresh_token` 非空、JWT 有效、`token_type == refresh`；
  - 用户不存在仍返回未授权错误；
  - access token 下发结构不变（`access_token/expires_in/tenant_id/user_info`）。

本地验证结果：

- `cargo fmt`：通过
- `cargo check`：通过
- `cargo test`：通过（**10 passed; 0 failed**）


## M3（第三阶段）第三步（第八部分）：认证鉴权中间件 Postgres 接线（2026-03-04）

本轮继续推进认证链路，完成 `auth_middleware` 的 Postgres 主链路接线，并保持 Memory fallback：

- 中间件改造：`src/middleware.rs::auth_middleware`
  - 在完成 `Bearer` 提取、JWT 校验与 `token_type == access` 校验后：
    - Postgres 模式下，基于 `claims.tenant_id + claims.user_id` 调用仓储 `find_user_by_id(...)` 二次校验用户有效性；
    - Memory 模式下，保持原有 `state.users` 内存查找逻辑。
- `AuthContext` 注入策略升级：
  - 不再直接信任 token 内 `role` 字段作为最终上下文；
  - 改为从当前用户实体读取并注入 `tenant_id/user_id/role`，确保角色变更后鉴权语义与存储态一致。
- 行为保持一致：
  - 缺少/非法 `Authorization` 仍返回未授权；
  - 非 `access` token 仍返回未授权；
  - 用户不存在仍返回未授权错误（附带 request_id）。

本地验证结果：

- `cargo fmt`：通过
- `cargo check`：通过
- `cargo test`：通过（**10 passed; 0 failed**）

## M3（第三阶段）第三步（第九部分）：鉴权中间件安全回归测试补齐（2026-03-04）

本轮在第八部分中间件改造基础上，补齐鉴权安全回归用例，确保“中间件二次校验用户有效性 + 角色以存储态为准”语义稳定：

- 新增测试（`src/routes.rs`）：
  - `protected_route_with_nonexistent_user_token_returns_401`
    - 使用合法签名但 `user_id` 不存在的 access token 访问受保护接口，断言 `4010`；
    - 验证中间件不会仅凭 token claim 放行。
  - `auth_middleware_should_use_current_user_role_instead_of_token_role`
    - 构造真实 `SALES` 用户，但伪造 token 中 `role=OWNER`；
    - 访问仅 `OWNER/PURCHASER` 可用的销售报表导出接口，断言 `4030`；
    - 验证鉴权最终角色取自当前用户实体而非 token 载荷。
- 结果：
  - 测试总数由 10 增至 12；
  - 认证中间件安全语义具备自动化回归保护。

本地验证结果：

- `cargo fmt`：通过
- `cargo check`：通过
- `cargo test`：通过（**12 passed; 0 failed**）

## M3（第三阶段）第三步（第十部分）：refresh 路由安全回归测试补齐（2026-03-04）

本轮在第七/第八/第九部分基础上，继续补齐 `POST /api/v1/auth/refresh` 的安全回归测试，覆盖“用户有效性校验 + 角色来源一致性 + 正常成功路径”：

- 新增测试（`src/routes.rs`）：
  - `refresh_token_for_nonexistent_user_returns_401`
    - 使用合法签名的 refresh token，但对应用户在当前存储态不存在；
    - 断言返回 `4010`，验证 refresh 链路不会仅凭 token claim 放行。
  - `refresh_token_should_return_current_user_role`
    - 先登录拿到 refresh token，再将当前用户角色改为 `OWNER` 后发起刷新；
    - 断言响应中的 `user_info.role=OWNER`，验证角色以存储态为准。
  - `refresh_token_success_returns_200_and_access_token`
    - 新增 `login_tokens(...)` 测试辅助函数复用登录动作；
    - 断言 refresh 正常返回 `200`，且响应内存在新的 `access_token` 与 `user_info`。
- 结果：
  - 测试总数由 12 增至 15；
  - refresh 路由的正/反向安全语义均具备自动化回归保护。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**15 passed; 0 failed**）

## M3（第三阶段）第三步（第十一部分）：认证 token_type 与 logout 回归测试补齐（2026-03-04）

本轮继续围绕认证链路补齐边界回归，聚焦“access/refresh token_type 不可混用”与 `logout` 受保护路由语义，确保中间件与 refresh 路由约束长期稳定：

- 新增测试（`src/routes.rs`）：
  - `refresh_endpoint_with_access_token_returns_401`
    - 使用 `access_token` 调用 `POST /api/v1/auth/refresh`；
    - 断言返回 `4010`，验证 refresh 仅接受 `refresh` token。
  - `protected_route_with_refresh_token_returns_401`
    - 使用 `refresh_token` 访问受保护业务路由；
    - 断言返回 `4010`，验证受保护路由仅接受 `access` token。
  - `logout_with_access_token_returns_200_and_user_id`
    - 使用合法 `access_token` 调用 `POST /api/v1/auth/logout`；
    - 断言 `200` 且返回 `logged_out=true`、`user_id`。
  - `logout_with_refresh_token_returns_401`
    - 使用 `refresh_token` 调用 `POST /api/v1/auth/logout`；
    - 断言返回 `4010`，验证中间件 token_type 拦截在 logout 场景同样生效。
- 结果：
  - 测试总数由 15 增至 19；
  - 认证 token_type 约束与 logout 链路具备自动化回归保护。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**19 passed; 0 failed**）

## M3（第三阶段）第三步（第十二部分）：request_id 透传与幂等回放链路回归测试补齐（2026-03-04）

本轮继续补齐链路级可观测性回归，聚焦 `x-request-id` 透传与幂等回放场景下 request_id 一致性，确保排障与审计链路稳定：

- 新增测试（`src/routes.rs`）：
  - `login_should_echo_request_id_header`
    - 登录请求显式携带 `x-request-id`；
    - 断言响应 header 与 body 的 `request_id` 均回显同一值。
  - `protected_route_401_should_echo_request_id_header`
    - 未鉴权访问受保护路由并携带 `x-request-id`；
    - 断言 `4010` 错误响应仍完整回显 `request_id`（header + body）。
  - `idempotent_replay_should_keep_first_request_id`
    - 同一 `X-Idempotency-Key` 首次请求与回放请求使用不同 `x-request-id`；
    - 断言回放响应 `x-idempotent-replay=true`，且 `request_id` 固定为首次请求值，保证幂等重放链路可追踪一致。
- 结果：
  - 测试总数由 19 增至 22；
  - request_id 透传与幂等回放的一致性具备自动化回归保护。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**22 passed; 0 failed**）

## M3（第三阶段）第三步（第十三部分）：认证负向边界回归测试补齐（2026-03-04）

本轮继续补齐认证链路负向边界回归，覆盖登录失败、refresh 参数校验与鉴权头格式异常，确保错误语义稳定：

- 新增测试（`src/routes.rs`）：
  - `login_with_wrong_password_returns_4010`
    - 使用错误密码登录；
    - 断言返回未授权业务码 `4010`。
  - `refresh_with_empty_token_returns_4000`
    - `refresh_token` 传入空白字符串；
    - 断言返回请求参数错误业务码 `4000`。
  - `protected_route_with_malformed_authorization_returns_4010`
    - 使用非法 `Authorization` 头（非 `Bearer <token>`）；
    - 断言中间件返回未授权业务码 `4010`。
- 结果：
  - 测试总数由 22 增至 25；
  - 认证负向边界语义具备自动化回归保护。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**25 passed; 0 failed**）

## M3（第三阶段）第三步（第十四部分）：JWT 非法/过期与登录负向回归测试补齐（2026-03-04）

本轮继续补齐认证安全边界，覆盖“用户不存在登录失败”“JWT 非法签名/非法格式/已过期”负向语义，并修正 JWT 校验默认过期宽限：

- 认证校验修正（`src/middleware.rs`）：
  - `verify_token` 改为显式 `Validation` 配置并设置 `validation.leeway = 0`；
  - 消除默认过期宽限对“已过期 token”判定的干扰，确保过期即拒绝。
- 新增测试（`src/routes.rs`）：
  - `login_with_unknown_user_returns_4010`
    - 不存在用户登录，断言 `4010`。
  - `refresh_with_invalid_jwt_returns_4010`
    - 非法 JWT 调用 refresh，断言 `4010`。
  - `protected_route_with_wrong_jwt_signature_returns_4010`
    - 错误密钥签发 token 访问受保护路由，断言 `4010`。
  - `protected_route_with_expired_access_token_returns_4010`
    - 过期 access token 访问受保护路由，断言 `4010`。
- 结果：
  - 测试总数由 25 增至 29；
  - JWT 非法/过期与登录负向边界具备自动化回归保护。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**29 passed; 0 failed**）

## M3（第三阶段）第三步（第十五部分）：refresh/logout 过期与签名异常回归测试补齐（2026-03-04）

本轮继续收紧认证边界，聚焦 refresh/logout 场景下的“过期 token + 错误签名”负向语义，确保与受保护资源路由一致：

- 新增测试（`src/routes.rs`）：
  - `refresh_with_expired_refresh_token_returns_4010`
    - 使用已过期 refresh token 调用 `POST /api/v1/auth/refresh`；
    - 断言返回 `4010`。
  - `refresh_with_wrong_jwt_signature_returns_4010`
    - 使用错误密钥签发的 refresh token 调用 refresh；
    - 断言返回 `4010`。
  - `logout_with_expired_access_token_returns_4010`
    - 使用已过期 access token 调用 `POST /api/v1/auth/logout`；
    - 断言返回 `4010`。
- 结果：
  - 测试总数由 29 增至 32；
  - refresh/logout 在过期与签名异常场景下具备自动化回归保护。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**32 passed; 0 failed**）

## M3（第三阶段）第三步（第十六部分）：幂等异常与 request_id 回显回归测试补齐（2026-03-04）

本轮继续加强链路可观测性与幂等异常语义，聚焦“缺少幂等键”与“4092 冲突”场景下 request_id 回显一致性：

- 新增测试（`src/routes.rs`）：
  - `create_product_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `POST /api/v1/products` 时不传 `X-Idempotency-Key`，并显式携带 `x-request-id`；
    - 断言返回 `4000`，且响应 header/body 的 `request_id` 均回显当前请求值。
  - `idempotency_payload_conflict_should_echo_current_request_id`
    - 同一幂等键首次请求成功，第二次改 payload 触发 `4092`；
    - 断言错误响应不带 `x-idempotent-replay`，并且 `request_id` 回显第二次（当前）请求值。
- 结果：
  - 测试总数由 32 增至 34；
  - 幂等异常路径的 request_id 透传语义具备自动化回归保护。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**34 passed; 0 failed**）

## M3（第三阶段）第三步（第十七部分）：自动 request_id 生成与回放一致性回归测试补齐（2026-03-04）

本轮继续补齐 request_id 可观测性边界，覆盖“未传 x-request-id 自动生成”与“幂等回放沿用首次自动 request_id”语义：

- 新增测试（`src/routes.rs`）：
  - `login_without_request_id_should_generate_header_and_body_request_id`
    - 登录请求不传 `x-request-id`；
    - 断言响应 header/body 都存在自动生成的 `request_id`（`req_` 前缀）且两者一致。
  - `protected_route_401_without_request_id_should_generate_header_and_body_request_id`
    - 未鉴权访问受保护路由且不传 `x-request-id`；
    - 断言 `4010` 错误响应仍生成并回显一致的 `request_id`（header/body）。
  - `idempotent_replay_without_request_id_should_keep_first_generated_request_id`
    - 同一幂等键两次请求均不传 `x-request-id`；
    - 断言第二次为回放（`x-idempotent-replay=true`），并沿用首次自动生成的 `request_id`。
- 结果：
  - 测试总数由 34 增至 37；
  - 自动 request_id 生成与幂等回放一致性具备自动化回归保护。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**37 passed; 0 failed**）

## M3（第三阶段）第三步（第十八部分）：写接口幂等键必填与 request_id 回显回归测试补齐（2026-03-04）

本轮继续补齐写接口幂等校验边界，聚焦“缺少 `X-Idempotency-Key`”场景下统一错误语义与 request_id 透传，确保各写接口行为一致、可观测性一致：

- 新增测试（`src/routes.rs`）：
  - `update_product_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `PUT /api/v1/products/:id` 不传 `X-Idempotency-Key`，并显式携带 `x-request-id`；
    - 断言返回 `4000`，且响应 header/body 的 `request_id` 均回显当前请求值。
  - `purchase_order_create_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `POST /api/v1/purchase-orders` 不传 `X-Idempotency-Key`；
    - 断言返回 `4000`，且 `request_id` 在 header/body 一致回显。
  - `inbound_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `POST /api/v1/inventory/inbound` 不传 `X-Idempotency-Key`；
    - 断言返回 `4000`，且 `request_id` 在 header/body 一致回显。
  - `outbound_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `POST /api/v1/inventory/outbound` 不传 `X-Idempotency-Key`；
    - 断言返回 `4000`，且 `request_id` 在 header/body 一致回显。
- 结果：
  - 测试总数由 37 增至 41；
  - 写接口缺少幂等键场景下的错误码与 request_id 回显语义具备自动化回归保护。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**41 passed; 0 failed**）

## M3（第三阶段）第三步（第十九部分）：剩余写接口缺少幂等键回归测试全覆盖（2026-03-04）

本轮继续补齐写接口在缺少 `X-Idempotency-Key` 时的统一错误语义，覆盖此前尚未纳入回归的写路径，并统一复用断言逻辑，确保 request_id 可观测性一致：

- 新增测试辅助函数（`src/routes.rs`）：
  - `assert_missing_idempotency_key_response`
    - 统一断言：HTTP `400`、业务码 `4000`、`x-idempotent-replay=false`、`x-request-id` 与 body `request_id` 一致回显。
- 新增测试（`src/routes.rs`）：
  - `delete_product_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `DELETE /api/v1/products/:id` 不传 `X-Idempotency-Key`；
    - 断言 `4000` 且 request_id 在 header/body 一致回显。
  - `purchase_order_confirm_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `POST /api/v1/purchase-orders/:id/confirm` 不传 `X-Idempotency-Key`；
    - 断言 `4000` 且 request_id 一致回显。
  - `purchase_order_void_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `POST /api/v1/purchase-orders/:id/void` 不传 `X-Idempotency-Key`；
    - 断言 `4000` 且 request_id 一致回显。
  - `create_sales_order_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `POST /api/v1/sales-orders` 不传 `X-Idempotency-Key`；
    - 断言 `4000` 且 request_id 一致回显。
  - `sales_order_confirm_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `POST /api/v1/sales-orders/:id/confirm` 不传 `X-Idempotency-Key`；
    - 断言 `4000` 且 request_id 一致回显。
  - `sales_order_void_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `POST /api/v1/sales-orders/:id/void` 不传 `X-Idempotency-Key`；
    - 断言 `4000` 且 request_id 一致回显。
  - `sales_order_return_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `POST /api/v1/sales-orders/:id/return` 不传 `X-Idempotency-Key`；
    - 断言 `4000` 且 request_id 一致回显。
  - `create_stock_check_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `POST /api/v1/inventory/stock-checks` 不传 `X-Idempotency-Key`；
    - 断言 `4000` 且 request_id 一致回显。
  - `stock_check_start_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `POST /api/v1/inventory/stock-checks/:id/start` 不传 `X-Idempotency-Key`；
    - 断言 `4000` 且 request_id 一致回显。
  - `stock_check_confirm_without_idempotency_key_returns_4000_and_echo_request_id`
    - 调用 `POST /api/v1/inventory/stock-checks/:id/confirm` 不传 `X-Idempotency-Key`；
    - 断言 `4000` 且 request_id 一致回显。
- 结果：
  - 测试总数由 41 增至 51；
  - 剩余写接口“缺少幂等键”场景实现回归测试全覆盖，错误码与 request_id 回显语义一致。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**51 passed; 0 failed**）

## M3（第三阶段）第三步（第二十部分）：缺幂等键且未传 request_id 自动生成链路回归测试补齐（2026-03-04）

本轮继续补齐写接口在缺少 `X-Idempotency-Key` 时的可观测性边界，聚焦“未传 `x-request-id` 自动生成”语义，确保错误路径下 header/body `request_id` 一致且可追踪：

- 新增测试辅助函数（`src/routes.rs`）：
  - `assert_missing_idempotency_key_response_with_generated_request_id`
    - 统一断言：HTTP `400`、业务码 `4000`、`x-idempotent-replay=false`；
    - 断言响应 `x-request-id` 自动生成且以 `req_` 开头；
    - 断言 body `request_id` 与 header 完全一致。
- 新增测试（`src/routes.rs`）：
  - `delete_product_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
  - `purchase_order_confirm_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
  - `purchase_order_void_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
  - `create_sales_order_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
  - `sales_order_confirm_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
  - `sales_order_void_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
  - `sales_order_return_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
  - `create_stock_check_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
  - `stock_check_start_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
  - `stock_check_confirm_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
- 结果：
  - 测试总数由 51 增至 61；
  - 缺幂等键且未传 request_id 场景下，自动 request_id 生成与 header/body 一致性具备自动化回归保障。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**61 passed; 0 failed**）

## M3（第三阶段）第三步（第二十一部分）：缺幂等键且未传 request_id 自动生成链路回归补齐（剩余写接口）（2026-03-05）

本轮继续补齐写接口在缺少 `X-Idempotency-Key` 且未传 `x-request-id` 时的自动 request_id 生成语义，覆盖此前剩余的 5 条写路径，确保错误路径下 header/body `request_id` 一致且可追踪：

- 新增测试（`src/routes.rs`）：
  - `update_product_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
    - 调用 `PUT /api/v1/products/:id` 不传 `X-Idempotency-Key` 且不传 `x-request-id`；
    - 断言 `4000`，并校验自动生成 `request_id`（`req_` 前缀）在 header/body 一致。
  - `purchase_order_create_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
    - 调用 `POST /api/v1/purchase-orders` 不传 `X-Idempotency-Key` 且不传 `x-request-id`；
    - 断言 `4000`，并校验自动生成 `request_id` 在 header/body 一致。
  - `inbound_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
    - 调用 `POST /api/v1/inventory/inbound` 不传 `X-Idempotency-Key` 且不传 `x-request-id`；
    - 断言 `4000`，并校验自动生成 `request_id` 在 header/body 一致。
  - `outbound_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
    - 调用 `POST /api/v1/inventory/outbound` 不传 `X-Idempotency-Key` 且不传 `x-request-id`；
    - 断言 `4000`，并校验自动生成 `request_id` 在 header/body 一致。
  - `create_product_without_idempotency_key_and_without_request_id_should_generate_header_and_body_request_id`
    - 调用 `POST /api/v1/products` 不传 `X-Idempotency-Key` 且不传 `x-request-id`；
    - 断言 `4000`，并校验自动生成 `request_id` 在 header/body 一致。
- 结果：
  - 测试总数由 61 增至 66；
  - 写接口“缺幂等键 + 未传 request_id”场景已在当前实现范围内补齐回归覆盖。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**66 passed; 0 failed**）

## M3（第三阶段）第三步（第二十二部分）：幂等回放语义回归补齐（剩余写接口）（2026-03-05）

本轮在既有幂等回放测试基础上，继续补齐剩余写接口的“同 key 同 payload 二次请求回放”语义，重点验证 `x-idempotent-replay` 与 `request_id` 固化行为：

- 新增测试（`src/routes.rs`）：
  - `update_product_idempotent_replay_header_should_be_true_on_second_call`
    - `PUT /api/v1/products/:id` 二次同 key 同 payload 请求；
    - 断言首次 `x-idempotent-replay=false`、二次 `x-idempotent-replay=true`，且二次 `request_id` 固定为首次请求值。
  - `purchase_order_create_idempotent_replay_header_should_be_true_on_second_call`
    - `POST /api/v1/purchase-orders` 二次同 key 同 payload 请求；
    - 断言回放请求 `x-idempotent-replay=true`，并保持首次 `request_id`。
  - `inbound_idempotent_replay_header_should_be_true_on_second_call`
    - `POST /api/v1/inventory/inbound` 二次同 key 同 payload 请求；
    - 断言回放请求 `x-idempotent-replay=true`，并保持首次 `request_id`。
  - `outbound_idempotent_replay_header_should_be_true_on_second_call`
    - `POST /api/v1/inventory/outbound` 二次同 key 同 payload 请求；
    - 断言回放请求 `x-idempotent-replay=true`，并保持首次 `request_id`。
- 断言语义：
  - 第一次请求：`x-idempotent-replay=false`
  - 第二次同 key 同 payload：`x-idempotent-replay=true`
  - 第二次响应 `x-request-id` 与 body `request_id` 均等于第一次请求的 `request_id`
- 结果：
  - 测试总数由 66 增至 70；
  - 幂等回放头与 request_id 固化链路在剩余写接口范围内完成回归覆盖。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**70 passed; 0 failed**）

## M3（第三阶段）第三步（第二十三部分）：幂等回放语义回归补齐（剩余写接口第二批）（2026-03-05）

本轮在“第二十二部分”基础上，继续补齐其余写接口的幂等回放语义，覆盖删除商品、采购确认/作废、销售创建/确认/作废/退货、盘点创建/开始/确认，确保所有写接口在“同 key + 同 payload”二次请求时行为一致：

- 新增测试（`src/routes.rs`）：
  - `delete_product_idempotent_replay_header_should_be_true_on_second_call`
  - `purchase_order_confirm_idempotent_replay_header_should_be_true_on_second_call`
  - `purchase_order_void_idempotent_replay_header_should_be_true_on_second_call`
  - `create_sales_order_idempotent_replay_header_should_be_true_on_second_call`
  - `sales_order_confirm_idempotent_replay_header_should_be_true_on_second_call`
  - `sales_order_void_idempotent_replay_header_should_be_true_on_second_call`
  - `sales_order_return_idempotent_replay_header_should_be_true_on_second_call`
  - `create_stock_check_idempotent_replay_header_should_be_true_on_second_call`
  - `stock_check_start_idempotent_replay_header_should_be_true_on_second_call`
  - `stock_check_confirm_idempotent_replay_header_should_be_true_on_second_call`
- 断言语义：
  - 第一次请求：`x-idempotent-replay=false`
  - 第二次同 key 同 payload：`x-idempotent-replay=true`
  - 第二次响应 `x-request-id` 与 body `request_id` 固定为第一次请求的 `request_id`
- 结果：
  - 幂等回放语义测试从 5 个扩展为 15 个（覆盖当前全部写接口）
  - 测试总数由 70 增至 80

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**80 passed; 0 failed**）

远端验收结果（`ssh ubuntu@1.14.45.242:/projects/jxcServer`）：

- `rsync` 同步：通过
- `cargo test`：通过（**80 passed; 0 failed**）

## M3（第三阶段）第三步（第二十四部分）：幂等请求体冲突（4092）回归补齐（第二批）（2026-03-05）

本轮在“第二十三部分”完成幂等回放全覆盖基础上，继续补齐“同 key 异 payload”冲突路径，覆盖更多写接口并统一断言语义，确保 `4092`、`x-idempotent-replay` 与 `request_id` 回显行为稳定：

- 新增测试辅助函数（`src/routes.rs`）：
  - `assert_idempotency_payload_conflict_response`
  - 统一断言：
    - HTTP `409`（`StatusCode::CONFLICT`）
    - 业务码 `4092`
    - `x-idempotent-replay=false`
    - `x-request-id` 与 body `request_id` 回显当前（第二次）请求值
- 重构既有测试：
  - `idempotency_payload_conflict_should_echo_current_request_id`
  - 改为复用统一 helper，减少重复断言。
- 新增 5 个“同 key 异 payload”冲突回归测试：
  - `update_product_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id`
  - `purchase_order_create_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id`
  - `inbound_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id`
  - `outbound_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id`
  - `create_sales_order_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id`
- 统一测试模式：
  - 第一次请求：同 key + payload A，预期成功（`200`）
  - 第二次请求：同 key + payload B（与 A 不同），预期冲突（`4092`）

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**85 passed; 0 failed**）

远端验收结果（`ssh ubuntu@1.14.45.242:/projects/jxcServer`）：

- `rsync` 同步：通过
- `cargo test`：通过（**85 passed; 0 failed**）

## M3（第三阶段）第三步（第二十五部分）：幂等请求体冲突（4092）回归补齐（第三批）（2026-03-05）

本轮继续补齐“同 key 异 payload”冲突路径，围绕此前未覆盖的 9 个写接口动作场景，统一验证 `4092`、`x-idempotent-replay=false` 与当前请求 `request_id` 回显语义。

- 断言策略：
  - 复用既有 helper：`assert_idempotency_payload_conflict_response`
  - 统一校验：
    - HTTP `409`（`StatusCode::CONFLICT`）
    - 业务码 `4092`
    - `x-idempotent-replay=false`
    - header/body `request_id` 回显第二次（当前）请求值
- 新增 9 个“同 key 异 payload”冲突回归测试（`src/routes.rs`）：
  - `delete_product_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id`
  - `purchase_order_confirm_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id`
  - `purchase_order_void_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id`
  - `sales_order_confirm_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id`
  - `sales_order_void_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id`
  - `sales_order_return_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id`
  - `create_stock_check_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id`
  - `stock_check_start_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id`
  - `stock_check_confirm_idempotency_payload_conflict_should_return_4092_and_echo_current_request_id`
- 差异化 payload 设计（用于稳定触发 4092）：
  - `DELETE /products/:id`：`expected_version=1` vs `2`
  - 采购确认/作废、销售确认/作废、盘点开始：`expected_version=1` vs `2`
  - 销售退货：`items.qty=1` vs `2`
  - 创建盘点单：`biz_no` 变化
  - 盘点确认：`actual_stock=103` vs `104`

本地验证结果：

- `cargo test --manifest-path server/Cargo.toml`：通过（**94 passed; 0 failed**）

远端验收结果（`ssh ubuntu@1.14.45.242:/projects/jxcServer`）：

- 已修复远端命令中的 `PATH` 展开问题（改为单引号包裹远端命令）
- `cargo test`：通过（**94 passed; 0 failed**）

## M3（第三阶段）第三步（第二十六部分）：Postgres 商品乐观锁（4091）仓储回归补齐（2026-03-05）

本轮聚焦“Postgres 后端商品更新乐观锁语义”回归补齐，确保仓储层在竞态窗口下对 `4091`（版本冲突）与 `4040`（资源不存在）做出正确区分，并与 API 文档约定保持一致。

- 新增仓储层 Postgres 回归测试（`src/repository.rs`）：
  - `postgres_update_product_stale_expected_version_returns_4091_with_latest_snapshot`
    - 场景：先成功更新版本，再以过期 `expected_version` 提交更新；
    - 断言：HTTP `409`、业务码 `4091`，并返回 `latest_snapshot`（含 `current_version`）。
  - `postgres_update_product_missing_resource_returns_4040_instead_of_4091`
    - 场景：目标商品不存在但携带 `expected_version`；
    - 断言：HTTP `404`、业务码 `4040`（不误判为 `4091`）。
- 新增测试基础设施：
  - `TEST_DATABASE_URL` 优先，回退 `DATABASE_URL`；
  - 未配置数据库连接时自动跳过 PG 集成测试（避免本地无库阻塞）；
  - `ensure_products_table(...)` 保障测试表存在；
  - `AtomicI64` 生成测试商品 ID，避免并发冲突。

本地验证结果：

- `cargo test`：通过（**96 passed; 0 failed**）
- 新增仓储层 2 个 PG 回归用例均通过。

远端验收结果（`ssh ubuntu@1.14.45.242:/projects/jxcServer`）：

- `rsync` 同步：通过
- 远端 `cargo test`：通过（**96 passed; 0 failed**）

## M3（第三阶段）第三步（第二十七部分）：Postgres 入/出库仓储回归补齐（2026-03-05）

本轮在“第二十六部分（商品 update 乐观锁）”基础上，继续补齐库存敏感链路在 Postgres 仓储层的回归覆盖，聚焦 `inbound/outbound` 的并发冲突与库存不足语义。

- 新增仓储层 Postgres 回归测试（`src/repository.rs`）：
  - `postgres_inbound_and_outbound_stale_expected_version_returns_4091_with_latest_snapshot`
    - 覆盖入库 stale `expected_version` -> `4091` + `latest_snapshot/current_version`；
    - 覆盖出库 stale `expected_version` -> `4091` + `latest_snapshot/current_version`。
  - `postgres_outbound_insufficient_stock_returns_4001_without_mutating_product`
    - 覆盖库存不足 -> `4001`；
    - 断言失败后商品 `current_stock/version` 不变，且无新增 `stock_logs`。
- 测试基础设施补齐：
  - `ensure_test_tables(...)` 统一建表入口；
  - 新增 `ensure_users_table(...)`、`ensure_stock_logs_table(...)`（满足操作人外键与流水断言依赖）；
  - 新增 `ensure_operator_user(...)`、`cleanup_inventory_fixture(...)`、`tenant_stock_log_count(...)` 等 helper，确保测试自包含与可重复执行。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**98 passed; 0 failed**）

远端验收结果（`ssh ubuntu@1.14.45.242:/projects/jxcServer`）：

- `rsync` 同步：通过
- 远端 `cargo test`：通过（**98 passed; 0 failed**）

## M3（第三阶段）第三步（第二十八部分）：Postgres 采购确认/销售确认仓储回归补齐（2026-03-05）

本轮在“第二十七部分（入/出库仓储回归）”基础上，继续补齐采购确认与销售确认两条库存敏感链路在 Postgres 仓储层的回归覆盖，聚焦 `4091` 版本冲突与 `4001` 库存不足语义。

- 新增仓储层 Postgres 回归测试（`src/repository.rs`）：
  - `postgres_confirm_purchase_order_stale_expected_version_returns_4091_with_latest_snapshot`
    - 首次确认采购单成功后，再以过期 `expected_version` 重试确认；
    - 断言返回 `4091`，并包含 `latest_snapshot/current_version`；
    - 断言 `stock_logs` 仅 1 条，避免重复入账。
  - `postgres_confirm_sales_order_insufficient_stock_returns_4001_without_mutating_order_and_product`
    - 确认销售单时库存不足；
    - 断言返回 `4001`；
    - 断言失败后销售单仍为 `DRAFT` 且 `version/confirmed_at` 不变；
    - 断言商品 `current_stock/version` 不变，且无新增 `stock_logs`。
- 测试基础设施补齐：
  - 扩展 `ensure_test_tables(...)`：新增 `ensure_audit_logs_table(...)`、`ensure_purchase_orders_tables(...)`、`ensure_sales_orders_tables(...)`；
  - 新增采购/销售单 fixture 构造：`sample_purchase_order(...)`、`sample_sales_order(...)`；
  - 扩展清理链路：`cleanup_tenant_audit_logs(...)`、`cleanup_tenant_purchase_orders(...)`、`cleanup_tenant_sales_orders(...)`；
  - 新增测试 ID 生成器：`next_test_purchase_order_id()`、`next_test_sales_order_id()`。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**100 passed; 0 failed**）

远端验收结果（`ssh ubuntu@1.14.45.242:/projects/jxcServer`）：

- `rsync` 同步：通过
- 远端 `cargo test`：通过（**100 passed; 0 failed**）
> 说明：当前仍以“内存态主流程 + Postgres 最小持久化通路”并行推进；其余业务表与完整事务迁移将在后续阶段继续补齐。

## M3（第三阶段）第三步（第二十九部分）：Postgres 采购作废仓储回归补齐（2026-03-05）

本轮在“第二十八部分（采购确认/销售确认仓储回归）”基础上，继续补齐采购作废链路在 Postgres 仓储层的关键语义回归，聚焦 `4091` 版本冲突与 `4001` 反向扣减库存不足场景。

- 新增仓储层 Postgres 回归测试（`src/repository.rs`）：
  - `postgres_void_purchase_order_stale_expected_version_returns_4091_with_latest_snapshot`
    - 采购单先确认成功后，使用过期 `expected_version` 执行作废；
    - 断言返回 `4091`，并携带 `latest_snapshot/current_version`；
    - 断言 `stock_logs` 仅保留确认阶段 1 条，不产生重复回滚流水。
  - `postgres_void_purchase_order_insufficient_stock_returns_4001_without_mutating_order_and_product`
    - 采购单确认入库后，通过出库降低库存，再执行作废触发反向扣减库存不足；
    - 断言返回 `4001`；
    - 断言失败后采购单保持 `CONFIRMED`，且 `version/voided_at` 不变；
    - 断言商品保持失败前快照，且 `stock_logs` 仅有 `IN_PURCHASE + OUT_SALE` 共 2 条（无 `VOID_PURCHASE`）。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**102 passed; 0 failed**）

远端验收结果（`ssh ubuntu@1.14.45.242:/projects/jxcServer`）：

- `rsync` 同步：通过
- 远端 `cargo test`：通过（**102 passed; 0 failed**）

## M3（第三阶段）第三步（第三十部分）：Postgres 盘点启动/确认仓储回归补齐（2026-03-05）

本轮在“第二十九部分（采购作废仓储回归）”基础上，继续补齐盘点链路在 Postgres 仓储层的关键冲突语义回归，聚焦盘点开始 `4091` 版本冲突，以及盘点确认时 `book_stock` 漂移导致的 `4091` 失败保护。

- 新增仓储层 Postgres 回归测试（`src/repository.rs`）：
  - `postgres_start_stock_check_stale_expected_version_returns_4091_with_latest_snapshot`
    - 盘点单首次开始成功后，再以过期 `expected_version` 重试开始；
    - 断言返回 `4091`，并携带 `latest_snapshot/current_version`；
    - 断言盘点单保持 `COUNTING` 且版本不回退，且 `stock_logs` 仍为 0（开始盘点不写库存流水）。
  - `postgres_confirm_stock_check_book_stock_drift_returns_4091_without_mutating_check_and_product`
    - 盘点单进入 `COUNTING` 后先通过出库制造库存漂移，再执行确认；
    - 断言返回 `4091`（`resource=product`），并携带 `book_stock/current_stock/latest_snapshot`；
    - 断言失败后盘点单仍为 `COUNTING`、`confirmed_at` 为空、明细 `actual_stock/delta_qty` 不被写入；
    - 断言商品保持漂移后的快照，且 `stock_logs` 仅有 1 条（仅出库流水，无盘点确认流水）。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**104 passed; 0 failed**）

远端验收结果（`ssh ubuntu@1.14.45.242:/projects/jxcServer`）：

- `rsync` 同步：通过
- 远端 `cargo test`：通过（**104 passed; 0 failed**）

## M3（第三阶段）第三步（第三十一部分）：Postgres 销售作废/退货 4090 仓储回归补齐（2026-03-05）

本轮在“第三十部分（盘点 start/confirm 仓储回归）”基础上，继续补齐销售链路在 Postgres 仓储层的业务冲突语义回归，聚焦 `4090` 场景下的失败后不变性保障。

- 新增仓储层 Postgres 回归测试（`src/repository.rs`）：
  - `postgres_void_sales_order_non_draft_returns_4090_without_mutating_order_and_stock_logs`
    - 销售单先确认后，再执行作废（非 `DRAFT` 场景）；
    - 断言返回 `4090`，并携带当前状态 `status=CONFIRMED`；
    - 断言失败后单据保持 `CONFIRMED`，且 `version/voided_at` 不变；
    - 断言 `stock_logs` 仍为 1 条（仅确认产生的 `OUT_SALE`）。
  - `postgres_return_sales_order_over_remaining_qty_returns_4090_without_mutating_order_product_and_stock_logs`
    - 销售单确认后对同商品执行超可退数量退货；
    - 断言返回 `4090`，并携带 `product_id/remain_qty/request_qty`；
    - 断言失败后销售单 `status/version/returned_at/returned_qty` 不变；
    - 断言商品 `current_stock/version` 不变，且 `stock_logs` 仍为 1 条（无新增退货流水）。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**106 passed; 0 failed**）

远端验收结果（`ssh ubuntu@1.14.45.242:/projects/jxcServer`）：

- `rsync` 同步：通过
- 远端 `cargo test`：通过（**106 passed; 0 failed**）

## M3（第三阶段）第三步（第三十二部分）：Postgres 销售作废/退货 4091 仓储回归补齐（2026-03-05）

本轮在“第三十一部分（销售作废/退货 4090 业务冲突）”基础上，继续补齐销售链路在 Postgres 仓储层的乐观锁冲突语义回归，聚焦 `4091` 场景下的失败后不变性保障。

- 新增仓储层 Postgres 回归测试（`src/repository.rs`）：
  - `postgres_void_sales_order_stale_expected_version_returns_4091_with_latest_snapshot`
    - 销售单在 `DRAFT` 状态首次作废成功后，再以过期 `expected_version` 重试作废；
    - 断言返回 `4091`，并携带 `resource/resource_id/expected_version/current_version/latest_snapshot`；
    - 断言失败后单据保持首次作废后的 `VOIDED` 状态，`version/voided_at` 不变；
    - 断言 `stock_logs` 仍为 0（作废草稿单不应写库存流水）。
  - `postgres_return_sales_order_stale_expected_version_returns_4091_without_mutating_order_product_and_stock_logs`
    - 销售单确认后，以过期 `expected_version` 执行退货；
    - 断言返回 `4091`，并携带 `latest_snapshot/current_version`；
    - 断言失败后销售单 `status/version/returned_at/returned_qty` 不变；
    - 断言商品 `current_stock/version` 不变，且 `stock_logs` 保持确认后的 1 条（无新增 `RETURN_SALE`）。

本地验证结果：

- `cargo fmt --all`：通过
- `cargo check`：通过
- `cargo test`：通过（**108 passed; 0 failed**）
