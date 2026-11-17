-- The YouTube ids of Discarded videos a reconcile removed from their source
-- (a channel or playlist), so a later reconcile never adds them back.
-- Deleted with the source.
CREATE TABLE IF NOT EXISTS discarded_videos (
    source_kind TEXT NOT NULL,
    source_id TEXT NOT NULL,
    youtube_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (source_kind, source_id, youtube_id)
);
