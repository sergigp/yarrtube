-- Optimistic concurrency token of a video: every update bumps it and
-- only applies when the stored value still matches the one read. See the
-- video-write-consistency capability.
ALTER TABLE videos ADD COLUMN version INTEGER NOT NULL DEFAULT 0;
