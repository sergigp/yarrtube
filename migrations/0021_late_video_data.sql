-- One policy for late YouTube data (see the late-video-data capability):
-- every piece of it records when it was last checked and how many checks
-- failed; chapters and quality also record whether they are settled.
ALTER TABLE videos RENAME COLUMN subtitles_first_missing_at TO subtitles_last_checked_at;
ALTER TABLE videos ADD COLUMN subtitles_failed_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE videos ADD COLUMN chapters_last_checked_at TEXT;
ALTER TABLE videos ADD COLUMN chapters_failed_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE videos ADD COLUMN chapters_settled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE videos ADD COLUMN downloaded_height INTEGER;
ALTER TABLE videos ADD COLUMN quality_last_checked_at TEXT;
ALTER TABLE videos ADD COLUMN quality_failed_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE videos ADD COLUMN quality_settled INTEGER NOT NULL DEFAULT 0;
