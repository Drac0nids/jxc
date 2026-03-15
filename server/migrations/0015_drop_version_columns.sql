-- Drop version column from products and orders

ALTER TABLE products DROP COLUMN version;
ALTER TABLE purchase_orders DROP COLUMN version;
ALTER TABLE sales_orders DROP COLUMN version;
ALTER TABLE stock_checks DROP COLUMN version;
