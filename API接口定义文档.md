# 🔌 SaaS 进销存（JXC）API 接口定义文档

| 文档属性 | 内容 |
| --- | --- |
| 版本 | v1.3.0（新增：经营趋势查询接口 `GET /reports/trend`） |
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

### 1.6 请求体解析错误兼容约定（新增）

- 当请求体 JSON 语法错误、字段类型不匹配、或 `Content-Type` 与期望不符导致后端解析失败时，服务端必须返回标准响应结构：

```json
{
  "code": 4000,
  "message": "请求体解析失败：...",
  "data": {},
  "request_id": "req_xxx"
}
```

- 不允许返回默认纯文本错误页/纯文本错误信息。
- 客户端应始终按统一结构处理错误，不需要为该场景额外解析非 JSON body。

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

> 移动端（Android）调用约束：
> - 读取能力：`OWNER/PURCHASER/SALES` 均可调用列表、详情。
> - 写入能力：仅 `OWNER/PURCHASER` 可调用新建、更新、删除。
> - `SALES` 调用商品读取接口时，`cost_price` 为 `null`。

## 3.1 条码查询商品（通用能力）
- **GET** `/products/scan?barcode=690123456789`
- **鉴权**：是
- **权限**：`OWNER`、`PURCHASER`、`SALES`

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
  "last_inbound_unit_cost": "2.20",
  "cost_price": "2.10",
  "version": 12,
  "min_stock_limit": 10
}
```

> 若不存在返回 `4040`。

**SALES 角色响应差异**
```json
{
  "id": 1001,
  "sku": "KO-330",
  "barcode": "690123456789",
  "name": "可口可乐 330ml",
  "unit": "罐",
  "current_stock": 150,
  "retail_price": "3.50",
  "last_inbound_unit_cost": "2.20",
  "cost_price": null,
  "version": 12,
  "min_stock_limit": 10
}
```

## 3.1.A 条码建议名称查询（新建商品辅助）
- **GET** `/products/barcode-lookup?barcode=690123456789`
- **鉴权**：是
- **权限**：`OWNER`、`PURCHASER`、`SALES`
- **Header**：不要求 `X-Idempotency-Key`（读接口）

**Query 参数**

| 参数 | 必填 | 说明 |
| --- | --- | --- |
| `barcode` | 是 | 待查询条码，去首尾空格后不能为空 |

**Response.data（FOUND 示例）**
```json
{
  "barcode": "690123456789",
  "status": "FOUND",
  "suggested_name": "可口可乐 330ml",
  "cache_hit": true,
  "source": "CACHE"
}
```

**Response.data（NOT_FOUND 示例）**
```json
{
  "barcode": "9999999999999",
  "status": "NOT_FOUND",
  "suggested_name": null,
  "cache_hit": false,
  "source": "THIRD_PARTY"
}
```

**字段语义**
- `status`：`FOUND` / `NOT_FOUND`
- `suggested_name`：建议商品名称，可为空（`NOT_FOUND` 时为空）
- `cache_hit`：是否命中本地缓存（含有效缓存与过期回退缓存）
- `source`：
  - `CACHE`：本地缓存命中且未过期
  - `THIRD_PARTY`：第三方回源结果
  - `CACHE_STALE`：第三方失败时使用过期缓存降级
  - `DEGRADED`：第三方失败且无可用缓存，降级返回 `NOT_FOUND`

**典型错误语义**
- `4000`：`barcode` 缺失或为空。
- `4030`：无权限（保留统一权限语义）。
- `5000`：系统内部错误。

> 说明：该接口用于“新建商品自动命名建议”，不替代 `/products/scan` 的本地商品建档查询语义。

## 3.2 新建商品（极简）
- **POST** `/products`
- **鉴权**：是
- **权限**：`OWNER`、`PURCHASER`
- **Header**：必须携带 `X-Idempotency-Key`

**Request**
```json
{
  "sku": "KO-330",
  "barcode": "690123456789",
  "name": "百事可乐",
  "unit": "瓶",
  "retail_price": "3.00",
  "init_stock": 10,
  "min_stock_limit": 5,
  "cost_price": "2.10"
}
```

**字段说明**
- `sku`：可选，空时服务端自动生成（如 `SKU-<id>`）。
- `init_stock`：可选，默认 `0`，必须 `>= 0`。
- `min_stock_limit`：可选，默认 `0`，必须 `>= 0`。
- `cost_price`：必填，必须为合法金额字符串（如 `"2.10"`），不允许缺失或空字符串。

**典型错误语义**
- `4002`：条码冲突（返回冲突条码与已有商品ID）。
- `4090`：SKU 冲突。
- `4030`：无写权限（如 `SALES`）。
- `4092`：幂等键请求体冲突。

## 3.3 商品列表
- **GET** `/products?page=1&page_size=20&keyword=可乐&barcode=690123456789`
- **鉴权**：是
- **权限**：`OWNER`、`PURCHASER`、`SALES`

**Query 参数**

| 参数 | 必填 | 说明 |
| --- | --- | --- |
| `page` | 否 | 页码，默认 `1` |
| `page_size` | 否 | 每页大小，默认 `20`，最大 `100` |
| `keyword` | 否 | 名称/SKU/条码模糊搜索 |
| `barcode` | 否 | 条码精确匹配 |
| `low_stock` | 否 | 传 `true` 时仅返回满足 `current_stock < min_stock_limit` 的商品，并按库存缺口（`min_stock_limit - current_stock`）从大到小排序 |

**Response.data**
```json
{
  "list": [
    {
      "id": 1001,
      "sku": "KO-330",
      "barcode": "690123456789",
      "name": "可口可乐 330ml",
      "unit": "罐",
      "current_stock": 150,
      "retail_price": "3.50",
      "last_inbound_unit_cost": "2.20",
      "cost_price": "2.10",
      "min_stock_limit": 10,
      "version": 12
    }
  ],
  "total": 1,
  "page": 1,
  "page_size": 20
}
```

> `SALES` 角色下 `list[*].cost_price` 返回 `null`。

### 3.3.A Android 看板“低库存商品数”下钓展示约定（本次新增）

1. Android 首页看板“低库存商品数”指标卡支持点击，点击后进入低库存商品列表页。
2. 列表页复用 `GET /products` 并固定传递 `low_stock=true`，不新增独立接口路径。
3. 首屏默认参数：`low_stock=true`、`page=1`、`page_size=50`。
4. 列表卡片展示字段：
   - `name`（商品名称）
   - `current_stock`（当前库存）
   - `min_stock_limit`（库存预警阈値）
   - `min_stock_limit - current_stock`（还差 N 欠达标，客户端计算）
   - 库存进度条：`current_stock / min_stock_limit`（客户端计算）
5. 权限边界：三种角色均可访问；`SALES` 角色下 `cost_price` 返回 `null`，前端不展示该字段。
6. 列表为空时展示空状态提示：“当前没有低库存商品，库存充足 🎉”。
7. 本约定仅添加可选查询参数，不改变现有分页、权限与错误码语义。


## 3.4 商品详情
- **GET** `/products/{id}`
- **鉴权**：是
- **权限**：`OWNER`、`PURCHASER`、`SALES`

**Response.data**：结构同 `ProductData`（与列表项一致）。

## 3.5 更新商品
- **PUT** `/products/{id}`
- **鉴权**：是
- **权限**：`OWNER`、`PURCHASER`
- **Header**：必须携带 `X-Idempotency-Key`

**Request（全字段可选，至少传一个）**
```json
{
  "sku": "KO-330",
  "barcode": "690123456789",
  "name": "可口可乐 330ml（新包装）",
  "unit": "罐",
  "retail_price": "3.80",
  "cost_price": "2.30",
  "min_stock_limit": 12,
  "expected_version": 12
}
```

**典型错误语义**
- `4091`：版本冲突（返回 `latest_snapshot`）。
- `4002`：条码冲突。
- `4090`：SKU 冲突。
- `4030`：无写权限。

**字段补充说明**
- `last_inbound_unit_cost`：最近一次成功入库的进货价格，可能为空（`null`）。
- `cost_price`：商品当前移动加权成本价。

## 3.6 删除商品
- **DELETE** `/products/{id}?expected_version=12`
- **鉴权**：是
- **权限**：`OWNER`、`PURCHASER`
- **Header**：必须携带 `X-Idempotency-Key`

**业务约束**
- 仅允许删除 `current_stock = 0` 的商品。

**Response.data**
```json
{
  "id": 1001,
  "deleted": true,
  "version": 13
}
```

**典型错误语义**
- `4090`：存在库存，无法删除。
- `4091`：版本冲突。
- `4030`：无写权限。

### 3.7 Android 端推荐调用序列（商品管理）

1. 首次进入“商品列表页”调用 `GET /products`（默认分页）。
2. 用户在列表页通过关键字/条码筛选定位商品；条码输入支持手工/扫码枪/手机摄像头回填。
3. `OWNER/PURCHASER` 通过 FAB 打开“新建商品 BottomSheet”，提交时调用 `POST /products`。
4. 点击“编辑”进入独立全屏编辑页，保存时调用 `PUT /products/{id}` 并携带 `expected_version`。
5. 编辑页或列表页删除操作调用 `DELETE /products/{id}` 并携带 `expected_version`（建议默认使用当前行 `version`）。
6. 收到 `4091` 时先刷新列表/详情，再提示用户重试。
7. `SALES` 角色仅允许列表/详情只读访问，不展示新建、编辑、删除写操作入口。

### 3.8 Android 条码输入与摄像头扫码并行约定（本次新增）

1. 本文档中涉及 `barcode` 参数的 Android 输入位，均允许两种来源：
   - 扫码枪/手工输入；
   - 手机摄像头扫码后自动回填。
2. 摄像头扫码仅改变输入来源，不改变接口、参数名、错误码和权限语义。
3. 对“扫码即触发动作”页面（入库/出库/盘点），客户端可在回填后立即发起既有 API 调用。
4. 对“商品管理”页面，摄像头扫码用于回填条码筛选、新建表单、编辑表单，不新增独立扫码查询接口入口。
5. 权限与错误处理口径保持一致：
   - 无权限仍返回 `4030`
   - 条码未建档返回 `4040`
   - 版本冲突返回 `4091`

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
  "expected_version": 12,
  "remark": "首批到货"
}
```

