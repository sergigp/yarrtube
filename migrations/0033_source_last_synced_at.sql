-- When each channel and playlist last finished a reconcile pass that listed
-- its videos on YouTube. NULL until the first such pass after this migration.
ALTER TABLE channels ADD COLUMN last_synced_at TEXT;
ALTER TABLE playlists ADD COLUMN last_synced_at TEXT;
