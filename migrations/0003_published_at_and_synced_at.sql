ALTER TABLE videos ADD COLUMN synced_at TEXT;

ALTER TABLE video_metadata ADD COLUMN published_at TEXT NOT NULL DEFAULT '';
ALTER TABLE video_metadata ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';
ALTER TABLE video_metadata DROP COLUMN premiered;
ALTER TABLE video_metadata DROP COLUMN year;
