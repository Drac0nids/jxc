-- Migration 0012: 商品启用批次追踪开关
-- v1.4.1: 新增 track_batches 字段
-- 当 track_batches = true 时，每次采购入库成功后，
-- 服务端响应中携带该标记，客户端据此弹出批次关联弹窗。

ALTER TABLE products
    ADD COLUMN IF NOT EXISTS track_batches BOOLEAN NOT NULL DEFAULT FALSE;

COMMENT ON COLUMN products.track_batches IS '是否启用批次追踪；true 时入库响应携带该标记，客户端弹出批次关联弹窗';
