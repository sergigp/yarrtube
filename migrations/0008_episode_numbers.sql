-- A video's TV-show episode number, assigned once from its publish time:
-- season = publish year, episode = MMDD * 100 + same-day index.
ALTER TABLE videos ADD COLUMN season INTEGER;
ALTER TABLE videos ADD COLUMN episode INTEGER;
