# 🔌 SaaS 进销存（JXC）API 接口定义文档

| 文档属性 | 内容 |
| --- | --- |
| 版本 | v1.1.0（优化版） |
| Base URL | `https://api.yoursaas.com/api/v1` |
| 协议 | HTTPS + JSON |
| 鉴权 | `Authorization: Bearer <access_token>` |

---

## 1. 全局规范（Global Standards）

## 1.1 统一响应结构

```json
{
  "code": 200,
  "message": "success",
  "data": {},
  "request_id": "req_20260302_xxx"
}
```

- `code`：业务码（非 HTTP 状态码）
- `message`：可读提示
- `data`：业务数据（对象或数组）
- `request_id`：链路追踪 ID

## 1.2 Header 规范
- `Authorization: Bearer <token>`（除注册/登录/刷新外均必填）
- `X-Idempotency-Key: <uuid>`（所有写接口必填）
- `X-Client-Type: windows | android`（可选，便于统计）

## 1.3 数据类型规范
- 金额字段统一为**字符串**：如 `"12.50"`。
- 时间统一 ISO-8601：`2026-03-02T12:00:00Z`。
- ID 类型：
  - 租户/用户：UUID 字符串
  - 业务主键：`int64`

## 1.4 分页规范
- 请求：`page`（从1开始）、`page_size`（默认20，最大100）
- 响应：

```json
{
  "list": [],
  "total": 0,
  "page": 1,
  "page_size": 20
}
```

## 1.5 统一错误码（节选）

| code | 含义 |
| --- | --- |
| 200 | 成功 |
| 4000 | 参数错误 |
| 4001 | 库存不足 |
| 4002 | 条码已存在 |
| 4003 | 幂等键重复请求 |
| 4010 | 未登录或 Token 失效 |
| 4030 | 无权限 |
| 4040 | 资源不存在 |
| 4090 | 数据冲突 |
| 4091 | 版本冲突（离线同步） |
| 4092 | 幂等键请求体冲突 |
| 5000 | 系统内部错误 |

---

## 2. 认证模块（Auth）

## 2.1 租户注册
- **POST** `/auth/register`
- **鉴权**：否

**Request**
```json
{
  "tenant_name": "华东便利店",
  "username": "owner_hddb",
  "name": "门店管理员",
  "password": "owner123456"
}
```

> `tenant_name` 可选；`username/name/password` 必填且不能为空。

**Response.data**
```json
{
  "registered": true,
  "tenant_name": "华东便利店",
  "access_token": "<jwt>",
  "refresh_token": "<jwt>",
  "expires_in": 7200,
  "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
  "user_info": {
    "id": "a3ec...",
    "name": "门店管理员",
    "role": "OWNER"
  }
}
```

**错误语义（典型）**
- `4000`：必填字段为空。
- `4090`：用户名已存在（`data.username` 返回冲突用户名）。

## 2.2 用户登录
- **POST** `/auth/login`
- **鉴权**：否

**Request**
```json
{
  "username": "admin",
  "password": "my_password"
}
```

**Response.data**
```json
{
  "access_token": "<jwt>",
  "refresh_token": "<jwt>",
  "expires_in": 7200,
  "tenant_id": "550e8400-e29b-41d4-a716-446655440000",
  "user_info": {
    "id": "a3ec...",
    "name": "张三",
    "role": "OWNER"
  }
}
```

## 2.3 刷新令牌
- **POST** `/auth/refresh`
- **鉴权**：否（使用 refresh_token）

**Request**
```json
{
  "refresh_token": "<jwt>"
}
```

## 2.4 退出登录
- **POST** `/auth/logout`
- **鉴权**：是

## 2.5 员工列表（仅 OWNER）
- **GET** `/users`
- **鉴权**：是
- **权限**：`OWNER`

**Response.data**
```json
{
  "list": [
    {
      "id": "a3ec0000-0000-0000-0000-000000000001",
      "username": "sales01",
      "name": "王五",
      "role": "SALES"
    }
  ],
  "total": 1
}
```

