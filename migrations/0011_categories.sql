-- User-created categories grouping tracked sources. A name is unique
-- ignoring case.
CREATE TABLE IF NOT EXISTS categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_categories_name ON categories (name COLLATE NOCASE);
-- A category's members, one row per member, in the order they were added.
-- `member_kind` is `channel` or `playlist` (leaving room for `video`);
-- `member_id` is the channel handle or the playlist ID.
CREATE TABLE IF NOT EXISTS category_members (
    category_id TEXT NOT NULL,
    member_kind TEXT NOT NULL,
    member_id TEXT NOT NULL,
    PRIMARY KEY (category_id, member_kind, member_id)
);
CREATE INDEX IF NOT EXISTS idx_category_members_member ON category_members (member_kind, member_id);
