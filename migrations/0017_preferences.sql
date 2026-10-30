-- The user's preferences: one row holding a JSON document. Keys missing from
-- it read as their default, so a new preference needs no migration.
CREATE TABLE preferences (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    data TEXT NOT NULL
);
