-- One status vocabulary and one retry policy for every per-video step (see
-- the video-retries capability): PENDING, RETRYING, DONE, UNAVAILABLE or
-- FAILED, with the step's consecutive failed attempts and, where it isn't
-- the last check time, when the last one failed. The download keeps its
-- own status column (PENDING, IN_PROGRESS, DOWNLOADED, RETRYING, FAILED,
-- UNAVAILABLE) and `last_errored_at` as its last failure time.
ALTER TABLE videos ADD COLUMN download_failed_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE videos ADD COLUMN download_failure TEXT;
ALTER TABLE videos ADD COLUMN thumbnail_status TEXT NOT NULL DEFAULT 'PENDING';
ALTER TABLE videos ADD COLUMN thumbnail_failed_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE videos ADD COLUMN thumbnail_last_failed_at TEXT;
ALTER TABLE videos ADD COLUMN subtitles_status TEXT NOT NULL DEFAULT 'PENDING';
ALTER TABLE videos ADD COLUMN chapters_status TEXT NOT NULL DEFAULT 'PENDING';
ALTER TABLE videos ADD COLUMN quality_status TEXT NOT NULL DEFAULT 'PENDING';
ALTER TABLE videos ADD COLUMN summary_status TEXT NOT NULL DEFAULT 'PENDING';
ALTER TABLE videos ADD COLUMN summary_failed_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE videos ADD COLUMN summary_last_failed_at TEXT;
ALTER TABLE videos ADD COLUMN categorization_status TEXT NOT NULL DEFAULT 'PENDING';
ALTER TABLE videos ADD COLUMN categorization_failed_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE videos ADD COLUMN categorization_last_failed_at TEXT;

-- Download: a retrying video has failed once; an errored one is given 7
-- more daily attempts, the first due 24 h after it last errored.
UPDATE videos
SET download_failure = COALESCE(
    (SELECT last_error FROM tasks
     WHERE task_type = 'download_video'
       AND json_extract(payload, '$.video_id') = videos.id
       AND last_error IS NOT NULL
     ORDER BY id DESC LIMIT 1),
    (SELECT last_error FROM tasks_dead_letter
     WHERE task_type = 'download_video'
       AND json_extract(payload, '$.video_id') = videos.id
     ORDER BY failed_at DESC, id DESC LIMIT 1))
WHERE status IN ('ERRORED_RETRYING', 'ERRORED');
UPDATE videos
SET status = 'RETRYING', download_failed_attempts = 1, last_errored_at = updated_at
WHERE status = 'ERRORED_RETRYING';
UPDATE videos
SET status = 'RETRYING', download_failed_attempts = 4,
    last_errored_at = COALESCE(last_errored_at, updated_at)
WHERE status = 'ERRORED';
UPDATE videos SET status = 'UNAVAILABLE' WHERE status = 'EXCLUDED';

-- Thumbnail: done once a file is recorded.
UPDATE videos SET thumbnail_status = 'DONE' WHERE thumbnail_filename IS NOT NULL;

-- Subtitles: data settled after at least 3 failed checks is failed (11
-- attempts), so the user can retry it.
UPDATE videos
SET subtitles_status = CASE
    WHEN subtitle_state = 'FETCHED' THEN 'DONE'
    WHEN subtitle_state = 'UNAVAILABLE' AND subtitles_failed_attempts >= 3 THEN 'FAILED'
    WHEN subtitle_state = 'UNAVAILABLE' THEN 'UNAVAILABLE'
    WHEN subtitles_failed_attempts > 0 THEN 'RETRYING'
    ELSE 'PENDING'
END;
UPDATE videos SET subtitles_failed_attempts = 11 WHERE subtitles_status = 'FAILED';
UPDATE videos SET subtitles_failed_attempts = 0 WHERE subtitles_status IN ('DONE', 'UNAVAILABLE');

-- Chapters and quality. The old failed attempts were never reset on
-- success, so settled data is mapped by what was found: found chapters are
-- done, none found is unavailable unless it took at least 3 failed checks,
-- and a settled quality is done (the old binary never retried it either).
UPDATE videos
SET chapters_status = CASE
    WHEN chapters_settled = 1 AND chapters <> '[]' THEN 'DONE'
    WHEN chapters_settled = 1 AND chapters_failed_attempts >= 3 THEN 'FAILED'
    WHEN chapters_settled = 1 THEN 'UNAVAILABLE'
    WHEN chapters_failed_attempts > 0 THEN 'RETRYING'
    ELSE 'PENDING'
END;
UPDATE videos SET chapters_failed_attempts = 11 WHERE chapters_status = 'FAILED';
UPDATE videos SET chapters_failed_attempts = 0 WHERE chapters_status IN ('DONE', 'UNAVAILABLE');
UPDATE videos
SET quality_status = CASE
    WHEN quality_settled = 1 THEN 'DONE'
    WHEN quality_failed_attempts > 0 THEN 'RETRYING'
    ELSE 'PENDING'
END;
UPDATE videos SET quality_failed_attempts = 0 WHERE quality_status = 'DONE';

-- Summary: an unavailable summary was given up on.
UPDATE videos
SET summary_status = CASE summary_state
    WHEN 'GENERATED' THEN 'DONE'
    WHEN 'UNAVAILABLE' THEN 'FAILED'
    ELSE 'PENDING'
END;
UPDATE videos SET summary_failed_attempts = 11 WHERE summary_status = 'FAILED';

-- Verdicts: an unavailable verdict was given up on.
UPDATE video_category_verdicts SET verdict = 'FAILED' WHERE verdict = 'UNAVAILABLE';

-- Categorization: a video with a failed verdict gave up, so it is failed
-- (11 attempts) and offers "Retry now"; every other one is pending.
UPDATE videos
SET categorization_status = 'FAILED', categorization_failed_attempts = 11,
    categorization_last_failed_at = updated_at
WHERE id IN (SELECT video_id FROM video_category_verdicts WHERE verdict = 'FAILED');

ALTER TABLE videos DROP COLUMN subtitle_state;
ALTER TABLE videos DROP COLUMN summary_state;
ALTER TABLE videos DROP COLUMN chapters_settled;
ALTER TABLE videos DROP COLUMN quality_settled;
