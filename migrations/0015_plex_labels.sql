-- The Plex label state of each stored video record (`videos.id`): whether
-- its episode's labels may be stale, and what yarrtube last wrote to which
-- episode. No backfill: the periodic label sync creates missing rows.
CREATE TABLE IF NOT EXISTS episode_labels (
    video_id TEXT PRIMARY KEY,
    dirty INTEGER NOT NULL,
    rating_key TEXT,
    section_id TEXT,
    labels TEXT NOT NULL DEFAULT '[]',
    checked_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_episode_labels_dirty ON episode_labels (dirty, checked_at);
-- The smart playlists yarrtube created, one per category (label) and Plex
-- library section.
CREATE TABLE IF NOT EXISTS category_playlists (
    section_id TEXT NOT NULL,
    label TEXT NOT NULL COLLATE NOCASE,
    rating_key TEXT NOT NULL,
    title TEXT NOT NULL,
    PRIMARY KEY (section_id, label)
);
