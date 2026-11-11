-- Indexes for lookups the existing UNIQUE constraints and primary keys
-- cannot serve.

-- A video's channel membership: find_by_video, delete by video, and the
-- episode-number lookups in the video repository.
CREATE INDEX IF NOT EXISTS idx_channel_videos_video ON channel_videos (video_id);

-- A video's playlist memberships: find_by_video and delete by video.
CREATE INDEX IF NOT EXISTS idx_playlist_videos_video ON playlist_videos (video_id);
