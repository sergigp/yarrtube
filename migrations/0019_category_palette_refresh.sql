-- The solid palette gives way to vivid standard colors: each former palette
-- color takes the palette color at its position. Other colors are kept.
UPDATE categories SET color = CASE color
    WHEN '#0193bf' THEN '#0ea5e9'
    ELSE color
END;
