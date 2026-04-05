-- Remove is_direct and target_peer columns from messages
ALTER TABLE messages DROP COLUMN is_direct;
ALTER TABLE messages DROP COLUMN target_peer;