# 系统模式与架构（System Patterns）

## 1. 总体架构模式
- 采用**分层架构（Layered Architecture）**：
  1. 客户端层（Windows / Android）
  2. 网关层（Nginx + 鉴权中间件）
  3. 应用服务层（Rust + Axum）
  4. 数据持久层（PostgreSQL + Redis）
- 系统形态为**多租户 SaaS**，通过共享数据库、共享 Schema + `tenant_id` 逻辑隔离。

## 2. 关键设计模式

### 2.1 多租户隔离模式
- 在认证中间件中解析 JWT 并注入 `TenantId` 到请求上下文。
- 业务查询与写入必须基于 `tenant_id` 过滤，避免跨租户数据污染。

### 2.2 事务一致性模式（库存核心路径）
- 出入库等库存变更必须在数据库事务中执行。
- 扣减库存采用 `SELECT ... FOR UPDATE` 悲观锁，保证并发下原子性与正确性。

### 2.3 审计日志模式
- `stock_logs` 作为不可变流水，记录：业务类型、变动数量、库存快照、成本快照、操作人。
- 用于追溯库存来源、定位差异、支撑审计。

### 2.4 离线优先模式（Android）
- 读取：优先本地 `Isar`。
- 写入：本地队列缓存，后台 Worker 异步同步云端，成功后出队。

### 2.5 状态机驱动模式（单据）
- 采购单：`DRAFT -> CONFIRMED -> VOIDED`
- 销售单：`DRAFT -> CONFIRMED -> RETURNED_PARTIAL/RETURNED_FULL`
- 盘点单：`DRAFT -> COUNTING -> CONFIRMED`
- 约束：已确认单据不得物理删除，必须通过“作废/退货”产生反向业务流水。

### 2.6 冲突检测与幂等模式
- 请求携带 `X-Idempotency-Key` 保证重试安全。
- 离线写请求携带 `expected_version` 进行版本冲突检测。
- 冲突码：`4091`（版本冲突）、`4092`（幂等请求体冲突）。

### 2.7 审计增强模式
- 高风险操作记录 `before_data/after_data` 与 `request_id`。
- 审计对象覆盖：改价、改单、作废、权限变更、库存阈值修改。

## 3. 数据模型关系（核心）
- `Tenants (1) -> (N) Users`
- `Tenants (1) -> (N) Products`
- `Tenants (1) -> (N) Suppliers`
- `Tenants (1) -> (N) Customers`
- `Products (1) -> (N) StockLogs`
- `PurchaseOrders (1) -> (N) PurchaseOrderItems`
- `SalesOrders (1) -> (N) SalesOrderItems`
- `StockChecks (1) -> (N) StockCheckItems`

## 4. API 契约模式
- 统一响应结构：`code / message / data / request_id`。
- 典型流程（入库）遵循：校验 -> 事务 -> 成本计算 -> 更新库存 -> 写流水 -> 提交。

## 5. 目前确认与关键约束
- 已确认：总体架构、核心表模型、库存并发处理策略。
- 已落地统一约束：
  - 响应结构：`code/message/data/request_id`
  - 字段语义：`current_stock`、`unit_cost`、`cost_price`
  - 写接口幂等：`X-Idempotency-Key`
  - 金额传输：字符串 + `rust_decimal`