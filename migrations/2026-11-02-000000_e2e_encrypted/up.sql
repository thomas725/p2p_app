-- End-to-end encrypted DM bodies (US-042). The receiver stores the opened
-- plaintext (this device is the intended reader) but remembers that the body
-- arrived sealed so both frontends can mark the message. Old rows and every
-- broadcast/group row default to 0 (not encrypted).
ALTER TABLE messages ADD COLUMN encrypted INTEGER NOT NULL DEFAULT 0;