> `unit_cost` 为可选字段：
> - 传值时使用传入进货价格参与本次入库成本计算；
> - 不传或传空字符串时，服务端按 `last_inbound_unit_cost ?? cost_price` 计算本次有效进货价格（`effective_unit_cost`）。

> `biz_no` 由服务端自动生成；老客户端若传入该字段，服务端不作为创建主依据。

### 4.1.A 入库前端扫码模式约定（本次新增）

1. 入库前端支持两种扫码模式：
   - `scan_confirm`（默认）：扫码命中后先确认数量/进价再写入；
   - `continuous_scan`：开启摄像头后持续识别，命中后继续下一条，手动结束会话后关闭摄像头。
2. 两种模式均复用 `POST /inventory/inbound`，不新增后端接口、不新增请求字段。
3. 在 `continuous_scan` 模式下，客户端每次识别成功后可展示页面成功提示并播放语音提示；该反馈属于前端交互层，不影响 API 契约。
4. 在 `continuous_scan` 模式下，客户端应执行同码短时去重，避免同一条码短时间重复入列。
5. 兼容迁移：若客户端读取到历史模式值 `quick_accumulate`，需自动映射为 `continuous_scan` 并写回本地偏好。

### 4.1.C 扫码模式记忆与确认后续扫约定（本次新增）

1. 客户端应按“用户 + 终端 + 页面”维度记忆扫码模式选择（入库/出库分别记忆）。
2. 首次无记录时使用默认模式：入库 `scan_confirm`，出库 `scan_confirm`。
3. `scan_confirm` 模式下，客户端可开启“确认后续扫”会话：
   - 条码命中后弹确认表单；
   - 用户确认写入后，自动回到下一次扫码等待态；
   - 提供“结束扫码”动作。
4. 上述行为属于客户端交互增强，不新增请求字段，不修改 `POST /inventory/inbound` 契约。
5. 兼容迁移：读取到历史值 `quick_accumulate` 时，客户端应映射并持久化为 `continuous_scan`。

