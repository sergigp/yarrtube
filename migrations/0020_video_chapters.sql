-- A video's YouTube chapters as a JSON array of {start_seconds, end_seconds,
-- title}. NULL = never checked (the next video sync schedules a chapters
-- fetch), '[]' = checked, the video has none.
ALTER TABLE videos ADD COLUMN chapters TEXT;
