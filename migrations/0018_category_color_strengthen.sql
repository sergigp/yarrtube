-- The pastel palette gives way to solid colors: each former palette color takes
-- the palette color at its position. Other light colors are strengthened by
-- the migration's hook, which keeps their hue.
UPDATE categories SET color = CASE color
    WHEN '#fff1bf' THEN '#9f8401'
    WHEN '#ffe0c7' THEN '#bd7123'
    WHEN '#ffd9d4' THEN '#c8645b'
    WHEN '#fadceb' THEN '#bd6396'
    WHEN '#e9def8' THEN '#9870c5'
    WHEN '#dbe3fb' THEN '#6a80d4'
    WHEN '#d3ecf8' THEN '#0193bf'
    WHEN '#cff0ea' THEN '#019b8d'
    WHEN '#e4f0cc' THEN '#79922c'
    WHEN '#eee5d7' THEN '#978364'
    ELSE color
END;
