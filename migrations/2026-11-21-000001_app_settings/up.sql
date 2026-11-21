-- Persisted app settings (US-051). A generic key/value table so future
-- Settings toggles (theme, notifications) reuse the same storage instead of
-- adding a column per feature. Values are stored as text; `src/settings.rs`
-- owns the typed accessors and is the only place that parses them.
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT NOT NULL PRIMARY KEY,
    value TEXT NOT NULL
);
