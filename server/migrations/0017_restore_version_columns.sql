-- Restore version columns that were prematurely dropped in 0015.
-- The application code still references these columns extensively.

ALTER TABLE products        ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE purchase_orders ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE sales_orders    ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE stock_checks    ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;
