-- Drop version column from products and orders (IF EXISTS for idempotency)

ALTER TABLE products DROP COLUMN IF EXISTS version;
ALTER TABLE purchase_orders DROP COLUMN IF EXISTS version;
ALTER TABLE sales_orders DROP COLUMN IF EXISTS version;
ALTER TABLE stock_checks DROP COLUMN IF EXISTS version;
