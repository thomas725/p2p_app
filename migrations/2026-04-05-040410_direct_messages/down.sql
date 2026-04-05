-- Remove is_direct and target_peer columns from messages
-- Note: SQLite doesn't support DROP COLUMN in some versions, so this may fail
-- In that case, the migration is not reversible
ALTER TABLE messages DROP COLUMN IF EXISTS is_direct;
ALTER TABLE messages DROP COLUMN IF EXISTS target_peer;