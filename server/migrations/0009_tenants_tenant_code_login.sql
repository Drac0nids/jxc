-- M9: 租户表 + 用户名租户内唯一 + 租户码登录
-- 目标：SaaS 多租户隔离，支持 tenant_code + username 登录

BEGIN;

-- ── 1. 新建租户表 ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY,
    code VARCHAR(8) UNIQUE NOT NULL,  -- 人类可读的短租户码，如 ABC123
    name VARCHAR(128) NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── 2. 为已有用户的 tenant_id 生成租户记录 ─────────────────────────────────────
-- 取 tenant_id 后 6 字符（十六进制），转大写，避免与已有 code 冲突
INSERT INTO tenants (id, code, name)
SELECT DISTINCT
    tenant_id,
    UPPER(SUBSTRING(tenant_id::text FROM 10 FOR 6)),  -- 取 UUID 第10-15位
    ''
FROM users
ON CONFLICT (id) DO NOTHING;

-- ── 3. 修改 users 唯一约束：从全局唯一改为租户内唯一 ─────────────────────────
DROP INDEX IF EXISTS ux_users_username;
CREATE UNIQUE INDEX IF NOT EXISTS ux_users_tenant_username ON users (tenant_id, username);

-- ── 4. 更新初始租户 code ────────────────────────────────────────────────────────
-- 给演示租户一个固定易读的 code
INSERT INTO tenants (id, code, name)
VALUES ('00000000-0000-0000-0000-000000000001', 'DEMO01', '演示租户')
ON CONFLICT (id) DO UPDATE SET code = 'DEMO01', name = '演示租户';

-- ── 5. 更新 ON CONFLICT 引用 ───────────────────────────────────────────────────
-- 原 0002 migration 中的 INSERT 用了 ON CONFLICT (username)，此处确保种子数据不报错
-- （已有数据不需重新插入，仅防迁移重跑报错）

INSERT INTO schema_migrations(version)
VALUES ('0009_tenants_tenant_code_login')
ON CONFLICT (version) DO NOTHING;

COMMIT;
