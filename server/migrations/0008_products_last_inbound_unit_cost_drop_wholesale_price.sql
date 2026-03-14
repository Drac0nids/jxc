ALTER TABLE products
ADD COLUMN IF NOT EXISTS last_inbound_unit_cost NUMERIC(18,4) NULL;

ALTER TABLE products
DROP COLUMN IF EXISTS wholesale_price;
