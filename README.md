# 极速云进销存（Hyper JXC）

面向小微批发/零售商家的**多租户 SaaS 进销存系统**，提供「商品 – 库存 – 采购 – 销售 – 报表」的闭环管理。

- **Windows 端（Tauri + Vue 3）**：侧重经营与管理，覆盖建档、开单、盘点、报表、审计与打印场景。
- **Android 端（Flutter）**：侧重移动作业，覆盖摄像头扫码与扫码枪连续录入的入库、出库、盘点链路。
- **服务端（Rust + Axum）**：多租户隔离、库存一致性、单据状态机与审计追踪，支持 PostgreSQL / SQLite 两种存储后端。

## 目录

- [交付形态](#交付形态)
- [项目定位](#项目定位)
- [已实现能力](#已实现能力)
- [技术栈](#技术栈)
- [仓库结构](#仓库结构)
- [快速开始](#快速开始)
- [工程约束](#工程约束)
- [质量与测试](#质量与测试)
- [文档基线](#文档基线)
- [项目规模](#项目规模)
- [路线图](#路线图)
- [许可](#许可)

## 交付形态

同一套业务代码支撑两种交付形态，差异仅在存储实现与部署方式：

| 形态 | 适用对象 | 组成 | 数据存储 |
| --- | --- | --- | --- |
| **SaaS 版（多租户）** | 多门店、多角色的成长型商户 | 服务端与客户端分离部署 | PostgreSQL + Redis |
| **单机版（本地模式）** | 单店、单机、无运维能力的个体商户 | 单个 Windows 桌面应用：内嵌服务端（Tauri sidecar）+ 打包后的前端 | 本地 SQLite 数据文件 |

单机版安装即用：不需要服务器，也不需要单独安装数据库，可完全离线运行。构建方式见[单机版（本地模式）](#单机版本地模式)。

## 项目定位

中小商贸企业的库存管理普遍存在三类问题：库存数据分散、更新滞后；扫码/开单/盘点依赖人工，差错率高；多门店多角色协同缺少统一数据源。

本项目提供多租户 SaaS 与单机版两种交付形态，设计上强调三点：

- **准确**：库存与成本变更走数据库事务，出入库采用行级锁，成本使用移动加权平均，全部变更留不可变流水。
- **可追溯**：库存流水与审计日志记录变动数量、库存快照、成本快照、操作人与 `request_id`。
- **可用**：写接口幂等（`X-Idempotency-Key`）+ 乐观锁（`expected_version`），支持重试安全与多端并发编辑冲突检测。

## 已实现能力

### 服务端

| 领域 | 能力 |
| --- | --- |
| 认证与租户 | 注册 / 登录 / 刷新 / 登出，JWT（Access + Refresh），`tenant_id` 全链路隔离，RBAC 角色（OWNER / PURCHASER / SALES） |
| 用户管理 | 用户列表、创建用户、修改角色、删除用户 |
| 商品中心 | 商品 CRUD（软删除）、条码租户内唯一、扫码查品（本地缓存 + 可选第三方回源）、分类、批次、供应商 |
| 序列号 | SN 逐码入库 / 出库、序列号查询与历史追溯 |
| 库存作业 | 单条与批量入库、出库、库存流水查询、低库存预警 |
| 库存盘点 | 盘点单状态机 `DRAFT → COUNTING → CONFIRMED`，盘点基准库存漂移检测，差异写入 `ADJ_CHECK` 流水 |
| 采购单 | 状态机 `DRAFT → CONFIRMED → VOIDED`，确认写 `IN_PURCHASE` 流水并更新加权成本，作废写反向流水 |
| 销售单 | 状态机 `DRAFT → CONFIRMED → RETURNED_PARTIAL / RETURNED_FULL`，支持部分与全部退货（`RETURN_SALE`） |
| 报表 | 经营看板、经营趋势、销售报表（按商品聚合）、CSV / XLSX 导出（含表头样式、冻结、筛选、SUMMARY 汇总行） |
| 审计 | 审计日志查询，关键动作记录 `before_data` / `after_data` / `request_id` |
| 运维 | `openapi.yaml` 接口契约、systemd 部署脚本、备份脚本、健康检查脚本 |

### Windows 客户端

登录、经营看板（订单下钻）、商品管理（增删改查）、采购入库、销售出库、采购单 / 销售单 / 盘点单状态流、低库存预警、库存流水、审计日志、销售报表与导出。

### Android 端

注册 / 登录 / 登出与会话持久化；经营看板与订单下钻、销售趋势、利润趋势、热销榜；商品管理（商品档案、分类管理、批次与效期批次、低库存预警）；摄像头 / 扫码枪 / 手工输入三种方式兼容的入库与出库扫码（含确认后自动续扫）；采购单、库存盘点与盘点流水、入库记录查询。

## 技术栈

| 层 | 选型 |
| --- | --- |
| 服务端 | Rust（edition 2024）+ Axum 0.7 + sqlx 0.8 + PostgreSQL + Redis + JWT + `rust_decimal` |
| 数据 | PostgreSQL（SaaS 主链路）或 SQLite（单机版），各 17 个迁移脚本、启动时自动应用；Redis 用于幂等与缓存，未配置时退回内存态 |
| Windows 端 | Tauri 2 + Vue 3 + TypeScript + Pinia + axios + Vite |
| Android 端 | Flutter + dio + mobile_scanner + fl_chart + shared_preferences |

## 仓库结构

```text
jxc/
├─ server/                 # Rust + Axum 服务端
│  ├─ src/routes/          # auth / products / inventory / purchase_orders / sales_orders
│  │                       # stock_checks / serials / batches / categories / suppliers
│  │                       # reports / audit / users
│  ├─ src/repository.rs           # PostgreSQL 仓储实现
│  ├─ src/repository_sqlite.rs    # SQLite 仓储实现（单机版）
│  ├─ migrations/          # 17 个 PostgreSQL 迁移脚本
│  ├─ migrations/sqlite/   # 17 个等价的 SQLite 迁移脚本
│  ├─ openapi.yaml         # OpenAPI 3.0 接口契约
│  ├─ scripts/             # 部署、备份、健康检查、冒烟脚本
│  └─ deploy/              # systemd 服务单元
├─ client/                 # Windows 客户端（Tauri + Vue 3）
│  ├─ src/pages/           # 看板、商品、入库、出库、单据、盘点、报表、审计
│  └─ src-tauri/           # Tauri Rust 壳工程（单机版以 sidecar 内嵌服务端）
├─ app/                    # Android 端（Flutter）
│  └─ lib/src/features/    # auth / dashboard / inventory / products …
├─ scripts/                # build-local.sh：单机版一键打包
├─ docs/                   # 项目上下文、技术上下文、系统模式、进度记录
├─ 产品需求文档.md          # PRD 基线
├─ 架构设计文档.md          # 架构设计基线
└─ API接口定义文档.md       # 接口契约基线
```

## 快速开始

### 服务端

```bash
cd server
cp .env.example .env        # 按需修改 DATABASE_URL / JWT_SECRET / REDIS_URL
cargo run                   # 启动时自动应用 migrations/
```

默认监听 `http://0.0.0.0:8080`，数据库迁移随启动自动执行。默认测试账号：`admin` / `admin123`。

```bash
curl -s http://127.0.0.1:8080/health
```

完整接口清单与调用示例（含幂等、冲突码、导出、审计查询）见 [`server/README.md`](server/README.md)；接口契约见 [`server/openapi.yaml`](server/openapi.yaml)。

### Android 端

```bash
cd app
flutter pub get
flutter run --dart-define=API_BASE_URL=http://127.0.0.1:8080/api/v1
```

### Windows 客户端

```bash
cd client
npm install
npm run dev                 # 前端开发模式
npm run tauri:dev           # Tauri 桌面开发
npm run tauri:build:win:portable:dist   # Windows 便携版（含 sha256）
```

CI 工作流 `.github/workflows/client-windows-installer.yml` 在 Windows runner 上产出 NSIS 安装包与便携包。

### 单机版（本地模式）

单机版把服务端编译为 Tauri sidecar，与前端一起打成单个桌面应用：

```bash
./scripts/build-local.sh                                # 本机原生构建
TARGET=x86_64-pc-windows-gnu ./scripts/build-local.sh   # 交叉编译 Windows 版
```

产物位于 `client/src-tauri/target/<target>/release/bundle/`（Windows NSIS 安装包、macOS app 与 dmg）。

也可以让服务端单独以单机模式跑起来（调试用）：

```bash
cd server
STORAGE_BACKEND=sqlite SQLITE_PATH=jxc.db cargo run
```

首次启动会自动创建单机租户（租户码 `local`）与管理员账号，数据落在 `SQLITE_PATH` 指定的 SQLite 文件里。

> `client/src-tauri/binaries/` 存放 sidecar 产物，不入库，由构建脚本生成。

## 工程约束

跨端统一约定，改动任何一端都必须保持一致：

| 约定 | 内容 |
| --- | --- |
| 响应结构 | `code / message / data / request_id` |
| 幂等 | 写接口必须带 `X-Idempotency-Key`；重放返回首次结果并附 `X-Idempotent-Replay: true` |
| 乐观锁 | 传入 `expected_version`；版本冲突 `4091`，幂等请求体冲突 `4092`，业务冲突 `4090` |
| 金额 | API 传输用字符串（最多 4 位小数），服务端映射 `rust_decimal`，数据库 `DECIMAL(16,4)` |
| 库存字段 | `current_stock`（当前可用库存）、`unit_cost`（本次入库进价）、`cost_price`（当前加权成本） |
| 单据可逆性 | 已确认单据禁止物理删除，只能通过作废 / 退货产生反向流水 |

## 质量与测试

```bash
cd server && cargo test      # 服务端测试：123 passed
cd app && flutter analyze    # Android 静态检查
cd client && npm run typecheck && npm run build
```

服务端测试覆盖业务主链路、幂等重放与冲突、版本冲突失败后不变性、盘点口径、报表口径与权限边界；其中 Postgres 仓储层回归需要先配置 `TEST_DATABASE_URL` 或 `DATABASE_URL`，未配置时自动跳过。`scripts/` 下提供部署与健康检查脚本用于发布验证。

`.github/workflows/client-windows-installer.yml` 在 Windows runner 上构建 sidecar、NSIS 安装包与便携包（含 sha256），最近一次运行成功产出 NSIS（约 6MB）与便携包（约 19MB）。

## 文档基线

项目采用「文档先行」的开发约束（见 [`.clinerules`](.clinerules)）：任何业务代码改动前，先更新以下三份基线文档，再进入实现。

| 文档 | 作用 |
| --- | --- |
| [`产品需求文档.md`](产品需求文档.md) | 功能范围、优先级、验收标准与交互规则 |
| [`架构设计文档.md`](架构设计文档.md) | 分层架构、数据模型、事务与并发策略、状态机 |
| [`API接口定义文档.md`](API接口定义文档.md) | 接口清单、字段约束、错误码与幂等语义 |

`docs/` 目录下另有面向开发的上下文记录：`productContext.md`、`techContext.md`、`systemPatterns.md`、`activeContext.md`、`progress.md`。

## 项目规模

| 部分 | 代码量 |
| --- | --- |
| 服务端 Rust（含 PostgreSQL / SQLite 双仓储） | ~28,200 行 |
| Android Dart | ~28,000 行 |
| Windows 客户端 Vue / TypeScript / Tauri | ~11,000 行 |
| 数据库迁移 SQL（PostgreSQL + SQLite 两份） | ~890 行 |
| 需求 / 架构 / API 文档 | ~8,400 行 |

## 路线图

已完成 M1（认证、商品、采购入库、销售出库）、M2（盘点、低库存预警、看板、销售报表、审计）、M3（PostgreSQL 持久化主链路、导出、契约与运维闭环），以及单机版（SQLite 存储 + Tauri sidecar 单包分发）。

后续计划：

1. Android 端离线队列与冲突处理（`4091` / `4092` 重放语义对齐），以及 Refresh Token 自动续期。
2. 双份迁移脚本（PostgreSQL / SQLite）一致性校验，避免两种形态出现语义漂移。
3. 依赖版本锁定、环境变量清单与自动化压测基线（库存并发、离线重放、幂等冲突）。
4. CI 自动发布接入（安装包与镜像产物）与生产部署、健康检查验收流程固化。

## 许可

本项目基于 [Apache License 2.0](LICENSE) 开源。