### 4.1.D 入库确认写入弹窗化约定（本次新增）

1. 仅在入库 `scan_confirm` 模式下启用确认写入弹窗（Modal/Dialog），`continuous_scan` 不受影响。
2. 扫码命中后，客户端通过弹窗承载“商品信息 + 数量输入”确认流程，替代页面内联确认区。
3. 弹窗确认成功后自动回到扫码等待态（聚焦扫码输入框），延续既有“确认后续扫”会话语义。
4. 快捷键约定：
   - Windows：`Enter` 确认写入，`Esc` 取消本次确认；
   - Android：支持键盘回车快速确认（扫码枪/外接键盘场景）。
5. 本能力仅为客户端交互层优化，不新增后端字段，不改变 `POST /inventory/inbound` 请求/响应结构、错误码与权限语义。
6. 入库提交成功后的客户端重置约定：
   - 清空 `product_id`、`barcode` 及扫码输入框条码，避免误复用上一次商品上下文；
   - `qty` 恢复默认值 `1`，用于下一笔入库初始录入；
   - `unit_cost/expected_version/remark` 按既有规则清空。
7. 本约定仅影响客户端表单状态管理，不改变后端接口入参定义与业务处理逻辑。

### 4.1.B 入库扫码未命中直达建档约定（本次新增）

1. 当入库扫码依赖的本地商品查询返回 `4040`（条码未建档）时，客户端应提示用户可立即新建商品。
2. 对 `OWNER/PURCHASER`：
   - 展示主动作“新建商品”；
   - 跳转商品创建入口时自动回填未命中条码。
3. 对无建档权限角色：
   - 不展示“新建商品”动作；
   - 提示“无建档权限，请联系管理员”。
4. 商品创建成功后，客户端应返回入库上下文并回填商品标识（`product_id`/`barcode`）供继续入库。
5. 本约定为客户端流程编排，不新增后端接口、不修改错误码与响应结构语义。

**后端必做逻辑**
1. 参数校验与权限校验。
2. 事务开启。
3. `SELECT ... FOR UPDATE` 锁定商品。
4. 计算本次有效成本 `effective_unit_cost`（优先 `unit_cost`，缺省回落商品当前 `cost_price`），并据此计算 `cost_price`（移动加权平均）。
5. 更新 `current_stock` + `cost_price`。
6. 写 `stock_logs`（`biz_type=IN_PURCHASE`）。
7. 提交事务。

## 4.1.1 批量采购入库（本次新增）
- **POST** `/inventory/inbound/batch`
- **鉴权**：是
- **权限**：`OWNER`、`PURCHASER`
- **Header**：必须携带 `X-Idempotency-Key`

**Request**
```json
{
  "items": [
    {
      "product_id": 1001,
      "barcode": "690123456789",
      "qty": 10,
      "unit_cost": "2.20",
      "expected_version": 12,
      "remark": "首批到货"
    },
    {
      "product_id": 1002,
      "qty": 5,
      "expected_version": 3
    }
  ]
}
```

**字段说明**
- `items`：必填，至少 1 条。
- `items[*].product_id / items[*].barcode`：二选一，至少提供一个。
- `items[*].qty`：必填，正整数。
- `items[*].unit_cost`：可选；不传或空字符串时回落商品当前 `cost_price`。
- `items[*].expected_version`：可选；用于并发冲突校验，失败返回 `4091`。
- `items[*].remark`：可选；仅作为该明细业务备注，不改变成本与库存计算语义。

**Response.data**
```json
{
  "biz_no": "IN-20260312-01",
  "items": [
    {
      "product_id": 1001,
      "current_stock": 110,
      "cost_price": "2.1091",
      "version": 13
    },
    {
      "product_id": 1002,
      "current_stock": 55,
      "cost_price": "3.2000",
      "version": 4
    }
  ]
}
```

**事务与一致性约束**
1. 整批 `items` 必须在单事务内处理，任一明细失败则整批回滚。
2. 批次内所有库存流水共用同一个服务端生成 `biz_no`。
3. 每条明细均执行 `FOR UPDATE` 锁 + 成本重算 + 库存更新 + 流水写入。

**典型错误语义**
- `4000`：`items` 为空、`qty` 非法、`product_id/barcode` 同时缺失等参数错误。
- `4030`：无权限。
- `4040`：任一商品不存在（整批回滚）。
- `4091`：任一明细版本冲突（整批回滚）。
- `4092`：幂等键请求体冲突。

### 4.1.2 入库页批量接入编排约定（本次新增）

1. Windows 与 Android 入库页必须支持“多商品明细暂存”能力，不得再以“当前表单已有商品”为由阻断跨商品扫码。
2. 客户端提交策略：
   - 明细数 `=1`：允许继续调用 `POST /inventory/inbound`；
   - 明细数 `>=2`：默认调用 `POST /inventory/inbound/batch`。
3. 批量提交时，客户端仅负责按 `items[]` 组包，不引入额外后端字段；事务原子性仍由服务端保证。
4. 批量成功后，客户端结果区应展示服务端返回的同一 `biz_no` 及每条明细回写结果。
5. 本约定为客户端编排层增强，不改变权限、错误码及幂等语义。

### 4.1.3 入库待提交明细商品名称与进价编辑约定（本次新增）

1. 入库页“待提交明细”展示层必须包含 `product_name`，不允许仅显示 `product_id/barcode`。
2. 待提交明细中的 `unit_cost` 支持行级编辑，编辑后作用于该行全部数量。
3. 同商品多次扫码合并为同一行时，`unit_cost` 编辑按“统一修改”语义处理。
4. `unit_cost` 仍为可选：
   - 空值允许，提交时沿用服务端回落 `cost_price`；
   - 非空时客户端需校验金额格式（最多 4 位小数）。
5. 本约定仅涉及客户端编排与展示，不新增后端接口，不改变权限、错误码、幂等语义。

## 4.2 销售出库（核心）
- **POST** `/inventory/outbound`

**Request**
```json
{
  "customer_id": 2001,
  "expected_version": 33,
  "items": [
    {"product_id": 1001, "qty": 2, "sell_price": "3.50"},
    {"product_id": 1002, "qty": 1, "sell_price": "10.00"}
  ],
  "remark": "门店零售"
}
```

