-- A video's subtitle state (MISSING / FETCHED / UNAVAILABLE) and its
-- recorded subtitle file. Every video already stored starts MISSING, so the
-- next reconcile pass backfills the library's subtitles.
ALTER TABLE videos ADD COLUMN subtitle_state TEXT NOT NULL DEFAULT 'MISSING';
ALTER TABLE videos ADD COLUMN subtitle_filename TEXT;
-- A video's plain-text transcript, at most one per video.
CREATE TABLE IF NOT EXISTS video_transcripts (
    video_id TEXT PRIMARY KEY,
    language TEXT NOT NULL,
    source TEXT NOT NULL,
    text TEXT NOT NULL,
    created_at TEXT NOT NULL
);
