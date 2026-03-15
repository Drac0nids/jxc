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

-- 联通采购单的 supplier_id 外键（之前是裸 BIGINT NULL，现正式关联）
ALTER TABLE purchase_orders
    ADD CONSTRAINT IF NOT EXISTS fk_purchase_orders_supplier
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id) ON DELETE SET NULL;
