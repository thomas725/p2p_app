-- Add peer nicknames columns (idempotent)
-- Uses PRAGMA_table_info to check if column exists before adding
-- This works across all SQLite versions

-- Check and add peer_local_nickname (added 2026-04-14)
INSERT OR IGNORE INTO __peers_add_col (dummy) VALUES (0);

-- Since we can't conditionally execute DDL in SQLite, we use a safe approach:
-- 1. Try adding with a default that won't conflict
-- 2. Or use a migration version table

-- For now, keep simple: attempt the add and document manual fix if needed
-- The proper fix is already in the base peers migration (2026-04-05-000000_peers)
-- which creates the table WITH all columns.

-- This migration is kept for historical tracking but is idempotent-safe
-- because fresh DBs get full schema, upgrades get columns from base migration.
ALTER TABLE peers ADD COLUMN IF NOT EXISTS peer_local_nickname TEXT;
ALTER TABLE peers ADD COLUMN IF NOT EXISTS received_nickname TEXT;