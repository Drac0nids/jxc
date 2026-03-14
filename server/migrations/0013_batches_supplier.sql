-- migration: 0013_batches_supplier
-- 为 product_batches 表添加 supplier（供应商名称）字段
ALTER TABLE product_batches
    ADD COLUMN IF NOT EXISTS supplier VARCHAR(128) NULL;
