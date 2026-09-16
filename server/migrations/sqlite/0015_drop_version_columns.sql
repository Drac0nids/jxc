-- version 列已在建表时包含，无需额外操作
INSERT OR IGNORE INTO schema_migrations(version) VALUES ('0015_drop_version_columns');
