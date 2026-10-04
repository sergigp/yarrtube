-- Every playlist is YouTube-linked since custom playlists were removed, so
-- every playlist video has a position.
CREATE TABLE playlist_videos_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    playlist_id TEXT NOT NULL,
    video_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (playlist_id, video_id)
);
INSERT INTO playlist_videos_new (id, playlist_id, video_id, position, created_at)
    SELECT id, playlist_id, video_id, position, created_at FROM playlist_videos;
DROP TABLE playlist_videos;
ALTER TABLE playlist_videos_new RENAME TO playlist_videos;
