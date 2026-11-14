-- The latest finished run of each Plex sync (`watch_state`, `labels`): when
-- it finished and either its error or what it saw of Plex (version, and the
-- titles of the libraries holding yarrtube's folders, as a JSON array).
CREATE TABLE IF NOT EXISTS plex_sync_runs (
    kind TEXT PRIMARY KEY,
    finished_at TEXT NOT NULL,
    error TEXT,
    version TEXT,
    libraries TEXT
);
-- How each category playlist lists its episodes. NULL for playlists created
-- before this was recorded, so the next label sync replaces them.
ALTER TABLE category_playlists ADD COLUMN watched TEXT;
ALTER TABLE category_playlists ADD COLUMN playlist_order TEXT;
