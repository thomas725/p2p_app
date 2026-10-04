-- The `attachment_json` columns added by up.sql are intentionally retained on
-- downgrade (SQLite column drops vary by version and the columns are nullable,
-- so leaving them is harmless). The files table is the reversible part.
DROP TABLE IF EXISTS files;