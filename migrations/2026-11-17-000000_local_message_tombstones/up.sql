-- Local-only tombstones for sent messages (US-005). A NULL value means the
-- row is visible; a non-NULL value is the LOCAL deletion time (Unix seconds).
-- Nothing is sent over the wire: peers keep their own copy of the row.
ALTER TABLE messages ADD COLUMN deleted_at DOUBLE;
ALTER TABLE group_messages ADD COLUMN deleted_at DOUBLE;
