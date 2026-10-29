-- The categories of each stored video record (`videos.id`, one per playlist
-- or channel copy), one row per category it belongs to.
CREATE TABLE IF NOT EXISTS video_categories (
    video_id TEXT NOT NULL,
    category_id TEXT NOT NULL,
    PRIMARY KEY (video_id, category_id)
);
CREATE INDEX IF NOT EXISTS idx_video_categories_category ON video_categories (category_id);
-- Existing records take the categories their channel or playlist is a member of.
INSERT OR IGNORE INTO video_categories (video_id, category_id)
SELECT channel_videos.video_id, category_members.category_id
FROM category_members
JOIN channel_videos ON channel_videos.channel_id = category_members.member_id
WHERE category_members.member_kind = 'channel';
INSERT OR IGNORE INTO video_categories (video_id, category_id)
SELECT playlist_videos.video_id, category_members.category_id
FROM category_members
JOIN playlist_videos ON playlist_videos.playlist_id = category_members.member_id
WHERE category_members.member_kind = 'playlist';
