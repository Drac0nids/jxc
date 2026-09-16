## 改动内容

<!-- 一句话说明这个 PR 做了什么 -->

## 关联 issue

<!-- 例如 Closes #12；没有就写“无” -->

## 验证方式

<!-- 写清楚你怎么确认改动是可用的，例如跑了哪些测试、手工验证了哪些路径 -->

- [ ] `cd server && cargo test`
- [ ] `cd app && flutter analyze && flutter test`
- [ ] `cd client && npm run typecheck && npm run build`
- [ ] Postgres 回归：`TEST_DATABASE_URL=... cargo test`（若有 Postgres 环境）

## 检查项

- [ ] 改动的三端保持一致（服务端 / Windows 客户端 / Android 端）
- [ ] 涉及业务语义时，已先更新基线文档（PRD / 架构 / API 定义）
- [ ] 迁移脚本两份（PostgreSQL / SQLite）已同步且可重复执行
- [ ] 没有提交密钥、`.env`、构建产物或本地数据文件
- [ ] 不包含与本 PR 无关的重排或格式化改动

## 破坏性变更 / 需要注意的地方

<!-- 没有就写“无” -->
