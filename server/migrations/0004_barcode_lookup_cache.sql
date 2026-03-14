-- v1.2.0：第三方条码查询落库缓存（Cache-Aside + 负缓存）

BEGIN;

CREATE TABLE IF NOT EXISTS barcode_lookup_cache (
    id BIGSERIAL PRIMARY KEY,
    tenant_id UUID NOT NULL,
    barcode VARCHAR(64) NOT NULL,
    lookup_status VARCHAR(16) NOT NULL,
    product_name VARCHAR(255) NULL,
    raw_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    expires_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT barcode_lookup_cache_status_check
        CHECK (lookup_status IN ('FOUND', 'NOT_FOUND')),
    CONSTRAINT ux_barcode_lookup_cache_tenant_barcode UNIQUE (tenant_id, barcode)
);

CREATE INDEX IF NOT EXISTS idx_barcode_lookup_cache_tenant_expire
    ON barcode_lookup_cache (tenant_id, expires_at);

INSERT INTO schema_migrations(version)
VALUES ('0004_barcode_lookup_cache')
ON CONFLICT (version) DO NOTHING;

COMMIT;
