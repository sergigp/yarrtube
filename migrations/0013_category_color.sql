-- Every category's color, a lowercase `#rrggbb` hex.
ALTER TABLE categories ADD COLUMN color TEXT NOT NULL DEFAULT '#fff1bf';
-- Existing categories take the palette colors in creation order, wrapping
-- after the tenth.
UPDATE categories SET color = (
    SELECT CASE (ranked.position - 1) % 10
        WHEN 0 THEN '#fff1bf'
        WHEN 1 THEN '#ffe0c7'
        WHEN 2 THEN '#ffd9d4'
        WHEN 3 THEN '#fadceb'
        WHEN 4 THEN '#e9def8'
        WHEN 5 THEN '#dbe3fb'
        WHEN 6 THEN '#d3ecf8'
        WHEN 7 THEN '#cff0ea'
        WHEN 8 THEN '#e4f0cc'
        ELSE '#eee5d7'
    END
    FROM (
        SELECT id, ROW_NUMBER() OVER (ORDER BY created_at, rowid) AS position
        FROM categories
    ) AS ranked
    WHERE ranked.id = categories.id
);
