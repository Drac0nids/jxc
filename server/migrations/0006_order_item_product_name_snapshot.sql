-- v1.2.14：订单明细增加商品名称快照（创建时固化）

BEGIN;

ALTER TABLE purchase_order_items
    ADD COLUMN IF NOT EXISTS product_name_snapshot TEXT NULL;

ALTER TABLE sales_order_items
    ADD COLUMN IF NOT EXISTS product_name_snapshot TEXT NULL;

INSERT INTO schema_migrations(version)
VALUES ('0006_order_item_product_name_snapshot')
ON CONFLICT (version) DO NOTHING;

COMMIT;