-- Local-only edits of sent messages (US-005). A NULL value means the row is
-- unchanged; a non-NULL value is the LOCAL edit time (Unix seconds). Nothing
-- is sent over the wire: peers keep their original copy of the message.
ALTER TABLE messages ADD COLUMN edited_at DOUBLE;
ALTER TABLE group_messages ADD COLUMN edited_at DOUBLE;
