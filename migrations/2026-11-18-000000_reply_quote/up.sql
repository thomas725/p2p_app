-- Immutable reply snapshots (US-006). NULL = the row is not a reply; otherwise
-- the JSON-encoded ReplyRef captured when the message was composed. The sender
-- and preview are frozen on purpose: a local tombstone of the source row must
-- not blank the quoted header.
ALTER TABLE messages ADD COLUMN reply_to_json TEXT;
ALTER TABLE group_messages ADD COLUMN reply_to_json TEXT;
