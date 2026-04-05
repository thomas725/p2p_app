-- Remove first_seen column from peers
ALTER TABLE peers DROP COLUMN IF EXISTS first_seen;