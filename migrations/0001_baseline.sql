CREATE TABLE IF NOT EXISTS channels (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    youtube_channel_id TEXT NOT NULL,
    quality TEXT NOT NULL,
    video_limit INTEGER NOT NULL,
    path TEXT NOT NULL,
    avatar_filename TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS playlists (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    quality TEXT NOT NULL,
    kind TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS videos (
    id TEXT PRIMARY KEY,
    youtube_id TEXT NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    quality TEXT,
    filename TEXT,
    thumbnail_filename TEXT,
    duration_seconds INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS video_metadata (
    video_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    plot TEXT NOT NULL,
    studio TEXT NOT NULL,
    director TEXT NOT NULL,
    premiered TEXT NOT NULL,
    year INTEGER NOT NULL,
    genre TEXT,
    tags TEXT NOT NULL,
    uniqueid TEXT NOT NULL,
    thumb TEXT,
    sorttitle TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS playlist_videos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    playlist_id TEXT NOT NULL,
    video_id TEXT NOT NULL,
    position INTEGER,
    created_at TEXT NOT NULL,
    UNIQUE (playlist_id, video_id)
);

CREATE TABLE IF NOT EXISTS channel_videos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    channel_id TEXT NOT NULL,
    video_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (channel_id, video_id)
);

CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    status TEXT NOT NULL,
    retries INTEGER NOT NULL DEFAULT 0,
    run_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_error TEXT
);

CREATE TABLE IF NOT EXISTS tasks_dead_letter (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    original_task_id INTEGER NOT NULL,
    task_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    retries INTEGER NOT NULL,
    last_error TEXT,
    created_at TEXT NOT NULL,
    failed_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    status TEXT NOT NULL,
    retries INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_error TEXT
);

CREATE TABLE IF NOT EXISTS domain_events_dead_letter (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    original_event_id INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    retries INTEGER NOT NULL,
    last_error TEXT,
    created_at TEXT NOT NULL,
    failed_at TEXT NOT NULL
);
