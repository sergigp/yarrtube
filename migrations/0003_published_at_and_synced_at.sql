ALTER TABLE videos ADD COLUMN synced_at TEXT;
UPDATE videos SET synced_at = updated_at WHERE status = 'DOWNLOADED';

ALTER TABLE video_metadata ADD COLUMN published_at TEXT NOT NULL DEFAULT '';
ALTER TABLE video_metadata ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';
UPDATE video_metadata SET published_at = premiered || 'T00:00:00+00:00';
ALTER TABLE video_metadata DROP COLUMN premiered;
ALTER TABLE video_metadata DROP COLUMN year;
