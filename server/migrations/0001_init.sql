-- M3 第三阶段（骨架）：PostgreSQL 初始化迁移占位
-- 说明：本次仅落地基础可插拔架构与初始化路径，
-- 业务表 DDL 将在后续阶段逐步补齐。

BEGIN;

CREATE TABLE IF NOT EXISTS schema_migrations (
    version VARCHAR(64) PRIMARY KEY,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO schema_migrations(version)
VALUES ('0001_init')
ON CONFLICT (version) DO NOTHING;

COMMIT;
