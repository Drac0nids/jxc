-- v1.2.28：库存流水增加入库真实进价快照（IN_PURCHASE 写入 effective_unit_cost）

BEGIN;

ALTER TABLE stock_logs
    ADD COLUMN IF NOT EXISTS snapshot_inbound_unit_cost NUMERIC(18,4) NULL;

INSERT INTO schema_migrations(version)
VALUES ('0007_stock_logs_snapshot_inbound_unit_cost')
ON CONFLICT (version) DO NOTHING;

COMMIT;