> `biz_no` 由服务端自动生成；创建成功后在响应 `data.biz_no` 返回。

### 4.2.A 出库前端流程模式化约定（本次新增）

1. 客户端默认出库模式为 `scan_confirm`：
   - 扫码命中商品后先进入确认表单，再写入明细。
2. 客户端可切换为 `continuous_scan`：
   - 打开摄像头后持续识别，命中后立即写入/累加明细，并继续扫描下一条。
3. 两种模式均复用 `POST /inventory/outbound`，请求参数与响应字段完全一致。
4. `continuous_scan` 模式下，客户端可在每次识别成功后进行页面提示与语音提示；不新增 API 字段。
5. 兼容迁移：若客户端读取到历史模式值 `quick_accumulate`，需自动映射为 `continuous_scan` 并写回本地偏好。
6. 该能力为纯前端交互增强，不新增后端接口，不新增错误码。

### 4.2.C 出库确认后续扫约定（本次新增）

1. 在 `scan_confirm` 模式中，客户端应支持“确认写入后自动续扫”，避免重复点击扫码入口。
2. 会话状态需可见（进行中/已处理条数），并支持用户手动结束。
3. 同码短时去重、成功反馈策略保持不变。
4. 本约定仅涉及前端交互层，不改变 `POST /inventory/outbound` 参数与响应结构。

### 4.2.E 出库 `scan_confirm` 弹窗化约定（本次新增）

1. Windows 与 Android 在 `scan_confirm` 模式下，命中商品后必须通过 Modal/Dialog 完成“数量/单价/版本”确认，不保留页面内联确认区。
2. 弹窗确认成功后，需自动恢复到下一次扫码等待态，延续“确认后续扫”会话语义。
3. 快捷键与交互建议：
   - Windows：`Enter` 确认、`Esc` 取消；
   - Android：支持回车快速确认（扫码枪/外接键盘场景）。
4. 弹窗化仅属于客户端交互层改造，不新增接口字段，不改变 `POST /inventory/outbound` 请求/响应结构与错误码语义。

### 4.2.F 出库明细商品名称展示与版本字段收敛约定（本次新增）

1. 出库明细展示层必须包含 `product_name`，用于现场核对。
2. 扫码命中并写入明细时，客户端需同步写入 `product_name`。
3. 手工新增明细允许补录 `product_name`，缺失时可使用兜底展示（如 `商品#<product_id>`）。
4. `expected_version` 从默认主输入收敛为非主输入（可隐藏或放入高级区），但请求组包兼容保持不变。
5. 本约定仅涉及客户端展示与组包策略，不新增后端字段，不改变权限、错误码、幂等语义。

### 4.2.B 订单明细编辑策略约定（本次新增）

1. 订单明细行默认只读，点击“编辑”后进入可编辑状态。
2. 提交前客户端可忽略全空白明细行（不组包入 `items`），以降低误触带来的校验失败。
3. 即使客户端过滤空白行，`items` 仍必须满足“至少 1 条有效明细”的原有约束。
4. 上述策略仅影响客户端 UI 交互，不改变 `POST /inventory/outbound` 契约语义。

## 4.2.1 销售单状态流转
- `DRAFT -> CONFIRMED -> RETURNED_PARTIAL / RETURNED_FULL`
- 已 `CONFIRMED` 单据不允许删除；退货需调用专用接口。

## 4.2.2 销售单作废（仅草稿）
- **POST** `/sales-orders/{id}/void`
- 仅允许 `DRAFT` 状态作废。

### 4.2.D 销售单响应小票字段增强（本次新增）

1. 销售单明细 `items[]` 新增字段：
   - `product_name`：商品名称，读取优先级为：`product_name_snapshot`（下单时固化） > 商品主数据映射名称 > `商品#<product_id>`。
   - `line_amount`：行小计，口径为 `qty * sell_price`，字符串金额，4 位小数。
2. 销售单订单级字段 `total_amount` 口径不变，定义为各明细行小计汇总，字符串金额，4 位小数。
3. 本增强仅扩展响应字段，不改变创建/确认/作废/退货请求结构、权限与错误码语义。

## 4.3 采购单状态流转与作废
- 状态：`DRAFT -> CONFIRMED -> VOIDED`
- **POST** `/purchase-orders/{id}/void`
- 已 `CONFIRMED` 作废时，服务端必须生成反向库存流水（审计可追踪）。

### 4.3.A 采购单响应小票字段增强（本次新增）

1. 采购单明细 `items[]` 新增字段：
   - `product_name`：商品名称，读取优先级为：`product_name_snapshot`（下单时固化） > 商品主数据映射名称 > `商品#<product_id>`。
   - `line_amount`：行小计，口径为 `qty * unit_cost`，字符串金额，4 位小数。
2. 采购单订单级新增字段：
   - `total_amount`：订单总金额，口径为各明细行小计汇总，字符串金额，4 位小数。
3. 本增强仅扩展响应字段，不改变创建/确认/作废请求结构、权限与错误码语义。

### 4.3.B 采购记录分页查询（只读，本次新增）
- **GET** `/purchase-orders?start_date=2026-03-12&end_date=2026-03-12&page=1&page_size=10`
- **鉴权**：是
- **权限**：`OWNER`、`PURCHASER`
- **Header**：不要求 `X-Idempotency-Key`（读接口）

**Query 参数**

| 参数 | 必填 | 说明 |
| --- | --- | --- |
| `start_date` | 否（与 `end_date` 成对） | 开始日期，格式 `YYYY-MM-DD`；不传时默认当天 |
| `end_date` | 否（与 `start_date` 成对） | 结束日期，格式 `YYYY-MM-DD`；不传时默认当天 |
| `page` | 否 | 页码，默认 `1` |
| `page_size` | 否 | 每页大小，默认 `10`，最大 `100` |

**响应结构（详细信息）**

