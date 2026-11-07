-- Whether a channel's or playlist's videos are categorized by AI (see the
-- source-categorization capability), off for every source stored until now,
-- and the description of the videos each category should hold.
ALTER TABLE channels ADD COLUMN categorization_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE playlists ADD COLUMN categorization_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE categories ADD COLUMN ai_prompt TEXT;
