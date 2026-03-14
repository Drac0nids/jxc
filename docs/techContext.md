# 技术上下文（Tech Context）

## 1. 技术栈
- **后端**：Rust + Axum
- **桌面端**：Tauri（前端建议 Vue 3，依据架构文档）
- **移动端**：Flutter
- **数据库**：PostgreSQL
- **缓存**：Redis
- **网关**：Nginx

## 2. 关键技术约束
- 多租户隔离必须贯彻到认证与数据访问全链路。
- 金额/成本相关字段统一高精度（`DECIMAL(16,4)`）。
- 库存变更走事务，避免并发超卖。
- 通信协议以 RESTful API 为主。

## 3. 推荐工程结构（后端）
```text
/backend/src
├── api            # 路由处理层
├── domain         # 业务逻辑（库存、成本算法）
├── infrastructure # 仓储层/数据库访问
├── middleware     # 鉴权与租户上下文
└── shared         # 通用能力（错误、JWT、工具）
```

## 4. 客户端技术实践
- **Windows（Tauri）**
  - axios 统一封装请求与鉴权头注入。
  - Pinia 管理全局状态。
  - 打印通过 `invoke` 调用 Rust 侧能力。
- **Android（Flutter）**
  - `MobileScanner` 扫码。
  - `Isar` 本地缓存。
  - 写操作通过离线队列与后台同步机制保障可用性。

## 5. 安全与运维
- JWT 有效期 2 小时，Refresh Token 有效期 7 天。
- PostgreSQL 开启 WAL，支持定时全量备份到对象存储。
- 推荐 Docker 多阶段构建与 CI 自动化打包（Windows/Android）。

## 6. 当前工程基线（v1.1）
- PRD/ADD/API 文档已统一完成，当前可进入开发。
- 新增统一技术约束：
  - 金额字段 API 传输使用字符串，服务端 `rust_decimal`。
  - 写接口强制幂等键 `X-Idempotency-Key`。
  - 响应协议统一 `code/message/data/request_id`。

## 7. 待补充（实施阶段）
- 依赖版本锁定（Rust crates / Flutter packages / Node packages）。
- 环境变量清单（DB/Redis/JWT/对象存储）。
- 自动化测试与压测基线（库存并发、离线重放、幂等冲突）。