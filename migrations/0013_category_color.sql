-- Every category's color, a lowercase `#rrggbb` hex.
ALTER TABLE categories ADD COLUMN color TEXT NOT NULL DEFAULT '#fff1bf';
