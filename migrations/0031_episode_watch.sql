-- The watch state yarrtube and the Plex episode `rating_key` last agreed on,
-- per stored video record (`videos.id`). No backfill: a missing row means
-- both agreed on unplayed, which is how the first watch sync seeds.
CREATE TABLE IF NOT EXISTS episode_watch (
    video_id TEXT PRIMARY KEY,
    rating_key TEXT NOT NULL,
    agreed_watched INTEGER NOT NULL,
    agreed_position_seconds INTEGER NOT NULL
);