```json
{
  "start_date": "2026-03-12",
  "end_date": "2026-03-12",
  "list": [
    {
      "id": 3101,
      "biz_no": "PO-20260312-01",
      "supplier_id": 2001,
      "status": "CONFIRMED",
      "items": [
        {
          "product_id": 1001,
          "product_name": "可口可乐 330ml",
          "qty": 12,
          "unit_cost": "2.1000",
          "line_amount": "25.2000"
        }
      ],
      "total_amount": "25.2000",
      "remark": "补货",
      "version": 2,
      "confirmed_at": "2026-03-12T09:10:00Z",
      "voided_at": null,
      "created_at": "2026-03-12T09:00:00Z",
      "updated_at": "2026-03-12T09:10:00Z"
    }
  ],
  "total": 1,
  "page": 1,
  "page_size": 10
}
```

**参数校验与错误语义**
- `start_date/end_date` 仅传其一：`4000`
- 日期格式错误：`4000`
- `start_date > end_date`：`4000`
- `page/page_size` 非法：服务端按默认与范围回落（`page>=1`，`1<=page_size<=100`）
- 权限不足：`4030`

**只读边界说明**
1. 本接口仅提供查询能力，不提供采购单修改入口。
2. 采购单变更仍通过既有状态流转接口处理（创建/确认/作废），并保留审计可追溯语义。

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

