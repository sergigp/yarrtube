ALTER TABLE videos ADD COLUMN watched_at TEXT;
ALTER TABLE videos ADD COLUMN playback_position_seconds INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_videos_youtube_id ON videos (youtube_id);
