-- Undo the group-chat tables (groups, group_members, group_messages).
DROP INDEX IF EXISTS group_messages_created_at;
DROP INDEX IF EXISTS group_messages_group_msg;
DROP TABLE IF EXISTS group_messages;
DROP INDEX IF EXISTS group_members_group_peer;
DROP TABLE IF EXISTS group_members;
DROP TABLE IF EXISTS groups;