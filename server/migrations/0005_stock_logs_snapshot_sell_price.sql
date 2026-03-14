-- v1.2.9：库存流水增加销售单价快照（快照优先，历史映射兜底）

BEGIN;

ALTER TABLE stock_logs
    ADD COLUMN IF NOT EXISTS snapshot_sell_price NUMERIC(18,4) NULL;

INSERT INTO schema_migrations(version)
VALUES ('0005_stock_logs_snapshot_sell_price')
ON CONFLICT (version) DO NOTHING;

COMMIT;
