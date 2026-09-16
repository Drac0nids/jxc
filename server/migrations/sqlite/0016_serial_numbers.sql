-- SQLite: 序列号表
CREATE TABLE IF NOT EXISTS serial_numbers (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    sn TEXT NOT NULL,
    product_id INTEGER NOT NULL,
    batch_id TEXT,
    status TEXT NOT NULL DEFAULT 'IN_STOCK' CHECK(status IN ('IN_STOCK','SOLD','RETURNED')),
    unit_cost TEXT,
    sell_price TEXT,
    inbound_biz_no TEXT,
    outbound_biz_no TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (tenant_id, sn)
);

CREATE INDEX IF NOT EXISTS serial_numbers_tenant_product_idx ON serial_numbers (tenant_id, product_id);
CREATE INDEX IF NOT EXISTS serial_numbers_tenant_status_idx ON serial_numbers (tenant_id, status);

-- track_serials 已在 products 建表时包含
INSERT OR IGNORE INTO schema_migrations(version) VALUES ('0016_serial_numbers');
