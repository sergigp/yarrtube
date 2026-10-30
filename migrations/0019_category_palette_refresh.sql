-- The solid palette gives way to vivid standard colors: each former palette
-- color takes the palette color at its position. Other colors are kept.
UPDATE categories SET color = CASE color
    WHEN '#9f8401' THEN '#f59e0b'
    WHEN '#bd7123' THEN '#f97316'
    WHEN '#c8645b' THEN '#ef4444'
    WHEN '#bd6396' THEN '#ec4899'
    WHEN '#9870c5' THEN '#8b5cf6'
    WHEN '#6a80d4' THEN '#3b82f6'
    WHEN '#0193bf' THEN '#0ea5e9'
    WHEN '#019b8d' THEN '#14b8a6'
    WHEN '#79922c' THEN '#22c55e'
    WHEN '#978364' THEN '#64748b'
    ELSE color
END;
