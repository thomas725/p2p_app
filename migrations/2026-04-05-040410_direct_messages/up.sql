-- Add is_direct and target_peer columns to messages table
-- These columns were manually added to existing databases but need to be added for fresh databases
ALTER TABLE messages ADD COLUMN is_direct INTEGER NOT NULL DEFAULT 0;
ALTER TABLE messages ADD COLUMN target_peer TEXT;