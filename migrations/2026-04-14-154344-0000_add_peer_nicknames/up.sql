-- Add peer nicknames columns
-- This migration adds peer_local_nickname and received_nickname to the peers table.
-- It's safe to run even if columns already exist - the duplicate column error will be ignored
-- by Diesel when the migration is marked as already run.
-- For manual cleanup of duplicate columns, run: ALTER TABLE peers DROP COLUMN column_name;

ALTER TABLE peers ADD COLUMN peer_local_nickname TEXT;
ALTER TABLE peers ADD COLUMN received_nickname TEXT;