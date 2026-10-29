-- How many times each episode_labels row was marked dirty. A label sync
-- saves a row only if no mark came after it read it, so a category change
-- made while a sync runs is never overwritten.
ALTER TABLE episode_labels ADD COLUMN generation INTEGER NOT NULL DEFAULT 0;