## 2.6 新增员工（仅 OWNER）
- **POST** `/users`
- **鉴权**：是
- **权限**：`OWNER`

**Request**
```json
{
  "username": "sales01",
  "name": "王五",
  "role": "SALES",
  "password": "password123"
}
```

**Response.data**：返回新增员工对象（同列表项结构）

## 2.7 修改员工角色（仅 OWNER）
- **PATCH** `/users/{id}/role`
- **鉴权**：是
- **权限**：`OWNER`

**Request**
```json
{
  "role": "PURCHASER"
}
```

## 2.8 重置员工密码（仅 OWNER）
- **POST** `/users/{id}/reset-password`
- **鉴权**：是
- **权限**：`OWNER`

**Request**
```json
{
  "new_password": "newpassword123"
}
```

---

## 3. 商品模块（Products）

## 3.1 扫码查询商品
- **GET** `/products/scan?barcode=690123456789`

**Response.data（存在）**
```json
{
  "id": 1001,
  "sku": "KO-330",
  "barcode": "690123456789",
  "name": "可口可乐 330ml",
  "unit": "罐",
  "current_stock": 150,
  "retail_price": "3.50",
  "cost_price": "2.10"
}
```

> 若不存在返回 `4040`。

## 3.2 新建商品（极简）
- **POST** `/products`

**Request**
```json
{
  "barcode": "690123456789",
  "name": "百事可乐",
  "unit": "瓶",
  "retail_price": "3.00",
  "wholesale_price": "2.80",
  "init_stock": 10,
  "min_stock_limit": 5
}
```

## 3.3 商品列表
- **GET** `/products?page=1&page_size=20&keyword=可乐&category_id=1`

**Response.data**：分页结构（`list/total/page/page_size`）

---

## 4. 库存模块（Inventory）

## 4.1 采购入库（核心）
- **POST** `/inventory/inbound`

**Request**
```json
{
  "product_id": 1001,
  "barcode": "690123456789",
  "qty": 50,
  "unit_cost": "2.20",
  "expected_version": 12,
  "biz_no": "PO-20260302-01",
  "remark": "首批到货"
}
```

**后端必做逻辑**
1. 参数校验与权限校验。
2. 事务开启。
3. `SELECT ... FOR UPDATE` 锁定商品。
4. 计算 `cost_price`（移动加权平均）。
5. 更新 `current_stock` + `cost_price`。
6. 写 `stock_logs`（`biz_type=IN_PURCHASE`）。
7. 提交事务。

## 4.2 销售出库（核心）
- **POST** `/inventory/outbound`

**Request**
```json
{
  "biz_no": "SO-20260302-99",
  "customer_id": 2001,
  "expected_version": 33,
  "items": [
    {"product_id": 1001, "qty": 2, "sell_price": "3.50"},
    {"product_id": 1002, "qty": 1, "sell_price": "10.00"}
  ],
  "remark": "门店零售"
}
```

## 4.2.1 销售单状态流转
- `DRAFT -> CONFIRMED -> RETURNED_PARTIAL / RETURNED_FULL`
- 已 `CONFIRMED` 单据不允许删除；退货需调用专用接口。

## 4.2.2 销售单作废（仅草稿）
- **POST** `/sales-orders/{id}/void`
- 仅允许 `DRAFT` 状态作废。

## 4.3 采购单状态流转与作废
- 状态：`DRAFT -> CONFIRMED -> VOIDED`
- **POST** `/purchase-orders/{id}/void`
- 已 `CONFIRMED` 作废时，服务端必须生成反向库存流水（审计可追踪）。

**库存不足错误示例**
```json
{
  "code": 4001,
  "message": "库存不足",
  "data": {
    "failed_product_id": 1001,
    "available_stock": 1,
    "required_qty": 2
  },
  "request_id": "req_xxx"
}
```

## 4.4 库存盘点确认
- **POST** `/inventory/stock-check/confirm`

