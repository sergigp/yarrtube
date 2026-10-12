-- A video's TV-show episode number, assigned once: a channel video's from
-- its publish time (season = publish year, episode = MMDD * 100 + same-day
-- index), a playlist video's from the playlist's counter (season 1).
ALTER TABLE videos ADD COLUMN season INTEGER;
ALTER TABLE videos ADD COLUMN episode INTEGER;
ALTER TABLE playlists ADD COLUMN last_episode INTEGER;
