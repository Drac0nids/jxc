-- SQLite: 核心业务表（用户、商品、幂等）

-- 用户表
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    username TEXT NOT NULL,
    name TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('OWNER', 'ADMIN', 'PURCHASER', 'SALES')),
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_users_tenant_username ON users (tenant_id, username);
CREATE INDEX IF NOT EXISTS ix_users_tenant_id ON users (tenant_id);

-- 商品表
CREATE TABLE IF NOT EXISTS products (
    id INTEGER PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    sku TEXT NOT NULL,
    barcode TEXT NOT NULL,
    name TEXT NOT NULL,
    unit TEXT NOT NULL,
    current_stock INTEGER NOT NULL DEFAULT 0,
    cost_price TEXT NOT NULL DEFAULT '0',
    retail_price TEXT NOT NULL DEFAULT '0',
    last_inbound_unit_cost TEXT,
    min_stock_limit INTEGER NOT NULL DEFAULT 0,
    version INTEGER NOT NULL DEFAULT 1,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    category_id INTEGER,
    track_batches INTEGER NOT NULL DEFAULT 0,
    track_serials INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS ix_products_tenant_id ON products (tenant_id);

-- 幂等表
CREATE TABLE IF NOT EXISTS idempotency_records (
    scope_key TEXT PRIMARY KEY,
    request_payload TEXT NOT NULL,
    response_body TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 租户表
CREATE TABLE IF NOT EXISTS tenants (
    id TEXT PRIMARY KEY,
    code TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 默认租户（单机版固定）
INSERT OR IGNORE INTO tenants (id, code, name)
VALUES ('00000000-0000-0000-0000-000000000001', 'LOCAL', '本地企业');

-- 默认账号 admin/admin123
-- sha256("admin123") = 240be518fabd2724ddb6f04eeb1da5967448d7e831c08c8fa822809f74c720a9
INSERT OR IGNORE INTO users (id, tenant_id, username, name, role, password_hash)
VALUES (
    '11111111-1111-1111-1111-111111111001',
    '00000000-0000-0000-0000-000000000001',
    'admin', '系统管理员', 'OWNER',
    '240be518fabd2724ddb6f04eeb1da5967448d7e831c08c8fa822809f74c720a9'
);

INSERT OR IGNORE INTO schema_migrations(version) VALUES ('0002_core_tables');
