-- SQLite: 补充 stock_logs 快照字段（已在 0003 建表时一并包含，此处仅记录版本）
INSERT OR IGNORE INTO schema_migrations(version) VALUES ('0005_stock_logs_snapshot_sell_price');
