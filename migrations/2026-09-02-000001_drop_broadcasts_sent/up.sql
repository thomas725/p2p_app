-- Remove the dormant, write-only `peers.broadcasts_sent` aggregate counter.
-- Broadcast-attribution is now tracked per message in `broadcast_recipients`.
-- SQLite 3.35+ supports ALTER TABLE ... DROP COLUMN (the crate already requires
-- `returning_clauses_for_sqlite_3_35`), so a fresh rebuild is safe here.
ALTER TABLE peers DROP COLUMN broadcasts_sent;