**Request**
```json
{
  "biz_no": "SC-20260302-01",
  "items": [
    {"product_id": 1001, "book_stock": 100, "actual_stock": 98},
    {"product_id": 1002, "book_stock": 50, "actual_stock": 55}
  ]
}
```

## 4.5 库存流水查询（库存追溯）
- **GET** `/inventory/logs`
- **鉴权**：是
- **权限**：`OWNER`、`PURCHASER`

**Query 参数**

| 参数 | 必填 | 说明 |
| --- | --- | --- |
| `page` | 否 | 页码，默认 `1` |
| `page_size` | 否 | 每页大小，默认 `20`，最大 `100` |
| `biz_type` | 否 | 业务类型（如 `IN_PURCHASE` / `OUT_SALE` / `RETURN_SALE` / `ADJ_CHECK` / `VOID_PURCHASE`） |
| `biz_no` | 否 | 业务单号（模糊匹配） |
| `product_id` | 否 | 商品 ID（正整数） |
| `operator_id` | 否 | 操作人 UUID |
| `start_date` | 否（与 `end_date` 成对） | 开始日期，格式 `YYYY-MM-DD` |
| `end_date` | 否（与 `start_date` 成对） | 结束日期，格式 `YYYY-MM-DD` |

**Response.data**
```json
{
  "list": [
    {
      "id": 101,
      "product_id": 1001,
      "biz_type": "OUT_SALE",
      "biz_no": "SO-20260306-01",
      "delta_qty": -2,
      "snapshot_stock": 98,
      "snapshot_cost": "2.1000",
      "operator_id": "a3ec0000-0000-0000-0000-000000000001",
      "created_at": "2026-03-06T09:20:11Z"
    }
  ],
  "total": 1,
  "page": 1,
  "page_size": 20
}
```

**参数校验与错误语义**
- `product_id <= 0`、`operator_id` 非 UUID、日期格式错误、仅传 `start_date/end_date`、`start_date > end_date`：返回 `4000`。
- 权限不足：返回 `4030`。

## 4.6 审计日志查询（操作追溯）
- **GET** `/audit/logs`
- **鉴权**：是
- **权限**：仅 `OWNER`

**Query 参数**

| 参数 | 必填 | 说明 |
| --- | --- | --- |
| `page` | 否 | 页码，默认 `1` |
| `page_size` | 否 | 每页大小，默认 `20`，最大 `100` |
| `action` | 否 | 操作动作（如 `STOCK_CHECK_CONFIRM`、`PURCHASE_ORDER_VOID`） |
| `target_type` | 否 | 审计对象类型（如 `stock_check`、`sales_order`、`purchase_order`） |
| `operator_id` | 否 | 操作人 UUID |
| `request_id` | 否 | 请求链路 ID 精确匹配 |
| `start_date` | 否（与 `end_date` 成对） | 开始日期，格式 `YYYY-MM-DD` |
| `end_date` | 否（与 `start_date` 成对） | 结束日期，格式 `YYYY-MM-DD` |

**Response.data**
```json
{
  "list": [
    {
      "id": 88,
      "operator_id": "a3ec0000-0000-0000-0000-000000000001",
      "action": "STOCK_CHECK_CONFIRM",
      "target_type": "stock_check",
      "target_id": "5001",
      "before_data": {
        "status": "COUNTING",
        "version": 2
      },
      "after_data": {
        "status": "CONFIRMED",
        "version": 3
      },
      "request_id": "req_audit_001",
      "created_at": "2026-03-06T09:30:21Z"
    }
  ],
  "total": 1,
  "page": 1,
  "page_size": 20
}
```

**参数校验与错误语义**
- `operator_id` 非 UUID、日期格式错误、仅传 `start_date/end_date`、`start_date > end_date`：返回 `4000`。
- 非 `OWNER` 访问：返回 `4030`。

---

## 5. 报表模块（Reports）

## 5.1 首页看板
- **GET** `/reports/dashboard?date=2026-03-02`

**Response.data**
```json
{
  "total_sales": "1250.50",
  "total_gross_profit": "320.30",
  "total_orders": 45,
  "low_stock_count": 3,
  "top_selling_item": "可口可乐"
}
```

