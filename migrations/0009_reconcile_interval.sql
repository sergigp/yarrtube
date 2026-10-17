ALTER TABLE channels ADD COLUMN reconcile_interval_minutes INTEGER NOT NULL DEFAULT 180;
ALTER TABLE playlists ADD COLUMN reconcile_interval_minutes INTEGER NOT NULL DEFAULT 180;
