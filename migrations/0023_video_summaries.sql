-- Where a video stands in getting its AI summary (see the video-summaries
-- capability). Every video already stored starts MISSING, so the sources
-- with summaries on backfill theirs.
ALTER TABLE videos ADD COLUMN summary_state TEXT NOT NULL DEFAULT 'MISSING';
-- A video's AI summary, at most one per video.
CREATE TABLE IF NOT EXISTS video_summaries (
    video_id TEXT PRIMARY KEY,
    text TEXT NOT NULL,
    model TEXT NOT NULL,
    created_at TEXT NOT NULL
);
