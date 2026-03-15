-- migration: 0014_suppliers
-- 供应商管理表

CREATE TABLE IF NOT EXISTS suppliers (
    id          BIGSERIAL PRIMARY KEY,
    tenant_id   UUID NOT NULL,
    name        VARCHAR(128) NOT NULL,
    phone       VARCHAR(32) NULL,
    notes       TEXT NULL,
    is_deleted  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS suppliers_tenant_idx ON suppliers (tenant_id, is_deleted);

-- 联通采购单的 supplier_id 外键
-- 用 DO $$ 避免重复执行时报错（PostgreSQL 不支持 ADD CONSTRAINT IF NOT EXISTS）
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'fk_purchase_orders_supplier'
    ) THEN
        ALTER TABLE purchase_orders
            ADD CONSTRAINT fk_purchase_orders_supplier
            FOREIGN KEY (supplier_id) REFERENCES suppliers(id) ON DELETE SET NULL;
    END IF;
END $$;
