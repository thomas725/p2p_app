-- Add peer nicknames to peers table
-- Note: This migration may fail on newer DBs where columns already exist.
-- The ensure_columns() in db.rs handles idempotent upgrades.
ALTER TABLE peers ADD COLUMN peer_local_nickname TEXT;
ALTER TABLE peers ADD COLUMN received_nickname TEXT;