-- A video's TV-show episode number, assigned once: a channel video's from
-- its publish time (season = publish year, episode = MMDD * 100 + same-day
-- index), a playlist video's from the playlist's counter (season 1).
ALTER TABLE videos ADD COLUMN season INTEGER;
ALTER TABLE videos ADD COLUMN episode INTEGER;
ALTER TABLE playlists ADD COLUMN last_episode INTEGER;
-- Videos already in a playlist are numbered in its stored order (ties by
-- insertion), and each playlist's counter starts after them.
UPDATE videos SET season = 1, episode = (
    SELECT n FROM (
        SELECT video_id, ROW_NUMBER() OVER (
            PARTITION BY playlist_id ORDER BY position, id) AS n
        FROM playlist_videos) AS numbered
    WHERE numbered.video_id = videos.id)
WHERE id IN (SELECT video_id FROM playlist_videos);
UPDATE playlists SET last_episode = (
    SELECT NULLIF(COUNT(*), 0) FROM playlist_videos WHERE playlist_id = playlists.id);
-- The Plex collections reconcile is gone (shows replace collections); drop
-- its queued runs so they don't fail with no handler registered.
DELETE FROM tasks WHERE task_type = 'reconcile_plex_collections';
