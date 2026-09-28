ALTER TABLE videos ADD COLUMN last_played_at TEXT;
UPDATE videos SET last_played_at = updated_at
 WHERE watched_at IS NULL AND playback_position_seconds > 0;
