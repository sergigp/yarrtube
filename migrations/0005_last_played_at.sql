ALTER TABLE videos ADD COLUMN last_played_at TEXT;
-- No play time was recorded before this column existed, so part-watched
-- videos count as played at migration time: they are offered to continue
-- watching for the next week, then drop out like any other.
UPDATE videos SET last_played_at = strftime('%Y-%m-%dT%H:%M:%S+00:00', 'now')
 WHERE watched_at IS NULL AND playback_position_seconds > 0;
