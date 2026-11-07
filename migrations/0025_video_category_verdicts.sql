-- At most one verdict per (video, category) pair: YES, NO or UNAVAILABLE,
-- decided by AI (USER: a later change). See the video-categorization
-- capability. No foreign keys: domain code deletes a video's or category's
-- verdicts with it.
CREATE TABLE IF NOT EXISTS video_category_verdicts (
    video_id    TEXT NOT NULL,
    category_id TEXT NOT NULL,
    verdict     TEXT NOT NULL,
    decided_by  TEXT NOT NULL,
    model       TEXT,
    reason      TEXT,
    decided_at  TEXT NOT NULL,
    PRIMARY KEY (video_id, category_id)
);
CREATE INDEX IF NOT EXISTS idx_video_category_verdicts_category
    ON video_category_verdicts (category_id);
