-- 商品批次管理（方案A：轻量批次，库存合并，批次过期预警）

CREATE TABLE product_batches (
    id           BIGSERIAL PRIMARY KEY,
    tenant_id    UUID NOT NULL,
    product_id   BIGINT NOT NULL REFERENCES products(id),
    lot_number   VARCHAR(100) NOT NULL,       -- 批次号，手填或自动（YYYYMMDD-NNN）
    inbound_at   DATE NOT NULL,               -- 入库日期
    produced_at  DATE,                        -- 生产日期（可选）
    expires_at   DATE,                        -- 过期日期（有值则参与预警）
    notes        TEXT,                        -- 备注
    is_sold_out  BOOLEAN NOT NULL DEFAULT FALSE,  -- 用户手动标记：该批次已售完
    sold_out_at  TIMESTAMPTZ,                 -- 标记售完时间
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 常用查询索引
CREATE INDEX idx_product_batches_tenant    ON product_batches (tenant_id);
CREATE INDEX idx_product_batches_product   ON product_batches (tenant_id, product_id);
CREATE INDEX idx_product_batches_expiring  ON product_batches (tenant_id, expires_at)
    WHERE is_sold_out = FALSE AND expires_at IS NOT NULL;
