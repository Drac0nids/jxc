-- SQLite: 商品分类表
CREATE TABLE IF NOT EXISTS categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id TEXT NOT NULL,
    parent_id INTEGER REFERENCES categories(id) ON DELETE RESTRICT,
    name TEXT NOT NULL,
    level INTEGER NOT NULL DEFAULT 1,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS ix_categories_tenant_id ON categories (tenant_id);
CREATE INDEX IF NOT EXISTS ix_categories_parent_id ON categories (parent_id);

-- products 表中 category_id 已在 0002 建表时包含
INSERT OR IGNORE INTO schema_migrations(version) VALUES ('0010_categories');
