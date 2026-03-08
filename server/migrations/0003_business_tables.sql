-- M3 第三阶段第三步（第一部分）：业务核心表迁移
-- 目标：补齐采购/销售/盘点/流水/审计表，为后续 Repository 事务持久化改造提供 DDL 基础。

BEGIN;

-- 采购单主表
CREATE TABLE IF NOT EXISTS purchase_orders (
    id BIGINT PRIMARY KEY,
    tenant_id UUID NOT NULL,
    biz_no VARCHAR(64) NOT NULL,
    supplier_id BIGINT NULL,
    status VARCHAR(32) NOT NULL,
    remark TEXT NULL,
    created_by UUID NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    confirmed_at TIMESTAMPTZ NULL,
    voided_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT purchase_orders_status_check
        CHECK (status IN ('DRAFT', 'CONFIRMED', 'VOIDED')),
    CONSTRAINT purchase_orders_created_by_fk
        FOREIGN KEY (created_by) REFERENCES users(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_purchase_orders_tenant_biz_no
    ON purchase_orders (tenant_id, biz_no);
CREATE INDEX IF NOT EXISTS ix_purchase_orders_tenant_id_id
    ON purchase_orders (tenant_id, id);
CREATE INDEX IF NOT EXISTS ix_purchase_orders_tenant_status_updated_at
    ON purchase_orders (tenant_id, status, updated_at DESC);

-- 采购单明细
CREATE TABLE IF NOT EXISTS purchase_order_items (
    id BIGSERIAL PRIMARY KEY,
    tenant_id UUID NOT NULL,
    purchase_order_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    qty INTEGER NOT NULL,
    unit_cost NUMERIC(18,4) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT purchase_order_items_qty_check CHECK (qty > 0),
    CONSTRAINT purchase_order_items_unit_cost_check CHECK (unit_cost >= 0),
    CONSTRAINT purchase_order_items_order_fk
        FOREIGN KEY (purchase_order_id) REFERENCES purchase_orders(id) ON DELETE CASCADE,
    CONSTRAINT purchase_order_items_product_fk
        FOREIGN KEY (product_id) REFERENCES products(id)
);

CREATE INDEX IF NOT EXISTS ix_purchase_order_items_tenant_order
    ON purchase_order_items (tenant_id, purchase_order_id);
CREATE INDEX IF NOT EXISTS ix_purchase_order_items_tenant_product
    ON purchase_order_items (tenant_id, product_id);

-- 销售单主表
CREATE TABLE IF NOT EXISTS sales_orders (
    id BIGINT PRIMARY KEY,
    tenant_id UUID NOT NULL,
    biz_no VARCHAR(64) NOT NULL,
    customer_id BIGINT NULL,
    status VARCHAR(32) NOT NULL,
    remark TEXT NULL,
    created_by UUID NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    confirmed_at TIMESTAMPTZ NULL,
    returned_at TIMESTAMPTZ NULL,
    voided_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sales_orders_status_check
        CHECK (status IN ('DRAFT', 'CONFIRMED', 'RETURNED_PARTIAL', 'RETURNED_FULL', 'VOIDED')),
    CONSTRAINT sales_orders_created_by_fk
        FOREIGN KEY (created_by) REFERENCES users(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_sales_orders_tenant_biz_no
    ON sales_orders (tenant_id, biz_no);
CREATE INDEX IF NOT EXISTS ix_sales_orders_tenant_id_id
    ON sales_orders (tenant_id, id);
CREATE INDEX IF NOT EXISTS ix_sales_orders_tenant_status_updated_at
    ON sales_orders (tenant_id, status, updated_at DESC);

-- 销售单明细
CREATE TABLE IF NOT EXISTS sales_order_items (
    id BIGSERIAL PRIMARY KEY,
    tenant_id UUID NOT NULL,
    sales_order_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    qty INTEGER NOT NULL,
    sell_price NUMERIC(18,4) NOT NULL,
    returned_qty INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sales_order_items_qty_check CHECK (qty > 0),
    CONSTRAINT sales_order_items_sell_price_check CHECK (sell_price >= 0),
    CONSTRAINT sales_order_items_returned_qty_check CHECK (returned_qty >= 0 AND returned_qty <= qty),
    CONSTRAINT sales_order_items_order_fk
        FOREIGN KEY (sales_order_id) REFERENCES sales_orders(id) ON DELETE CASCADE,
    CONSTRAINT sales_order_items_product_fk
        FOREIGN KEY (product_id) REFERENCES products(id)
);

CREATE INDEX IF NOT EXISTS ix_sales_order_items_tenant_order
    ON sales_order_items (tenant_id, sales_order_id);
CREATE INDEX IF NOT EXISTS ix_sales_order_items_tenant_product
    ON sales_order_items (tenant_id, product_id);

-- 盘点单主表
CREATE TABLE IF NOT EXISTS stock_checks (
    id BIGINT PRIMARY KEY,
    tenant_id UUID NOT NULL,
    biz_no VARCHAR(64) NOT NULL,
    status VARCHAR(32) NOT NULL,
    remark TEXT NULL,
    created_by UUID NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    counting_at TIMESTAMPTZ NULL,
    confirmed_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT stock_checks_status_check
        CHECK (status IN ('DRAFT', 'COUNTING', 'CONFIRMED')),
    CONSTRAINT stock_checks_created_by_fk
        FOREIGN KEY (created_by) REFERENCES users(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_stock_checks_tenant_biz_no
    ON stock_checks (tenant_id, biz_no);
CREATE INDEX IF NOT EXISTS ix_stock_checks_tenant_id_id
    ON stock_checks (tenant_id, id);
CREATE INDEX IF NOT EXISTS ix_stock_checks_tenant_status_updated_at
    ON stock_checks (tenant_id, status, updated_at DESC);

-- 盘点单明细
CREATE TABLE IF NOT EXISTS stock_check_items (
    id BIGSERIAL PRIMARY KEY,
    tenant_id UUID NOT NULL,
    stock_check_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    book_stock INTEGER NOT NULL,
    actual_stock INTEGER NULL,
    delta_qty INTEGER NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT stock_check_items_order_fk
        FOREIGN KEY (stock_check_id) REFERENCES stock_checks(id) ON DELETE CASCADE,
    CONSTRAINT stock_check_items_product_fk
        FOREIGN KEY (product_id) REFERENCES products(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_stock_check_items_tenant_order_product
    ON stock_check_items (tenant_id, stock_check_id, product_id);
CREATE INDEX IF NOT EXISTS ix_stock_check_items_tenant_product
    ON stock_check_items (tenant_id, product_id);

-- 库存流水（不可变）
CREATE TABLE IF NOT EXISTS stock_logs (
    id BIGINT PRIMARY KEY,
    tenant_id UUID NOT NULL,
    product_id BIGINT NOT NULL,
    biz_type VARCHAR(32) NOT NULL,
    biz_no VARCHAR(64) NOT NULL,
    delta_qty INTEGER NOT NULL,
    snapshot_stock INTEGER NOT NULL,
    snapshot_cost NUMERIC(18,4) NOT NULL,
    operator_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT stock_logs_biz_type_check
        CHECK (biz_type IN ('INIT_PRODUCT', 'IN_PURCHASE', 'VOID_PURCHASE', 'OUT_SALE', 'RETURN_SALE', 'ADJ_CHECK')),
    CONSTRAINT stock_logs_product_fk
        FOREIGN KEY (product_id) REFERENCES products(id),
    CONSTRAINT stock_logs_operator_fk
        FOREIGN KEY (operator_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS ix_stock_logs_tenant_product_created_at
    ON stock_logs (tenant_id, product_id, created_at DESC);
CREATE INDEX IF NOT EXISTS ix_stock_logs_tenant_biz_no
    ON stock_logs (tenant_id, biz_no);
CREATE INDEX IF NOT EXISTS ix_stock_logs_tenant_biz_type_created_at
    ON stock_logs (tenant_id, biz_type, created_at DESC);

-- 审计日志（高风险操作留痕）
CREATE TABLE IF NOT EXISTS audit_logs (
    id BIGINT PRIMARY KEY,
    tenant_id UUID NOT NULL,
    operator_id UUID NOT NULL,
    action VARCHAR(64) NOT NULL,
    target_type VARCHAR(64) NOT NULL,
    target_id VARCHAR(64) NOT NULL,
    before_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    after_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    request_id VARCHAR(128) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT audit_logs_operator_fk
        FOREIGN KEY (operator_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS ix_audit_logs_tenant_created_at
    ON audit_logs (tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS ix_audit_logs_tenant_action_created_at
    ON audit_logs (tenant_id, action, created_at DESC);
CREATE INDEX IF NOT EXISTS ix_audit_logs_tenant_target
    ON audit_logs (tenant_id, target_type, target_id);
CREATE INDEX IF NOT EXISTS ix_audit_logs_request_id
    ON audit_logs (request_id);

INSERT INTO schema_migrations(version)
VALUES ('0003_business_tables')
ON CONFLICT (version) DO NOTHING;

COMMIT;
