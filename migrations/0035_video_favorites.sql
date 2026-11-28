-- When a video's YouTube video was favorited (NULL = not a favorite), and
-- whether a channel's or playlist's membership row is a favorite kept after
-- it left the source's listing.
ALTER TABLE videos ADD COLUMN favorited_at TEXT;
ALTER TABLE channel_videos ADD COLUMN kept INTEGER NOT NULL DEFAULT 0;
ALTER TABLE playlist_videos ADD COLUMN kept INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_videos_favorited_at ON videos (favorited_at);
