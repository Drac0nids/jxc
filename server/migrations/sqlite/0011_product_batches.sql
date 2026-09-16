-- SQLite: 商品批次表
CREATE TABLE IF NOT EXISTS product_batches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id TEXT NOT NULL,
    product_id INTEGER NOT NULL REFERENCES products(id),
    lot_number TEXT NOT NULL,
    supplier TEXT,
    inbound_at TEXT NOT NULL,
    produced_at TEXT,
    expires_at TEXT,
    notes TEXT,
    is_sold_out INTEGER NOT NULL DEFAULT 0,
    sold_out_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_product_batches_tenant ON product_batches (tenant_id);
CREATE INDEX IF NOT EXISTS idx_product_batches_product ON product_batches (tenant_id, product_id);
CREATE INDEX IF NOT EXISTS idx_product_batches_expiring ON product_batches (tenant_id, expires_at)
    WHERE is_sold_out = 0 AND expires_at IS NOT NULL;

INSERT OR IGNORE INTO schema_migrations(version) VALUES ('0011_product_batches');