> 注：创建盘点单 `POST /inventory/stock-checks` 不再要求请求体提供 `biz_no`，由服务端自动生成。

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
      "snapshot_inbound_unit_cost": null,
      "snapshot_sell_price": "3.5000",
      "operator_id": "a3ec0000-0000-0000-0000-000000000001",
      "created_at": "2026-03-06T09:20:11Z"
    }
  ],
  "total": 1,
  "page": 1,
  "page_size": 20
}
```

**字段补充说明**
- `snapshot_inbound_unit_cost`：本次入库真实进货价格快照。
  - 对 `IN_PURCHASE` 新流水必须写入（含单条入库、批量入库、采购单确认入库）；
  - 取值口径：`effective_unit_cost = unit_cost ?? product.last_inbound_unit_cost ?? product.cost_price`；
  - 非入库类型及历史流水可为空（`null`）。
- `snapshot_sell_price`：销售成交单价快照。
  - 对 `OUT_SALE/RETURN_SALE` 新流水优先写入；
  - 历史流水可为空（`null`），用于兼容旧数据。

**参数校验与错误语义**
- `product_id <= 0`、`operator_id` 非 UUID、日期格式错误、仅传 `start_date/end_date`、`start_date > end_date`：返回 `4000`。
- 权限不足：返回 `4030`。

### 4.5.A Android 入库记录查询约定（本次新增）

1. Android 端 Dashboard 入口由“采购记录”调整为“入库记录”，查询链路复用 `GET /inventory/logs`。
2. 入库记录固定过滤 `biz_type=IN_PURCHASE`，默认参数：
   - `start_date = 当天`
   - `end_date = 当天`
   - `page = 1`
   - `page_size = 10`
3. 每页条数可由用户调整，合法范围 `1~100`；输入为空或非法时客户端回落 `10`。
4. 页面仅提供只读展示，不新增任何库存写操作 API。
5. 角色权限保持不变：仅 `OWNER/PURCHASER` 可查询，`SALES` 返回 `4030`。

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

### 5.1.A 经营看板日期筛选交互约定（本次新增）

1. Windows 端看板查询日期默认使用当天（`YYYY-MM-DD`），并在首屏自动发起当天查询。
2. 日期输入控件采用日历选择器（Date Picker，`input[type=date]`），减少手输格式错误。
3. 客户端提交时仍按 `date=YYYY-MM-DD` 传参，不新增字段、不改变接口路径与方法。
4. “重置为今天”动作会回填当天日期并重新调用 `GET /reports/dashboard`。
5. 本约定仅为前端交互层增强，不改变权限与错误码语义。

**Response.data**
```json
{
  "date": "2026-03-02",
  "total_sales": "1250.50",
  "total_gross_profit": "320.30",
  "total_orders": 45,
  "low_stock_count": 3,
  "top_selling_item": "可口可乐"
}
```

### 5.1.B 经营看板统计口径统一约定（本次新增）

1. **统计时间口径**
   - 看板全部指标统一按租户时区自然日统计（当前实现为 `UTC+8`）。

2. **销售额/毛利/热销品口径**
   - 数据源统一来自当日 `stock_logs` 中 `OUT_SALE / RETURN_SALE`。
   - 单价口径采用“快照优先、映射兜底”：
     1) 优先使用 `stock_logs.snapshot_sell_price`；
     2) 快照为空时回退 `(biz_no, product_id) -> sales_order_items.sell_price`。
   - 当快照与映射都不可用时，该条流水不计入：
     - `total_sales`
     - `total_gross_profit`
     - `top_selling_item`

3. **订单数口径**
   - `total_orders` 定义为：当日满足“存在有效销售单价（快照或映射）”的有效 `OUT_SALE` 流水中，唯一 `biz_no` 数量。
   - 覆盖 `SO-*` 与 `OUT-*` 业务单号。
   - 不再按 `sales_orders.confirmed_at` 直接计数，避免“已确认但无销售流水”的统计偏差。

4. **低库存口径**
   - `low_stock_count` 定义为：当前满足 `!is_deleted && current_stock < min_stock_limit` 的商品数量。

5. **遗留流水处理规则**
   - 无法映射到销售单明细的历史/遗留 `OUT_SALE/RETURN_SALE` 流水必须忽略，不得污染经营看板统计。

### 5.1.C 看板订单数下钻列表（本次新增）

- **GET** `/reports/dashboard/orders?start_date=2026-03-02&end_date=2026-03-02&page=1&page_size=20`
- **鉴权**：是
- **权限**：`OWNER`、`SALES`

**Query 参数**

| 参数 | 必填 | 说明 |
| --- | --- | --- |
| `start_date` | 否（与 `end_date` 成对） | 开始日期，格式 `YYYY-MM-DD`；不传时默认当天 |
| `end_date` | 否（与 `start_date` 成对） | 结束日期，格式 `YYYY-MM-DD`；不传时默认当天 |
| `page` | 否 | 页码，默认 `1` |
| `page_size` | 否 | 每页大小，默认 `20`，最大 `100` |

**Windows 看板跳转联动约定（补充）**

1. 从经营看板点击“订单数”进入下钻页时，客户端应按看板 `total_orders` 动态设置首屏 `page_size`：
   - `page_size = min(max(total_orders, 1), 100)`。
2. 跳转时默认传入：
   - `start_date = 看板日期`
   - `end_date = 看板日期`
   - `page = 1`
3. 当 `total_orders > 100` 时，首屏展示前 `100` 条，剩余数据通过既有分页查询继续查看。
4. 该联动为客户端 query 组包优化，不新增接口字段，不改变后端分页上限、权限与错误码语义。
5. 点击反馈可感知约束：
   - 下钻入口点击热区不应仅限数字文本，建议整张“订单数”指标卡可点击；
   - 对无下钻权限角色点击时，客户端必须展示明确提示（如“仅 OWNER / SALES 可查看订单下钻”），不得静默无反馈；
   - 路由跳转失败（含前端路由异常）时，客户端必须回显失败提示，避免用户感知为“点击无反应”。
6. 第 5 条属于客户端交互增强，不新增后端 API 字段，不改变 `GET /reports/dashboard/orders` 的参数、权限与错误码语义。

**Android 看板跳转联动约定（本次新增）**

1. Android 端从经营看板点击“今日订单数”指标卡进入下钻页时，默认参数必须与看板联动：
   - `start_date = 看板 date`
   - `end_date = 看板 date`
   - `page = 1`
   - `page_size = 10`（默认）
2. Android 下钻页每页条数交互约束：
   - 用户可手动修改每页条数；
   - 当每页条数输入为空或非法时，客户端回落为 `10`。
3. 角色权限约束：
   - `OWNER`、`SALES`：允许进入下钻并调用 `GET /reports/dashboard/orders`；
   - `PURCHASER`：不允许进入，客户端点击后必须即时提示无权限，不得静默无反馈。
4. 下钻页日期筛选支持用户修改查询区间并触发分页查询；请求参数与返回结构保持现有契约，不新增 API 字段。
5. 日期校验策略：
   - `start_date` / `end_date` 格式必须为 `YYYY-MM-DD`；
   - `start_date <= end_date`；
   - 校验失败时客户端必须本地提示并阻断请求发送。
6. 跳转与查询失败可感知约束：
   - 路由跳转异常必须回显失败提示；
   - 查询失败必须展示可读错误信息（建议附带 `code/request_id`）。
7. 本约定属于 Android 客户端交互增强，不改变后端权限、错误码、分页上限与统计口径语义。

### 5.1.D App 日期输入日历化兼容约定（本次新增）

1. Android App 中涉及日期输入的查询位（当前含 `GET /reports/dashboard` 的 `date`、`GET /reports/dashboard/orders` 的 `start_date/end_date`）统一由日历选择器产生值，不再依赖自由文本输入。
2. 默认值约定：
   - `date` 默认当天（`YYYY-MM-DD`）；
   - `start_date/end_date` 默认均为当天（`YYYY-MM-DD`）。
3. 客户端提交参数格式保持不变，仍按既有字符串日期格式传参：`YYYY-MM-DD`。
4. 当客户端初始化参数为空、或接口返回日期字段缺失时，客户端应回落当天日期后再发起查询，避免空日期请求。
5. 本约定仅为客户端输入来源与默认值策略调整，不新增接口字段，不改变接口路径、权限与错误码语义。

**口径说明（必须与看板订单数一致）**

1. 仅统计日期范围内满足“存在有效销售单价（快照或映射）”的有效 `OUT_SALE` 流水。
2. 按唯一 `biz_no` 聚合得到订单集合。
3. 若某 `biz_no` 存在有效 `OUT_SALE` 流水但不存在 `sales_orders` 主记录，接口仍返回该 `biz_no` 的聚合行：
   - `status=OUTBOUND_ONLY`
   - 优先返回按 `(biz_no, product_id, sell_price)` 聚合得到的 `items`
   - 仅在历史数据无法聚合时允许 `items=[]`
   - `remark="由库存流水聚合生成（无销售单主记录）"`
   - `total_amount` 由该 `biz_no` 的有效 `OUT_SALE/RETURN_SALE` 流水净额聚合得到。
4. 返回 `total` 必须与该范围内同口径订单总数一致。

**订单列表小票化展示约定（本次新增）**

1. 销售单页下钻列表建议按“每笔订单一个小票卡片”渲染，不再仅限订单摘要行表格。
2. 每个订单卡片展示字段至少包括：
   - 订单头：`biz_no`、`status`、`total_amount`；
   - 明细区（若 `items` 非空）：`product_name`、`qty`、`sell_price`、`line_amount`；
   - 订单尾：`created_at`、`updated_at`、`remark`。
3. 当订单为聚合行（`status=OUTBOUND_ONLY`）时，客户端必须仍展示订单头与订单尾信息；若 `items` 非空按普通明细渲染，若 `items=[]` 仍需保留 `remark` 且不得因无明细而整单隐藏。
4. 本约定属于客户端展示层增强，不改变 `GET /reports/dashboard/orders` 的参数、返回字段、权限与错误码语义。

**Android 端小票化展示补充约定（本次新增）**

1. Android 下钻页（`DashboardOrdersPage`）与 Windows 销售单下钻列表保持同口径字段展示：
   - 订单头：`biz_no / status / total_amount`
   - 明细区：`product_name / qty / sell_price`（Android 展示层不显示 `line_amount`）
   - 订单尾：`created_at / remark`（Android 展示层不显示 `updated_at`）
2. 当订单为聚合行时，Android 端必须保留整单卡片；若 `items` 非空则渲染商品名称/数量/单价，若 `items=[]` 则显示“无明细（聚合行）”提示。
3. 服务端返回结构保持不变，`line_amount` 与 `updated_at` 字段继续返回用于跨端兼容，Android 客户端按展示策略忽略即可。
4. 该补充仅约束客户端渲染，不新增后端字段，不改变接口参数、权限和错误码语义。

**Response.data**
```json
{
  "start_date": "2026-03-02",
  "end_date": "2026-03-02",
  "list": [
    {
      "id": 7001,
      "biz_no": "SO-TEST-001",
      "customer_id": null,
      "status": "CONFIRMED",
      "items": [
        {
          "product_id": 1001,
          "product_name": "可口可乐 330ml",
          "qty": 3,
          "sell_price": "3.5000",
          "line_amount": "10.5000",
          "returned_qty": 1
        }
      ],
      "total_amount": "10.5000",
      "remark": null,
      "version": 2,
      "confirmed_at": "2026-03-02T10:00:00Z",
      "returned_at": null,
      "voided_at": null,
      "created_at": "2026-03-02T09:58:00Z",
      "updated_at": "2026-03-02T10:00:00Z"
    },
    {
      "id": -1,
      "biz_no": "OUT-LEGACY-DRILL",
      "customer_id": null,
      "status": "OUTBOUND_ONLY",
      "items": [
        {
          "product_id": 1001,
          "product_name": "可口可乐 330ml",
          "qty": 5,
          "sell_price": "4.0000",
          "line_amount": "20.0000",
          "returned_qty": 0
        }
      ],
      "total_amount": "18.0000",
      "remark": "由库存流水聚合生成（无销售单主记录）",
      "version": 0,
      "confirmed_at": "2026-03-02T11:20:00Z",
      "returned_at": null,
      "voided_at": null,
      "created_at": "2026-03-02T11:20:00Z",
      "updated_at": "2026-03-02T11:20:00Z"
    }
  ],
  "total": 2,
  "page": 1,
  "page_size": 20
}
```

**典型错误语义**
- `4000`：日期格式错误、仅传 `start_date/end_date`、`start_date > end_date`、分页参数非法。
- `4030`：无权限（如 `PURCHASER`）。

## 5.1.E 经营趋势查询（本次新增）

- **GET** `/reports/trend?start_date=2026-03-07&end_date=2026-03-13`
- **鉴权**：是
- **权限**：`OWNER`、`SALES`
- **Header**：不要求 `X-Idempotency-Key`（读接口）

**Query 参数**

| 参数 | 必填 | 说明 |
| --- | --- | --- |
| `start_date` | 否（与 `end_date` 成对） | 开始日期，格式 `YYYY-MM-DD`；不传时默认 `今天-6天` |
| `end_date` | 否（与 `start_date` 成对） | 结束日期，格式 `YYYY-MM-DD`；不传时默认当天 |

**参数校验与错误语义**
- `start_date/end_date` 仅传其一：`4000`
- 日期格式错误：`4000`
- `start_date > end_date`：`4000`
- 日期范围超过 90 天：`4000`（可配置上限，防止慢查询）
- 权限不足：`4030`

**Response.data**
```json
{
  "start_date": "2026-03-07",
  "end_date": "2026-03-13",
  "days": [
    {
      "date": "2026-03-07",
      "total_sales": "980.00",
      "total_gross_profit": "210.50",
      "total_orders": 32,
      "low_stock_count": 2,
      "top_selling_item": "百事可乐"
    },
    {
      "date": "2026-03-08",
      "total_sales": "1250.50",
      "total_gross_profit": "320.30",
      "total_orders": 45,
      "low_stock_count": 3,
      "top_selling_item": "可口可乐"
    }
  ]
}
```

**字段说明**
- `days`：按日期升序排列的每日汇总数据数组。
- 每个 `days[*]` 元素的字段语义与 `GET /reports/dashboard` 单日看板完全一致。
- 统计口径复用看板已确立的规则（快照优先、映射兖底、无效流水忽略）。
- 若某天无任何销售流水，仍须返回该天的记录，各指标为零值（`"0.00"` / `0` / `"暂无"`）。

**后端实现建议**
1. 内部复用 `GET /reports/dashboard` 的计算逻辑，按日期范围循环汇总。
2. 可优化为单次 SQL 按日期分组聚合，减少数据库往返。
3. 日期范围上限建议 90 天，防止性能砑颈。

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

---

## 9. Android UI/UX 优化的 API 兼容约定

1. Android 端进行页面布局与视觉样式优化时，以下契约必须保持不变：
   - 接口路径、方法、参数名、字段语义；
   - 统一响应结构 `code/message/data/request_id`；
   - 角色权限语义与错误码语义（`4030/4040/4091/4092` 等）。

2. UI 层允许优化内容：
   - 同一页面内操作顺序重排（例如把高频操作前置）；
   - 输入区/结果区/提示区视觉分组；
   - 成功/失败提示文案与展示位置优化。

3. UI 层禁止变更内容：
   - 因界面调整而新增“隐式参数”或变更参数默认值含义；
   - 以客户端本地逻辑替代服务端鉴权与冲突判定；
   - 改写后端错误码并导致语义漂移。

4. Android 端页面优化后的联调验收最低要求：
   - 登录、看板、入库、出库、盘点、商品管理链路请求包字段与优化前一致；
   - 写接口继续携带 `X-Idempotency-Key`；
   - 发生 `4091` 时仍按“刷新后重试”逻辑处理。
   - 条码输入位可通过输入框尾部扫码图标触发摄像头扫码回填；该交互变更不影响接口参数与错误码语义。

5. 入库/出库启用“连续扫码会话模式”后的兼容约束：
   - 连续扫码仅改变前端采集方式（单次扫码 -> 会话连续扫码），不改变请求体结构；
   - 每次识别成功后的提示/语音反馈不进入 API 契约；
   - 同码去重、会话结束控制由客户端实现，不改变服务端幂等与错误码语义。

## 9.1 Android 高配软装 UI 升级兼容补充（本次新增）

1. 本次高配软装升级仅涉及 Flutter `presentation/theme` 层，不新增或变更任何后端 API。
2. 新增亮色/暗色主题、品牌头图、分区卡片、状态提示等视觉组件，不改变请求参数、响应字段与错误码语义。
3. 页面结构调整后，以下能力的调用路径与协议语义必须保持不变：
   - 认证：`/auth/login`、`/auth/register`；
   - 看板：`/reports/dashboard`；
   - 商品：`/products*`；
   - 库存：`/inventory/inbound`、`/inventory/outbound`、`/inventory/stock-check*`。
4. 写接口继续遵守幂等头约束：`X-Idempotency-Key` 必填；重试与冲突行为不变（`4092`）。
5. 冲突与权限错误处理语义不变：`4030`、`4040`、`4091` 的客户端处理策略与升级前一致。

## 10. 业务单号自动化 API 兼容约定（本次新增）

1. 以下创建接口请求体不再强制客户端传 `biz_no`：
   - `POST /inventory/inbound`
   - `POST /inventory/outbound`
   - `POST /purchase-orders`
   - `POST /sales-orders`
   - `POST /inventory/stock-checks`

2. 上述接口创建成功时，响应 `data.biz_no` 必须返回服务端生成单号。

3. 老客户端兼容：
   - 若仍传 `biz_no`，服务端可接收但不作为创建主依据；
   - 幂等语义不变（`X-Idempotency-Key`、`4092`）。

4. Android 端联调验收要求补充：
   - 创建页无需填写业务单号也可成功提交；
   - 提交成功后可在结果区看到服务端返回的 `biz_no`。

5. Windows 端联调验收要求补充：
   - 入库/出库/采购单/销售单/盘点单创建请求不再组包 `biz_no`；
   - 创建成功后结果区展示响应 `data.biz_no`；
   - 页面不再要求用户输入业务单号。

## 11. 扫码文案与行为一致性 API 兼容约定（本次新增）

1. 本次“扫码文案校准”仅为客户端展示层改造，不新增或修改任何 API 路径、方法、字段和错误码。
2. 客户端命名约束：
   - 仅会直接拉起摄像头的入口允许使用“扫码”动词（如“摄像头扫码”）；
   - 处理当前输入框条码（含扫码枪输入）的主动作，统一使用“按条码处理/按条码加入明细/按条码查询”等文案。
3. 连续扫码会话 `continuous_scan` 保持现有契约与字段语义不变；仅调整按钮展示文案，不影响请求组包。
4. 联调验收口径：
   - 文案调整前后，请求参数与响应结构保持一致；
   - `4030/4040/4091/4092` 处理逻辑保持一致；
   - 写接口 `X-Idempotency-Key` 约束保持一致。

## 12. 扫码模式记忆与确认后续扫兼容约定（本次新增）

1. 本次改造仅涉及客户端本地偏好存储与交互状态机，不新增/修改任何后端接口。
2. 扫码模式记忆键值应按“用户 + 终端 + 页面”隔离，避免跨账号串用。
3. `scan_confirm` 的“确认后自动续扫”不改变请求组包；每次确认仍按既有接口逐笔提交。
4. 兼容边界：
   - 无本地偏好时仍使用既有默认模式；
   - 清理会话（登出）后可清理当前用户扫码偏好；
   - 权限、错误码、幂等语义保持不变。

## 13. 扫码模式 Tab 化与智能触发兼容约定（本次新增）

1. 入库/出库页将扫码模式切换控件由下拉框改为并排 Tab（分段按钮）属于前端展示层改造，不新增或修改后端 API。
2. 模式值收敛为：`scan_confirm / continuous_scan`，请求字段、响应字段与错误码语义保持不变。
3. Android 端仅在 `scan_confirm` 模式下，当条码输入为空且点击主处理按钮时，智能拉起摄像头扫码；识别成功后继续沿用既有接口调用链路。
4. `continuous_scan` 模式保持显式会话入口，不由空输入智能触发替代。
5. 兼容迁移：若读取到历史值 `quick_accumulate`，客户端应自动映射为 `continuous_scan` 并写回本地偏好。
6. 联调验收口径：
   - 改造前后写接口仍携带 `X-Idempotency-Key`；
   - `4030/4040/4091/4092` 处理逻辑不变；
   - 不新增任何后端字段与错误码。

## 14. 入库提交按钮位置优化兼容约定（本次新增）

1. Windows 与 Android 入库页采用“两段式交互”：
   - 表单区仅负责录入与“加入明细/重置”；
   - 主提交动作“提交入库”统一放在“待提交入库明细”区域。
2. 当待提交明细为空时，“提交入库”按钮必须禁用，并给出明确引导提示（如“请先加入明细，再提交入库”）。
3. 当存在待提交明细时，允许在明细区直接触发提交，并沿用既有“单条/批量”编排规则。
4. 本次改造仅涉及客户端交互层，不新增后端接口、不修改请求字段、不改变响应结构。
5. 权限、错误码与幂等语义保持不变：
   - 写接口仍要求 `X-Idempotency-Key`；
   - `4030/4040/4091/4092` 处理逻辑与改造前一致。

## 15. 入库/出库页面表单收敛兼容约定（本次新增）

1. 入库页交互主路径收敛为：`扫码 -> 待提交明细 -> 提交入库`。
2. 入库页手工录入区默认折叠，仅作为异常兜底入口；用户手动展开后可继续“加入明细”。
3. 入库提交动作只从“待提交明细”区触发：
   - 待提交明细为空时禁止提交；
   - 不再采用“明细为空时回退单表单直提”的客户端策略。
4. 出库页单据头录入区（客户、备注、版本）默认折叠；`expected_version` 放入高级字段分组按需展开。
5. 上述改造仅为客户端展示与交互编排调整，不新增后端 API，不改变请求字段、权限模型、错误码语义与幂等语义。

## 16. Windows 视觉体系升级兼容约定（本次新增）

1. 本次升级范围限定于 Windows 客户端 `presentation` 层（样式、图标、动效、加载态与可视化增强），不新增/变更后端 API。
2. 以下视觉升级不影响接口契约：
   - 深夜蓝主色与语义功能色（入库绿、出库橙、库存不足红）；
   - 卡片圆角/阴影/留白体系升级；
   - 看板金额滚动动画、趋势 Sparkline、低库存进度条；
   - Skeleton 骨架屏替代纯文本加载态；
   - 导航双色图标与列表线性图标。
3. 任何页面视觉升级后，请求参数、响应字段与错误码语义必须保持一致，尤其包括：
   - `GET /reports/dashboard`
   - `GET /reports/dashboard/orders`
   - `GET /inventory/low-stock-alerts`
   - `GET /inventory/logs`
   - 商品/入库/出库既有写接口。
4. 写接口幂等规则保持不变：`X-Idempotency-Key` 仍为必填，`4092` 语义不变。
5. 权限与数据可见性规则保持不变：`4030`、`4040`、`4091` 等错误处理流程保持一致。
