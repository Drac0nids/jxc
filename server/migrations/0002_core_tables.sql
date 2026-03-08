-- M3 第三阶段第二步：核心业务表（登录/商品/幂等）
-- 目标：为 Repository(Postgres) 最小闭环提供真实持久化 DDL。

BEGIN;

-- 用户表：登录链路依赖（find_user_by_username）
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    username VARCHAR(64) NOT NULL,
    name VARCHAR(128) NOT NULL,
    role VARCHAR(32) NOT NULL,
    password_hash VARCHAR(128) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT users_role_check CHECK (role IN ('OWNER', 'PURCHASER', 'SALES'))
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_users_username ON users (username);
CREATE INDEX IF NOT EXISTS ix_users_tenant_id ON users (tenant_id);

-- 商品表：商品查询/创建/更新链路依赖
CREATE TABLE IF NOT EXISTS products (
    id BIGINT PRIMARY KEY,
    tenant_id UUID NOT NULL,
    sku VARCHAR(64) NOT NULL,
    barcode VARCHAR(64) NOT NULL,
    name VARCHAR(255) NOT NULL,
    unit VARCHAR(32) NOT NULL,
    current_stock INTEGER NOT NULL DEFAULT 0,
    cost_price NUMERIC(18,2) NOT NULL DEFAULT 0,
    retail_price NUMERIC(18,2) NOT NULL DEFAULT 0,
    wholesale_price NUMERIC(18,2) NOT NULL DEFAULT 0,
    min_stock_limit INTEGER NOT NULL DEFAULT 0,
    version INTEGER NOT NULL DEFAULT 1,
    is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS ix_products_tenant_id ON products (tenant_id);
CREATE INDEX IF NOT EXISTS ix_products_tenant_id_id ON products (tenant_id, id);
CREATE UNIQUE INDEX IF NOT EXISTS ux_products_tenant_barcode_active
    ON products (tenant_id, barcode)
    WHERE is_deleted = FALSE;
CREATE UNIQUE INDEX IF NOT EXISTS ux_products_tenant_sku_active
    ON products (tenant_id, LOWER(sku))
    WHERE is_deleted = FALSE;

-- 幂等表：写接口幂等重放/冲突校验依赖
CREATE TABLE IF NOT EXISTS idempotency_records (
    scope_key VARCHAR(256) PRIMARY KEY,
    request_payload JSONB NOT NULL,
    response_body JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 常见查询按 scope_key 主键即可命中；补充更新时间索引便于后续清理策略扩展
CREATE INDEX IF NOT EXISTS ix_idempotency_records_updated_at
    ON idempotency_records (updated_at DESC);

-- 初始租户与账号（与内存态演示账号保持一致：admin/admin123）
-- sha256("admin123") = 240be518fabd2724ddb6f04eeb1da5967448d7e831c08c8fa822809f74c720a9
INSERT INTO users (id, tenant_id, username, name, role, password_hash)
VALUES
    ('11111111-1111-1111-1111-111111111001', '00000000-0000-0000-0000-000000000001', 'admin', '系统管理员', 'OWNER', '240be518fabd2724ddb6f04eeb1da5967448d7e831c08c8fa822809f74c720a9'),
    ('11111111-1111-1111-1111-111111111002', '00000000-0000-0000-0000-000000000001', 'purchaser', '采购员', 'PURCHASER', '240be518fabd2724ddb6f04eeb1da5967448d7e831c08c8fa822809f74c720a9'),
    ('11111111-1111-1111-1111-111111111003', '00000000-0000-0000-0000-000000000001', 'sales', '销售员', 'SALES', '240be518fabd2724ddb6f04eeb1da5967448d7e831c08c8fa822809f74c720a9')
ON CONFLICT (username) DO NOTHING;

-- 示例商品（与内存态默认条码保持一致，便于联调）
INSERT INTO products (
    id, tenant_id, sku, barcode, name, unit, current_stock,
    cost_price, retail_price, wholesale_price, min_stock_limit,
    version, is_deleted
)
VALUES (
    1001, '00000000-0000-0000-0000-000000000001',
    'KO-330', '690123456789', '可口可乐 330ml', '罐', 100,
    2.10, 3.50, 3.20, 10,
    1, FALSE
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO schema_migrations(version)
VALUES ('0002_core_tables')
ON CONFLICT (version) DO NOTHING;

COMMIT;