## 5.2 销售报表
- **GET** `/reports/sales?start_date=2026-03-01&end_date=2026-03-31&group_by=product&page=1&page_size=20`

## 5.3 销售报表导出
- **GET** `/reports/sales/export?start_date=2026-03-01&end_date=2026-03-31&group_by=product&format=xlsx`
- **鉴权**：是
- **权限**：`OWNER`、具备授权的 `PURCHASER`

**Query 参数**

| 参数 | 必填 | 说明 |
| --- | --- | --- |
| `start_date` | 否（与 `end_date` 成对） | 开始日期，格式 `YYYY-MM-DD` |
| `end_date` | 否（与 `start_date` 成对） | 结束日期，格式 `YYYY-MM-DD` |
| `group_by` | 否 | 当前仅支持 `product`，默认 `product` |
| `format` | 否 | 支持 `csv` / `xlsx`，默认 `csv` |

**Response（成功）**

- HTTP Status: `200`
- 当 `format=csv`：
  - Header：
    - `Content-Type: text/csv; charset=utf-8`
    - `Content-Disposition: attachment; filename="sales_report_<start>_<end>.csv"`
  - Body：CSV 文件流（UTF-8 BOM）
    - 表头：`product_id,product_name,total_qty,total_sales,total_cost,gross_profit`
    - 末尾汇总行：`SUMMARY,,total_qty,total_sales,total_cost,total_gross_profit`
- 当 `format=xlsx`：
  - Header：
    - `Content-Type: application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`
    - `Content-Disposition: attachment; filename="sales_report_<start>_<end>.xlsx"`
  - Body：xlsx 二进制文件流（Sheet1 与 CSV 同口径，含 `SUMMARY` 汇总行）
  - 展示增强（不改变统计口径）：
    - 表头样式（加粗、居中、背景色、边框）
    - 汇总行样式（加粗、背景色、边框）
    - 冻结首行（便于滚动查看明细）
    - 明细区自动筛选
    - 列宽预设（可读性优化）
    - 数量/金额字段以数值单元格写入（便于 Excel 二次分析）

**参数错误示例（仍遵循统一错误响应）**

- `format` 非 `csv/xlsx` -> `4000`
- `group_by` 非 `product` -> `4000`
- 仅传 `start_date` 或仅传 `end_date` -> `4000`
- `start_date > end_date` -> `4000`

---

## 6. 幂等与离线同步约定

1. 所有写接口必须携带 `X-Idempotency-Key`。
2. 相同 `tenant_id + method + path + key` 的重复请求，返回首次处理结果。
3. App 离线重放时沿用同一个幂等键，防止重复入库/出库。
4. 相同 key 但请求体不一致，返回 `4092`。
5. 重放响应建议 Header：`X-Idempotent-Replay: true`。

### 6.1 版本冲突返回示例（4091）

```json
{
  "code": 4091,
  "message": "版本冲突，请刷新后重试",
  "data": {
    "resource": "product",
    "resource_id": 1001,
    "expected_version": 12,
    "current_version": 15,
    "latest_snapshot": {
      "id": 1001,
      "current_stock": 96,
      "cost_price": "2.34",
      "version": 15
    }
  },
  "request_id": "req_xxx"
}
```

---

## 7. 安全约束

1. Token 过期返回 `4010`。
2. 权限不足返回 `4030`。
3. 所有接口在服务端强制注入并校验 `tenant_id`。
4. 严禁客户端传入 `tenant_id` 作为可信字段。

## 7.1 操作级权限约束（节选）
- `SALES` 不可查看 `cost_price`。
- 导出经营报表仅 `OWNER` 与具备授权的 `PURCHASER`。
- 权限变更接口仅 `OWNER` 可调用。

---

## 8. OpenAPI 落地建议

建议后续将本文件转换为 `openapi.yaml`，用于：
- 自动生成 Rust/Flutter/TS SDK
- 自动参数校验与接口测试
- 与前后端联调平台集成
