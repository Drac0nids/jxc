-- SQLite: 条码查询缓存表
CREATE TABLE IF NOT EXISTS barcode_lookup_cache (
    tenant_id TEXT NOT NULL,
    barcode TEXT NOT NULL,
    lookup_status TEXT NOT NULL CHECK(lookup_status IN ('FOUND', 'NOT_FOUND')),
    product_name TEXT,
    raw_payload TEXT NOT NULL DEFAULT '{}',
    expires_at TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (tenant_id, barcode)
);

INSERT OR IGNORE INTO schema_migrations(version) VALUES ('0004_barcode_lookup_cache');
