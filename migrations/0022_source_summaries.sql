-- Whether a channel's or playlist's videos get an AI summary (see the
-- source-summaries capability). Off for every source stored until now.
ALTER TABLE channels ADD COLUMN summaries_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE playlists ADD COLUMN summaries_enabled INTEGER NOT NULL DEFAULT 0;
