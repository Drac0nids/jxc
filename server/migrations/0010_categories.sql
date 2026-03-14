-- 0010: 商品三级分类
-- 新增 categories 表（邻接表结构），products 表增加 category_id 外键列

BEGIN;

CREATE TABLE IF NOT EXISTS categories (
    id          BIGSERIAL PRIMARY KEY,
    tenant_id   UUID NOT NULL,
    parent_id   BIGINT REFERENCES categories(id) ON DELETE RESTRICT,
    name        VARCHAR(64) NOT NULL,
    level       SMALLINT NOT NULL DEFAULT 1,    -- 1=大类 2=中类 3=小类
    sort_order  INTEGER NOT NULL DEFAULT 0,
    is_deleted  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS ix_categories_tenant_id ON categories (tenant_id);
CREATE INDEX IF NOT EXISTS ix_categories_parent_id ON categories (parent_id);

-- 同一租户下同一父节点内名称唯一（忽略大小写）
CREATE UNIQUE INDEX IF NOT EXISTS ux_categories_tenant_parent_name
    ON categories (tenant_id, COALESCE(parent_id, 0), LOWER(name))
    WHERE is_deleted = FALSE;

-- products 表增加 category_id（可空，删除分类时 SET NULL）
ALTER TABLE products ADD COLUMN IF NOT EXISTS category_id BIGINT
    REFERENCES categories(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS ix_products_category_id ON products (category_id);

INSERT INTO schema_migrations(version)
VALUES ('0010_categories')
ON CONFLICT (version) DO NOTHING;

COMMIT;
