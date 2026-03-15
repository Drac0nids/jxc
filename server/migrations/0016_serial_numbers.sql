-- products: add track_serials flag
ALTER TABLE products
    ADD COLUMN IF NOT EXISTS track_serials BOOLEAN NOT NULL DEFAULT FALSE;

-- serial numbers table
CREATE TABLE IF NOT EXISTS serial_numbers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       UUID NOT NULL,
    sn              TEXT NOT NULL,
    product_id      BIGINT NOT NULL,
    batch_id        UUID,                     -- optional, when track_batches+track_serials both on
    status          TEXT NOT NULL DEFAULT 'IN_STOCK',  -- IN_STOCK | SOLD | RETURNED
    unit_cost       NUMERIC(14,4),            -- inbound unit cost snapshot
    sell_price      NUMERIC(14,4),            -- outbound sell price snapshot
    inbound_biz_no  TEXT,
    outbound_biz_no TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_id, sn)
);

CREATE INDEX IF NOT EXISTS serial_numbers_tenant_product_idx
    ON serial_numbers (tenant_id, product_id);

CREATE INDEX IF NOT EXISTS serial_numbers_tenant_status_idx
    ON serial_numbers (tenant_id, status);
