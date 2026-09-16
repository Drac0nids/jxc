-- SQLite: 业务核心表（采购/销售/盘点/流水/审计）

-- 采购单主表
CREATE TABLE IF NOT EXISTS purchase_orders (
    id INTEGER PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    biz_no TEXT NOT NULL,
    supplier_id INTEGER,
    status TEXT NOT NULL DEFAULT 'DRAFT' CHECK(status IN ('DRAFT','CONFIRMED','VOIDED')),
    remark TEXT,
    created_by TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    confirmed_at TEXT,
    voided_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_purchase_orders_tenant_biz_no ON purchase_orders (tenant_id, biz_no);
CREATE INDEX IF NOT EXISTS ix_purchase_orders_tenant_id ON purchase_orders (tenant_id);

-- 采购单明细
CREATE TABLE IF NOT EXISTS purchase_order_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id TEXT NOT NULL,
    purchase_order_id INTEGER NOT NULL REFERENCES purchase_orders(id) ON DELETE CASCADE,
    product_id INTEGER NOT NULL,
    qty INTEGER NOT NULL CHECK(qty > 0),
    unit_cost TEXT NOT NULL,
    product_name_snapshot TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS ix_po_items_order ON purchase_order_items (purchase_order_id);

-- 销售单主表
CREATE TABLE IF NOT EXISTS sales_orders (
    id INTEGER PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    biz_no TEXT NOT NULL,
    customer_id INTEGER,
    status TEXT NOT NULL DEFAULT 'DRAFT' CHECK(status IN ('DRAFT','CONFIRMED','RETURNED_PARTIAL','RETURNED_FULL','VOIDED')),
    remark TEXT,
    created_by TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    confirmed_at TEXT,
    returned_at TEXT,
    voided_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_sales_orders_tenant_biz_no ON sales_orders (tenant_id, biz_no);
CREATE INDEX IF NOT EXISTS ix_sales_orders_tenant_id ON sales_orders (tenant_id);

-- 销售单明细
CREATE TABLE IF NOT EXISTS sales_order_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id TEXT NOT NULL,
    sales_order_id INTEGER NOT NULL REFERENCES sales_orders(id) ON DELETE CASCADE,
    product_id INTEGER NOT NULL,
    qty INTEGER NOT NULL CHECK(qty > 0),
    sell_price TEXT NOT NULL,
    returned_qty INTEGER NOT NULL DEFAULT 0,
    product_name_snapshot TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS ix_so_items_order ON sales_order_items (sales_order_id);

-- 盘点单主表
CREATE TABLE IF NOT EXISTS stock_checks (
    id INTEGER PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    biz_no TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'DRAFT' CHECK(status IN ('DRAFT','COUNTING','CONFIRMED')),
    remark TEXT,
    created_by TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    counting_at TEXT,
    confirmed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_stock_checks_tenant_biz_no ON stock_checks (tenant_id, biz_no);
CREATE INDEX IF NOT EXISTS ix_stock_checks_tenant_id ON stock_checks (tenant_id);

-- 盘点单明细
CREATE TABLE IF NOT EXISTS stock_check_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id TEXT NOT NULL,
    stock_check_id INTEGER NOT NULL REFERENCES stock_checks(id) ON DELETE CASCADE,
    product_id INTEGER NOT NULL,
    book_stock INTEGER NOT NULL,
    actual_stock INTEGER,
    delta_qty INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_sc_items_order_product ON stock_check_items (stock_check_id, product_id);

-- 库存流水
CREATE TABLE IF NOT EXISTS stock_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id TEXT NOT NULL,
    product_id INTEGER NOT NULL,
    biz_type TEXT NOT NULL,
    biz_no TEXT NOT NULL,
    delta_qty INTEGER NOT NULL,
    snapshot_stock INTEGER NOT NULL,
    snapshot_cost TEXT NOT NULL,
    snapshot_sell_price TEXT,
    snapshot_inbound_unit_cost TEXT,
    operator_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS ix_stock_logs_tenant_product ON stock_logs (tenant_id, product_id);
CREATE INDEX IF NOT EXISTS ix_stock_logs_tenant_biz_no ON stock_logs (tenant_id, biz_no);

-- 审计日志
CREATE TABLE IF NOT EXISTS audit_logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id TEXT NOT NULL,
    operator_id TEXT NOT NULL,
    action TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    before_data TEXT NOT NULL DEFAULT '{}',
    after_data TEXT NOT NULL DEFAULT '{}',
    request_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS ix_audit_logs_tenant ON audit_logs (tenant_id, created_at);

INSERT OR IGNORE INTO schema_migrations(version) VALUES ('0003_business_tables');
