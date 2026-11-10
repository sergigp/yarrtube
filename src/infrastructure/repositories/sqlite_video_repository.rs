use crate::domain::shared::Quality;
use crate::domain::video::{
    ChannelEpisodeNumber, EpisodeNumber, LateDataChecks, PlaybackPosition, RetryAttempts,
    StaleVideo, StepState, StepStatus, Video, VideoChapter, VideoStatus, WatchState,
};
use crate::domain::video::{VideoId, YoutubeVideoId};
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params, params_from_iter};
use std::collections::HashMap;

const VIDEO_COLUMNS: &str = "id, youtube_id, title, status, quality, filename, thumbnail_filename, duration_seconds, created_at, updated_at, watched_at, playback_position_seconds, synced_at, last_errored_at, last_played_at, season, episode, subtitles_status, subtitle_filename, subtitles_last_checked_at, subtitles_failed_attempts, chapters, chapters_last_checked_at, chapters_failed_attempts, chapters_status, downloaded_height, quality_last_checked_at, quality_failed_attempts, quality_status, summary_status, version, download_failed_attempts, download_failure, thumbnail_status, thumbnail_failed_attempts, thumbnail_last_failed_at, summary_failed_attempts, summary_last_failed_at, categorization_status, categorization_failed_attempts, categorization_last_failed_at";

/// A `videos` row as read, before its values are parsed into a `Video`.
struct VideoRow {
    id: String,
    youtube_id: String,
    title: String,
    status: String,
    quality: Option<String>,
    filename: Option<String>,
    thumbnail_filename: Option<String>,
    duration_seconds: Option<i64>,
    created_at: String,
    updated_at: String,
    watched_at: Option<String>,
    playback_position_seconds: i64,
    synced_at: Option<String>,
    last_errored_at: Option<String>,
    last_played_at: Option<String>,
    season: Option<i32>,
    episode: Option<u32>,
    subtitles_status: String,
    subtitle_filename: Option<String>,
    subtitles_last_checked_at: Option<String>,
    subtitles_failed_attempts: u32,
    chapters: Option<String>,
    chapters_last_checked_at: Option<String>,
    chapters_failed_attempts: u32,
    chapters_status: String,
    downloaded_height: Option<u32>,
    quality_last_checked_at: Option<String>,
    quality_failed_attempts: u32,
    quality_status: String,
    summary_status: String,
    version: u32,
    download_failed_attempts: u32,
    download_failure: Option<String>,
    thumbnail_status: String,
    thumbnail_failed_attempts: u32,
    thumbnail_last_failed_at: Option<String>,
    summary_failed_attempts: u32,
    summary_last_failed_at: Option<String>,
    categorization_status: String,
    categorization_failed_attempts: u32,
    categorization_last_failed_at: Option<String>,
}

pub trait VideoRepository: Send + Sync {
    /// Inserts a new video; fails if a video with its id is already stored.
    fn insert(&self, video: &Video) -> anyhow::Result<()>;
    fn find(&self, id: &VideoId) -> anyhow::Result<Option<Video>>;
    /// The stored videos among `ids`, in the order given, skipping ids with
    /// no stored video. One query however many ids.
    fn find_many(&self, ids: &[VideoId]) -> anyhow::Result<Vec<Video>>;
    /// Writes every mutable column and bumps `version`, only if the stored
    /// version is still `video.version`; otherwise fails with `StaleVideo`.
    /// A video no longer stored is left absent.
    fn update(&self, video: &Video) -> anyhow::Result<()>;
    /// Puts every Failed summary back to Pending with no failed attempts,
    /// bumping each changed video's `version`.
    fn reset_failed_summaries(&self, now: DateTime<Utc>) -> anyhow::Result<()>;
    fn delete(&self, id: &VideoId) -> anyhow::Result<()>;
    /// Every stored copy of a YouTube video, across all playlists/channels.
    fn find_by_youtube_id(&self, youtube_id: &YoutubeVideoId) -> anyhow::Result<Vec<Video>>;
    /// The video's episode number: the one already recorded, if any, else the
    /// next free same-day index among the videos of the same channel
    /// published on `published_at`'s day, recorded in one write. Fails for an
    /// unnumbered video in no channel: a playlist video is numbered when it is
    /// added to its playlist, never by publish date. Recording a number bumps
    /// the video's `version`.
    ///
    /// Reading the day's last episode and recording the next one happen under
    /// the one write connection's lock, deliberately not as `find` + compute +
    /// `update` in the domain: a download and a thumbnail fetch of two
    /// same-day videos run concurrently, and would otherwise both read the
    /// same last episode and record a permanent duplicate. The numbering rule
    /// itself (first of day, next after the last) is
    /// `ChannelEpisodeNumber::next_of_day`'s.
    fn assign_episode_number(
        &self,
        id: &VideoId,
        published_at: DateTime<Utc>,
    ) -> anyhow::Result<EpisodeNumber>;
    /// Every `Downloaded` video, across all playlists and channels.
    fn list_downloaded(&self) -> anyhow::Result<Vec<Video>>;
    /// Up to `limit` `Downloaded` videos whose title contains `text`, ignoring
    /// ASCII case, newest sync first.
    fn find_downloaded_by_title(&self, text: &str, limit: usize) -> anyhow::Result<Vec<Video>>;
    /// The number `assign_episode_number` would return right now, without
    /// recording it.
    fn next_episode_number(
        &self,
        id: &VideoId,
        published_at: DateTime<Utc>,
    ) -> anyhow::Result<EpisodeNumber>;
}

pub struct SqliteVideoRepository {
    db: Database,
}

impl SqliteVideoRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Every stored video, for tests asserting the table's whole final state;
    /// the port itself has no use case that lists all videos.
    #[cfg(test)]
    pub fn list(&self) -> anyhow::Result<Vec<Video>> {
        let ids: Vec<String> = {
            let conn = self.db.read().unwrap();
            let mut stmt = conn.prepare("SELECT id FROM videos ORDER BY rowid ASC")?;
            stmt.query_map([], |row| row.get(0))?
                .collect::<Result<_, _>>()?
        };
        ids.into_iter()
            .map(|id| Ok(self.find(&VideoId::new(id)?)?.unwrap()))
            .collect()
    }
}

impl VideoRepository for SqliteVideoRepository {
    fn insert(&self, video: &Video) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            &format!(
                "INSERT INTO videos ({VIDEO_COLUMNS}) VALUES ({})",
                (1..=41)
                    .map(|i| format!("?{i}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            params![
                video.id.as_str(),
                video.youtube_id.as_str(),
                video.title,
                video.status.as_str(),
                video.quality.map(|q| q.as_str()),
                video.filename,
                video.thumbnail_filename,
                video.duration_seconds,
                video.created_at.to_rfc3339(),
                video.updated_at.to_rfc3339(),
                video.watch.watched_at.map(|w| w.to_rfc3339()),
                video.watch.position.seconds(),
                video.synced_at.map(|s| s.to_rfc3339()),
                Self::encode_timestamp(video.download_attempts.last_failed_at),
                video.watch.last_played_at.map(|p| p.to_rfc3339()),
                video.episode_number.map(|n| n.season()),
                video.episode_number.map(|n| n.episode()),
                video.subtitles.state.status.as_str(),
                video.subtitle_filename,
                Self::encode_timestamp(video.subtitles.last_checked_at),
                video.subtitles.state.attempts.failed,
                Self::encode_chapters(&video.chapters)?,
                Self::encode_timestamp(video.chapters_checks.last_checked_at),
                video.chapters_checks.state.attempts.failed,
                video.chapters_checks.state.status.as_str(),
                video.downloaded_height,
                Self::encode_timestamp(video.quality_checks.last_checked_at),
                video.quality_checks.state.attempts.failed,
                video.quality_checks.state.status.as_str(),
                video.summary.status.as_str(),
                video.version,
                video.download_attempts.failed,
                video.download_failure,
                video.thumbnail.status.as_str(),
                video.thumbnail.attempts.failed,
                Self::encode_timestamp(video.thumbnail.attempts.last_failed_at),
                video.summary.attempts.failed,
                Self::encode_timestamp(video.summary.attempts.last_failed_at),
                video.categorization.status.as_str(),
                video.categorization.attempts.failed,
                Self::encode_timestamp(video.categorization.attempts.last_failed_at),
            ],
        )
        .inspect_err(
            |e| tracing::error!(video_id = %video.id, error = %e, "failed to insert video"),
        )
        .context("failed to insert video")?;
        Ok(())
    }

    fn find(&self, id: &VideoId) -> anyhow::Result<Option<Video>> {
        let conn = self.db.read()?;
        conn.query_row(
            &format!("SELECT {VIDEO_COLUMNS} FROM videos WHERE id = ?1"),
            params![id.as_str()],
            Self::read_row,
        )
        .optional()
        .inspect_err(|e| tracing::error!(video_id = %id, error = %e, "failed to find video"))
        .context("failed to find video")?
        .map(Self::row_to_video)
        .transpose()
    }

    fn find_many(&self, ids: &[VideoId]) -> anyhow::Result<Vec<Video>> {
        let conn = self.db.read()?;
        let placeholders = vec!["?"; ids.len()].join(", ");
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {VIDEO_COLUMNS} FROM videos WHERE id IN ({placeholders})"
            ))
            .context("failed to prepare find many videos")?;
        let mut found: HashMap<String, VideoRow> = stmt
            .query_map(
                params_from_iter(ids.iter().map(|id| id.as_str())),
                Self::read_row,
            )
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .inspect_err(|e| tracing::error!(error = %e, "failed to find many videos"))
            .context("failed to find many videos")?
            .into_iter()
            .map(|row| (row.id.clone(), row))
            .collect();
        ids.iter()
            .filter_map(|id| found.remove(id.as_str()))
            .map(Self::row_to_video)
            .collect()
    }

    fn update(&self, video: &Video) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        let updated = conn.execute(
            "UPDATE videos SET youtube_id = ?2, title = ?3, status = ?4, quality = ?5, filename = ?6, thumbnail_filename = ?7, duration_seconds = ?8, updated_at = ?9, watched_at = ?10, playback_position_seconds = ?11, synced_at = ?12, last_errored_at = ?13, last_played_at = ?14, season = ?15, episode = ?16, subtitles_status = ?17, subtitle_filename = ?18, subtitles_last_checked_at = ?19, subtitles_failed_attempts = ?20, chapters = ?21, chapters_last_checked_at = ?22, chapters_failed_attempts = ?23, chapters_status = ?24, downloaded_height = ?25, quality_last_checked_at = ?26, quality_failed_attempts = ?27, quality_status = ?28, summary_status = ?29, download_failed_attempts = ?30, download_failure = ?31, thumbnail_status = ?32, thumbnail_failed_attempts = ?33, thumbnail_last_failed_at = ?34, summary_failed_attempts = ?35, summary_last_failed_at = ?36, categorization_status = ?37, categorization_failed_attempts = ?38, categorization_last_failed_at = ?39, version = version + 1
             WHERE id = ?1 AND version = ?40",
            params![
                video.id.as_str(),
                video.youtube_id.as_str(),
                video.title,
                video.status.as_str(),
                video.quality.map(|q| q.as_str()),
                video.filename,
                video.thumbnail_filename,
                video.duration_seconds,
                video.updated_at.to_rfc3339(),
                video.watch.watched_at.map(|w| w.to_rfc3339()),
                video.watch.position.seconds(),
                video.synced_at.map(|s| s.to_rfc3339()),
                Self::encode_timestamp(video.download_attempts.last_failed_at),
                video.watch.last_played_at.map(|p| p.to_rfc3339()),
                video.episode_number.map(|n| n.season()),
                video.episode_number.map(|n| n.episode()),
                video.subtitles.state.status.as_str(),
                video.subtitle_filename,
                Self::encode_timestamp(video.subtitles.last_checked_at),
                video.subtitles.state.attempts.failed,
                Self::encode_chapters(&video.chapters)?,
                Self::encode_timestamp(video.chapters_checks.last_checked_at),
                video.chapters_checks.state.attempts.failed,
                video.chapters_checks.state.status.as_str(),
                video.downloaded_height,
                Self::encode_timestamp(video.quality_checks.last_checked_at),
                video.quality_checks.state.attempts.failed,
                video.quality_checks.state.status.as_str(),
                video.summary.status.as_str(),
                video.download_attempts.failed,
                video.download_failure,
                video.thumbnail.status.as_str(),
                video.thumbnail.attempts.failed,
                Self::encode_timestamp(video.thumbnail.attempts.last_failed_at),
                video.summary.attempts.failed,
                Self::encode_timestamp(video.summary.attempts.last_failed_at),
                video.categorization.status.as_str(),
                video.categorization.attempts.failed,
                Self::encode_timestamp(video.categorization.attempts.last_failed_at),
                video.version,
            ],
        )
        .inspect_err(|e| {
            tracing::error!(video_id = %video.id, error = %e, "failed to update video")
        })
        .context("failed to update video")?;
        if updated == 0 && Self::exists(&conn, &video.id)? {
            return Err(StaleVideo(video.id.clone()).into());
        }
        Ok(())
    }

    fn reset_failed_summaries(&self, now: DateTime<Utc>) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            "UPDATE videos SET summary_status = ?1, summary_failed_attempts = 0, summary_last_failed_at = NULL, updated_at = ?2, version = version + 1 WHERE summary_status = ?3",
            params![
                StepStatus::Pending.as_str(),
                now.to_rfc3339(),
                StepStatus::Failed.as_str(),
            ],
        )
        .inspect_err(|e| tracing::error!(error = %e, "failed to reset failed summaries"))?;
        Ok(())
    }

    fn delete(&self, id: &VideoId) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute("DELETE FROM videos WHERE id = ?1", params![id.as_str()])
            .inspect_err(|e| tracing::error!(video_id = %id, error = %e, "failed to delete video"))
            .context("failed to delete video")?;
        Ok(())
    }

    fn find_by_youtube_id(&self, youtube_id: &YoutubeVideoId) -> anyhow::Result<Vec<Video>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {VIDEO_COLUMNS} FROM videos WHERE youtube_id = ?1 ORDER BY rowid ASC"
            ))
            .context("failed to prepare find videos by youtube id")?;
        stmt.query_map(params![youtube_id.as_str()], Self::read_row)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .inspect_err(|e| {
                tracing::error!(youtube_id = %youtube_id, error = %e, "failed to find videos by youtube id")
            })
            .context("failed to find videos by youtube id")?
            .into_iter()
            .map(Self::row_to_video)
            .collect()
    }

    fn assign_episode_number(
        &self,
        id: &VideoId,
        published_at: DateTime<Utc>,
    ) -> anyhow::Result<EpisodeNumber> {
        let conn = self.db.write()?;
        if let Some(recorded) = Self::recorded_episode_number(&conn, id)? {
            return Ok(recorded);
        }
        let number = Self::next_free_episode_number(&conn, id, published_at)?;
        conn.execute(
            "UPDATE videos SET season = ?2, episode = ?3, version = version + 1 WHERE id = ?1",
            params![id.as_str(), number.season(), number.episode()],
        )
        .inspect_err(
            |e| tracing::error!(video_id = %id, error = %e, "failed to record the episode number"),
        )
        .context("failed to record the episode number")?;
        Ok(number)
    }

    fn next_episode_number(
        &self,
        id: &VideoId,
        published_at: DateTime<Utc>,
    ) -> anyhow::Result<EpisodeNumber> {
        let conn = self.db.read()?;
        match Self::recorded_episode_number(&conn, id)? {
            Some(recorded) => Ok(recorded),
            None => Self::next_free_episode_number(&conn, id, published_at),
        }
    }

    fn list_downloaded(&self) -> anyhow::Result<Vec<Video>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {VIDEO_COLUMNS} FROM videos WHERE status = ?1 ORDER BY rowid ASC"
            ))
            .context("failed to prepare list downloaded videos")?;
        stmt.query_map(params![VideoStatus::Downloaded.as_str()], Self::read_row)
            .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
            .inspect_err(|e| tracing::error!(error = %e, "failed to list downloaded videos"))
            .context("failed to list downloaded videos")?
            .into_iter()
            .map(Self::row_to_video)
            .collect()
    }

    fn find_downloaded_by_title(&self, text: &str, limit: usize) -> anyhow::Result<Vec<Video>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {VIDEO_COLUMNS} FROM videos \
                 WHERE status = ?1 AND title LIKE '%' || ?2 || '%' ESCAPE '\\' \
                 ORDER BY created_at DESC, rowid DESC LIMIT ?3"
            ))
            .context("failed to prepare find downloaded videos by title")?;
        stmt.query_map(
            params![
                VideoStatus::Downloaded.as_str(),
                Self::escape_like(text),
                i64::try_from(limit).unwrap_or(i64::MAX)
            ],
            Self::read_row,
        )
        .and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
        .inspect_err(|e| tracing::error!(error = %e, "failed to find downloaded videos by title"))
        .context("failed to find downloaded videos by title")?
        .into_iter()
        .map(Self::row_to_video)
        .collect()
    }
}

impl SqliteVideoRepository {
    fn exists(conn: &Connection, id: &VideoId) -> anyhow::Result<bool> {
        conn.query_row(
            "SELECT 1 FROM videos WHERE id = ?1",
            params![id.as_str()],
            |_| Ok(()),
        )
        .optional()
        .context("failed to check whether the video exists")
        .map(|found| found.is_some())
    }

    /// `text` with LIKE's wildcards and escape character escaped, so they
    /// match literally.
    fn escape_like(text: &str) -> String {
        text.chars()
            .flat_map(|c| match c {
                '%' | '_' | '\\' => vec!['\\', c],
                _ => vec![c],
            })
            .collect()
    }

    fn recorded_episode_number(
        conn: &Connection,
        id: &VideoId,
    ) -> anyhow::Result<Option<EpisodeNumber>> {
        let (season, episode): (Option<i32>, Option<u32>) = conn
            .query_row(
                "SELECT season, episode FROM videos WHERE id = ?1",
                params![id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .inspect_err(|e| {
                tracing::error!(video_id = %id, error = %e, "failed to read the episode number")
            })
            .context("failed to read the episode number")?;
        Self::decode_episode_number(season, episode)
    }

    /// The next free same-day index on `published_at`'s day among the videos
    /// sharing a channel with `id`.
    fn next_free_episode_number(
        conn: &Connection,
        id: &VideoId,
        published_at: DateTime<Utc>,
    ) -> anyhow::Result<EpisodeNumber> {
        if !Self::is_channel_video(conn, id)? {
            anyhow::bail!(
                "video {id} has no episode number and is in no channel: only a channel video is numbered by publish date"
            );
        }
        let last_of_day = Self::last_sibling_episode_of_day(conn, id, published_at)?;
        Ok(ChannelEpisodeNumber::next_of_day(published_at, last_of_day)?.into())
    }

    fn is_channel_video(conn: &Connection, id: &VideoId) -> anyhow::Result<bool> {
        conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM channel_videos WHERE video_id = ?1)",
            params![id.as_str()],
            |row| row.get(0),
        )
        .inspect_err(|e| {
            tracing::error!(video_id = %id, error = %e, "failed to read the video's channel membership")
        })
        .context("failed to read the video's channel membership")
    }

    /// The highest episode recorded on `published_at`'s day among the videos sharing
    /// a channel with `id`.
    fn last_sibling_episode_of_day(
        conn: &Connection,
        id: &VideoId,
        published_at: DateTime<Utc>,
    ) -> anyhow::Result<Option<u32>> {
        let first = ChannelEpisodeNumber::first_of_day(published_at);
        conn.query_row(
            "SELECT MAX(episode) FROM videos
             WHERE season = ?2 AND episode / 100 = ?3 AND id IN (
                 SELECT video_id FROM channel_videos WHERE channel_id IN (
                     SELECT channel_id FROM channel_videos WHERE video_id = ?1))",
            params![id.as_str(), first.season(), first.same_day_prefix()],
            |row| row.get(0),
        )
        .inspect_err(|e| {
            tracing::error!(video_id = %id, error = %e, "failed to find the last episode of the day")
        })
        .context("failed to find the last episode of the day")
    }

    fn read_row(row: &Row<'_>) -> rusqlite::Result<VideoRow> {
        Ok(VideoRow {
            id: row.get(0)?,
            youtube_id: row.get(1)?,
            title: row.get(2)?,
            status: row.get(3)?,
            quality: row.get(4)?,
            filename: row.get(5)?,
            thumbnail_filename: row.get(6)?,
            duration_seconds: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
            watched_at: row.get(10)?,
            playback_position_seconds: row.get(11)?,
            synced_at: row.get(12)?,
            last_errored_at: row.get(13)?,
            last_played_at: row.get(14)?,
            season: row.get(15)?,
            episode: row.get(16)?,
            subtitles_status: row.get(17)?,
            subtitle_filename: row.get(18)?,
            subtitles_last_checked_at: row.get(19)?,
            subtitles_failed_attempts: row.get(20)?,
            chapters: row.get(21)?,
            chapters_last_checked_at: row.get(22)?,
            chapters_failed_attempts: row.get(23)?,
            chapters_status: row.get(24)?,
            downloaded_height: row.get(25)?,
            quality_last_checked_at: row.get(26)?,
            quality_failed_attempts: row.get(27)?,
            quality_status: row.get(28)?,
            summary_status: row.get(29)?,
            version: row.get(30)?,
            download_failed_attempts: row.get(31)?,
            download_failure: row.get(32)?,
            thumbnail_status: row.get(33)?,
            thumbnail_failed_attempts: row.get(34)?,
            thumbnail_last_failed_at: row.get(35)?,
            summary_failed_attempts: row.get(36)?,
            summary_last_failed_at: row.get(37)?,
            categorization_status: row.get(38)?,
            categorization_failed_attempts: row.get(39)?,
            categorization_last_failed_at: row.get(40)?,
        })
    }

    fn row_to_video(row: VideoRow) -> anyhow::Result<Video> {
        Ok(Video {
            id: VideoId::new(row.id)?,
            youtube_id: YoutubeVideoId::new(row.youtube_id)?,
            title: row.title,
            status: VideoStatus::parse(&row.status)?,
            download_attempts: RetryAttempts {
                failed: row.download_failed_attempts,
                last_failed_at: Self::decode_timestamp(row.last_errored_at, "last_errored_at")?,
            },
            download_failure: row.download_failure,
            quality: row.quality.map(Quality::new).transpose()?,
            filename: row.filename,
            thumbnail_filename: row.thumbnail_filename,
            thumbnail: Self::decode_step(
                &row.thumbnail_status,
                row.thumbnail_failed_attempts,
                row.thumbnail_last_failed_at,
                "thumbnail_last_failed_at",
            )?,
            duration_seconds: row.duration_seconds,
            created_at: Self::parse_timestamp(&row.created_at, "created_at")?,
            updated_at: Self::parse_timestamp(&row.updated_at, "updated_at")?,
            synced_at: Self::decode_timestamp(row.synced_at, "synced_at")?,
            episode_number: Self::decode_episode_number(row.season, row.episode)?,
            subtitles: Self::decode_checks(
                &row.subtitles_status,
                row.subtitles_failed_attempts,
                row.subtitles_last_checked_at,
                "subtitles_last_checked_at",
            )?,
            subtitle_filename: row.subtitle_filename,
            chapters: row
                .chapters
                .map(|chapters| Self::decode_chapters(&chapters))
                .transpose()?,
            chapters_checks: Self::decode_checks(
                &row.chapters_status,
                row.chapters_failed_attempts,
                row.chapters_last_checked_at,
                "chapters_last_checked_at",
            )?,
            downloaded_height: row.downloaded_height,
            quality_checks: Self::decode_checks(
                &row.quality_status,
                row.quality_failed_attempts,
                row.quality_last_checked_at,
                "quality_last_checked_at",
            )?,
            summary: Self::decode_step(
                &row.summary_status,
                row.summary_failed_attempts,
                row.summary_last_failed_at,
                "summary_last_failed_at",
            )?,
            categorization: Self::decode_step(
                &row.categorization_status,
                row.categorization_failed_attempts,
                row.categorization_last_failed_at,
                "categorization_last_failed_at",
            )?,
            version: row.version,
            watch: WatchState {
                watched_at: Self::decode_timestamp(row.watched_at, "watched_at")?,
                position: PlaybackPosition::new(row.playback_position_seconds)?,
                last_played_at: Self::decode_timestamp(row.last_played_at, "last_played_at")?,
            },
        })
    }

    /// A video's chapters as stored: a JSON array, or NULL while unchecked.
    fn encode_chapters(chapters: &Option<Vec<VideoChapter>>) -> anyhow::Result<Option<String>> {
        chapters
            .as_ref()
            .map(|chapters| {
                serde_json::to_string(chapters).context("failed to serialize video chapters")
            })
            .transpose()
    }

    /// A late-data step as stored: its status, failed attempts and last
    /// check time, which is also when its last attempt failed.
    fn decode_checks(
        status: &str,
        failed: u32,
        last_checked_at: Option<String>,
        column: &str,
    ) -> anyhow::Result<LateDataChecks> {
        let last_checked_at = Self::decode_timestamp(last_checked_at, column)?;
        Ok(LateDataChecks::restored(
            StepStatus::parse(status)?,
            failed,
            last_checked_at,
        ))
    }

    fn decode_step(
        status: &str,
        failed: u32,
        last_failed_at: Option<String>,
        column: &str,
    ) -> anyhow::Result<StepState> {
        Ok(StepState {
            status: StepStatus::parse(status)?,
            attempts: RetryAttempts {
                failed,
                last_failed_at: Self::decode_timestamp(last_failed_at, column)?,
            },
        })
    }

    fn encode_timestamp(at: Option<DateTime<Utc>>) -> Option<String> {
        at.map(|at| at.to_rfc3339())
    }

    fn decode_timestamp(
        value: Option<String>,
        column: &str,
    ) -> anyhow::Result<Option<DateTime<Utc>>> {
        value.map(|v| Self::parse_timestamp(&v, column)).transpose()
    }

    fn decode_chapters(value: &str) -> anyhow::Result<Vec<VideoChapter>> {
        serde_json::from_str(value).context("failed to parse stored chapters")
    }

    fn parse_timestamp(value: &str, column: &str) -> anyhow::Result<DateTime<Utc>> {
        Ok(DateTime::parse_from_rfc3339(value)
            .with_context(|| format!("failed to parse stored {column}"))?
            .with_timezone(&Utc))
    }

    /// The episode number recorded in a row's `season`/`episode` columns,
    /// none unless both are set.
    fn decode_episode_number(
        season: Option<i32>,
        episode: Option<u32>,
    ) -> anyhow::Result<Option<EpisodeNumber>> {
        season
            .zip(episode)
            .map(|(season, episode)| {
                EpisodeNumber::from_columns(season, episode).map_err(Into::into)
            })
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::ChannelHandle;
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::playlist::PlaylistId;
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::video::PlaylistEpisodeNumber;
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, SqliteChannelVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::{
        PlaylistVideoRepository, SqlitePlaylistVideoRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;

    fn repo() -> SqliteVideoRepository {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::shared::sqlite_migrations::apply(&mut conn).unwrap();
        SqliteVideoRepository::new(Database::single(conn))
    }

    #[test]
    fn it_should_round_trip_every_step_state() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(1_000, 0).unwrap();
        let failed_at = DateTime::<Utc>::from_timestamp(900, 0).unwrap();
        let attempts = |failed| RetryAttempts {
            failed,
            last_failed_at: Some(failed_at),
        };
        let step = |status, failed| StepState {
            status,
            attempts: attempts(failed),
        };
        let checked = |status, failed| LateDataChecks {
            state: step(status, failed),
            last_checked_at: Some(failed_at),
        };
        let video = Video {
            status: VideoStatus::Retrying,
            download_attempts: attempts(3),
            download_failure: Some("HTTP Error 503".to_string()),
            thumbnail: step(StepStatus::Failed, 11),
            subtitles: checked(StepStatus::Retrying, 2),
            chapters_checks: LateDataChecks {
                state: StepState::pending().unavailable(),
                last_checked_at: Some(failed_at),
            },
            quality_checks: checked(StepStatus::Failed, 11),
            summary: step(StepStatus::Retrying, 1),
            categorization: step(StepStatus::Failed, 11),
            ..video("Every step", now)
        };

        repo.insert(&video).unwrap();
        let inserted = repo.find(&video.id).unwrap();
        let retried = video.clone().retry(now);
        repo.update(&retried).unwrap();

        assert_eq!(
            (inserted, repo.find(&video.id).unwrap()),
            (
                Some(video),
                Some(Video {
                    version: 1,
                    ..retried
                })
            )
        );
    }

    #[test]
    fn it_should_read_a_pending_summary_for_a_new_video() {
        let db = TestDatabase::new();
        let repo = SqliteVideoRepository::new(db.database());
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let video = video("New", now);

        repo.insert(&video).unwrap();

        let stored = repo.find(&video.id).unwrap().unwrap();
        assert_eq!(stored.summary, StepState::pending());
    }

    #[test]
    fn it_should_reset_failed_summaries_to_pending() {
        let db = TestDatabase::new();
        let repo = SqliteVideoRepository::new(db.database());
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let later = DateTime::<Utc>::from_timestamp(200, 0).unwrap();
        let failed = video("Failed", now).mark_summary_failed(now);
        let generated = Video::create(YoutubeVideoId::new("yt2").unwrap(), "Generated", now)
            .mark_summary_generated(now);
        repo.insert(&failed).unwrap();
        repo.insert(&generated).unwrap();

        repo.reset_failed_summaries(later).unwrap();

        assert_eq!(
            (
                repo.find(&failed.id).unwrap(),
                repo.find(&generated.id).unwrap()
            ),
            (
                Some(Video {
                    summary: StepState::pending(),
                    updated_at: later,
                    version: 1,
                    ..failed
                }),
                Some(generated)
            )
        );
    }

    #[test]
    fn it_should_bump_the_version_on_update() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let video = video("First", now);
        repo.insert(&video).unwrap();
        let renamed = video.with_title("Renamed", now);

        repo.update(&renamed).unwrap();

        assert_eq!(
            repo.find(&renamed.id).unwrap(),
            Some(Video {
                version: 1,
                ..renamed
            })
        );
    }

    #[test]
    fn it_should_fail_to_update_a_stale_video() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let read = video("First", now);
        repo.insert(&read).unwrap();
        repo.update(&read.clone().with_title("Renamed", now))
            .unwrap();

        let result = repo.update(&read.clone().mark_watched(now));

        assert_eq!(
            result.map_err(|e| e.downcast::<StaleVideo>().ok()),
            Err(Some(StaleVideo(read.id.clone())))
        );
        assert_eq!(
            repo.find(&read.id).unwrap(),
            Some(Video {
                version: 1,
                ..read.with_title("Renamed", now)
            })
        );
    }

    #[test]
    fn it_should_ignore_update_of_a_deleted_video() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let video = video("First", now);
        repo.insert(&video).unwrap();
        repo.delete(&video.id).unwrap();

        let result = repo.update(&video.clone().mark_watched(now));

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        assert_eq!(repo.find(&video.id).unwrap(), None);
    }

    #[test]
    fn it_should_bump_the_version_when_resetting_failed_summaries() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let later = DateTime::<Utc>::from_timestamp(200, 0).unwrap();
        let failed = video("Failed", now).mark_summary_failed(now);
        repo.insert(&failed).unwrap();

        repo.reset_failed_summaries(later).unwrap();

        assert_eq!(
            repo.find(&failed.id).unwrap(),
            Some(Video {
                summary: StepState::pending(),
                updated_at: later,
                version: 1,
                ..failed
            })
        );
    }

    #[test]
    fn it_should_insert_a_new_video_at_version_zero() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let video = video("First", now);

        repo.insert(&video).unwrap();

        assert_eq!(
            repo.find(&video.id).unwrap(),
            Some(Video {
                version: 0,
                ..video
            })
        );
    }

    #[test]
    fn it_should_fail_to_insert_an_existing_video() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let video = video("First", now);
        repo.insert(&video).unwrap();

        let result = repo.insert(&Video {
            title: "Second".to_string(),
            ..video.clone()
        });

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err("failed to insert video".to_string())
        );
        assert_eq!(repo.find(&video.id).unwrap(), Some(video));
    }

    fn video(title: &str, now: DateTime<Utc>) -> Video {
        Video::create(YoutubeVideoId::new("yt1").unwrap(), title, now)
    }

    #[test]
    fn it_should_return_the_video_after_saving_it() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let video = video("First", now);

        repo.insert(&video).unwrap();

        let found = repo.find(&video.id).unwrap().unwrap();
        assert_eq!(found.title, "First");
        assert_eq!(found.youtube_id.as_str(), "yt1");
        assert_eq!(found.status, VideoStatus::Pending);
        assert_eq!(found.quality, None);
    }

    #[test]
    fn it_should_find_many_videos_in_the_order_given_skipping_missing_ones() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let first = video("First", now);
        let second = video("Second", now);
        let unrequested = video("Unrequested", now);
        repo.insert(&first).unwrap();
        repo.insert(&second).unwrap();
        repo.insert(&unrequested).unwrap();

        let found = repo.find_many(&[
            second.id.clone(),
            VideoId::new_generated(),
            first.id.clone(),
        ]);

        assert_eq!(found.unwrap(), vec![second, first]);
    }

    #[test]
    fn it_should_find_many_of_no_ids() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        repo.insert(&video("First", now)).unwrap();

        let found = repo.find_many(&[]);

        assert_eq!(found.unwrap(), vec![]);
    }

    #[test]
    fn it_should_allow_two_videos_to_share_the_same_youtube_id() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let a = Video::create(YoutubeVideoId::new("shared").unwrap(), "A", now);
        let b = Video::create(YoutubeVideoId::new("shared").unwrap(), "B", now);

        repo.insert(&a).unwrap();
        repo.insert(&b).unwrap();

        assert_eq!(repo.find(&a.id).unwrap().unwrap().title, "A");
        assert_eq!(repo.find(&b.id).unwrap().unwrap().title, "B");
    }

    #[test]
    fn it_should_round_trip_a_video_with_a_recorded_quality() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let downloaded = video("First", now).start_download(now).mark_downloaded(
            Quality::Mid,
            "First.mp4",
            None,
            None,
            now,
        );
        repo.insert(&downloaded).unwrap();

        let found = repo.find(&downloaded.id).unwrap().unwrap();

        assert_eq!(found.quality, Some(Quality::Mid));
        assert_eq!(found.filename, Some("First.mp4".to_string()));
    }

    #[test]
    fn it_should_round_trip_a_video_with_a_recorded_thumbnail_filename() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let downloaded = video("First", now).start_download(now).mark_downloaded(
            Quality::Mid,
            "First.mp4",
            Some("First.jpg".to_string()),
            None,
            now,
        );
        repo.insert(&downloaded).unwrap();

        let found = repo.find(&downloaded.id).unwrap().unwrap();

        assert_eq!(found.thumbnail_filename, Some("First.jpg".to_string()));
    }

    #[test]
    fn it_should_round_trip_a_video_with_a_recorded_duration() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let downloaded = video("First", now).start_download(now).mark_downloaded(
            Quality::Mid,
            "First.mp4",
            None,
            Some(223),
            now,
        );
        repo.insert(&downloaded).unwrap();

        let found = repo.find(&downloaded.id).unwrap().unwrap();

        assert_eq!(found.duration_seconds, Some(223));
    }

    #[test]
    fn it_should_round_trip_a_video_with_no_recorded_duration() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let downloaded = video("First", now).start_download(now).mark_downloaded(
            Quality::Mid,
            "First.mp4",
            None,
            None,
            now,
        );
        repo.insert(&downloaded).unwrap();

        let found = repo.find(&downloaded.id).unwrap().unwrap();

        assert_eq!(found.duration_seconds, None);
    }

    #[test]
    fn it_should_delete_only_the_named_video() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let one = video("One", now);
        let two = video("Two", now);
        repo.insert(&one).unwrap();
        repo.insert(&two).unwrap();

        repo.delete(&one.id).unwrap();

        assert!(repo.find(&one.id).unwrap().is_none());
        assert!(repo.find(&two.id).unwrap().is_some());
    }

    #[test]
    fn it_should_succeed_when_deleting_a_missing_video() {
        let repo = repo();

        assert!(repo.delete(&VideoId::new_generated()).is_ok());
    }

    #[test]
    fn it_should_return_none_when_finding_a_missing_video() {
        let repo = repo();

        let found = repo.find(&VideoId::new_generated()).unwrap();

        assert!(found.is_none());
    }

    #[test]
    fn it_should_return_the_stored_video_with_its_updated_status_after_update() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let original = video("First", now);
        repo.insert(&original).unwrap();
        let stored = repo.find(&original.id).unwrap().unwrap();

        let later = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        repo.update(&stored.start_download(later)).unwrap();

        let found = repo.find(&original.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::InProgress);
        assert_eq!(found.updated_at, later);
    }

    #[test]
    fn it_should_round_trip_chapters_none_empty_and_some() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let unchecked = Video::create(YoutubeVideoId::new("yt1").unwrap(), "Unchecked", now);
        let without = Video::create(YoutubeVideoId::new("yt2").unwrap(), "Without", now)
            .with_chapters(vec![], now);
        let with = Video::create(YoutubeVideoId::new("yt3").unwrap(), "With", now).with_chapters(
            vec![
                VideoChapter {
                    start_seconds: 0,
                    end_seconds: 60,
                    title: "Intro \"quoted\" & more".to_string(),
                },
                VideoChapter {
                    start_seconds: 60,
                    end_seconds: 600,
                    title: "Main".to_string(),
                },
            ],
            now,
        );
        [&unchecked, &without, &with]
            .into_iter()
            .for_each(|video| repo.insert(video).unwrap());

        assert_eq!(repo.list().unwrap(), vec![unchecked, without, with]);
    }

    #[test]
    fn it_should_round_trip_late_data_checks() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let checked_at = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let checks = LateDataChecks::checked(Some(checked_at), 2);
        let video = Video {
            subtitles: checks,
            chapters: Some(vec![]),
            chapters_checks: LateDataChecks::done_at(Some(checked_at)),
            downloaded_height: Some(1080),
            quality_checks: LateDataChecks::done_at(Some(checked_at)),
            ..video("Checked", now)
        };

        repo.insert(&video).unwrap();

        assert_eq!(repo.find(&video.id).unwrap(), Some(video));
    }

    #[test]
    fn it_should_round_trip_the_subtitle_checks() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let checked_at = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let original = video("First", now);
        repo.insert(&original).unwrap();
        let checked = Video {
            subtitles: LateDataChecks::checked(Some(checked_at), 2),
            updated_at: checked_at,
            ..original.clone()
        };

        repo.update(&checked).unwrap();

        assert_eq!(
            repo.find(&original.id).unwrap(),
            Some(Video {
                version: 1,
                ..checked
            })
        );
    }

    #[test]
    fn it_should_round_trip_a_watched_video_with_a_playback_position() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let watched_at = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let video = Video {
            watch: WatchState {
                watched_at: Some(watched_at),
                position: PlaybackPosition::new(42).unwrap(),
                last_played_at: None,
            },
            ..video("First", now)
        };

        repo.insert(&video).unwrap();

        assert_eq!(repo.find(&video.id).unwrap(), Some(video));
    }

    #[test]
    fn it_should_round_trip_a_video_with_a_recorded_last_errored_time() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let errored_at = DateTime::<Utc>::from_timestamp(100, 0).unwrap();
        let video = video("First", now).mark_download_failed("HTTP Error 503", errored_at);

        repo.insert(&video).unwrap();

        assert_eq!(repo.find(&video.id).unwrap(), Some(video));
    }

    #[test]
    fn it_should_round_trip_a_video_with_a_last_played_time() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let video = Video {
            watch: WatchState {
                last_played_at: Some(DateTime::<Utc>::from_timestamp(100, 0).unwrap()),
                ..WatchState::unwatched()
            },
            ..video("First", now)
        };
        repo.insert(&video).unwrap();
        let replayed = Video {
            watch: WatchState {
                last_played_at: Some(DateTime::<Utc>::from_timestamp(200, 0).unwrap()),
                ..video.watch
            },
            ..video.clone()
        };

        repo.update(&replayed).unwrap();

        assert_eq!(
            repo.find(&video.id).unwrap(),
            Some(Video {
                version: 1,
                ..replayed
            })
        );
    }

    #[test]
    fn it_should_round_trip_a_video_never_played() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let video = Video {
            watch: WatchState {
                last_played_at: None,
                ..WatchState::unwatched()
            },
            ..video("First", now)
        };

        repo.insert(&video).unwrap();

        assert_eq!(repo.find(&video.id).unwrap(), Some(video));
    }

    #[test]
    fn it_should_find_every_copy_of_a_youtube_video() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let first_copy = Video::create(YoutubeVideoId::new("shared").unwrap(), "A", now);
        let other = Video::create(YoutubeVideoId::new("other").unwrap(), "B", now);
        let second_copy = Video::create(YoutubeVideoId::new("shared").unwrap(), "C", now);
        repo.insert(&first_copy).unwrap();
        repo.insert(&other).unwrap();
        repo.insert(&second_copy).unwrap();

        let found = repo.find_by_youtube_id(&YoutubeVideoId::new("shared").unwrap());

        assert_eq!(found.unwrap(), vec![first_copy, second_copy]);
    }

    #[test]
    fn it_should_find_no_copies_of_an_unknown_youtube_video() {
        let repo = repo();
        let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        repo.insert(&video("First", now)).unwrap();

        let found = repo.find_by_youtube_id(&YoutubeVideoId::new("unknown").unwrap());

        assert_eq!(found.unwrap(), vec![]);
    }

    #[test]
    fn it_should_bump_the_version_when_assigning_an_episode_number() {
        let db = TestDatabase::new();
        let repo = SqliteVideoRepository::new(db.database());
        let video = video("First", fixed_timestamp());
        repo.insert(&video).unwrap();
        link_to_channel(&db, &video, "@somechannel");

        repo.assign_episode_number(&video.id, published_at())
            .unwrap();

        assert_eq!(
            repo.find(&video.id).unwrap(),
            Some(Video {
                episode_number: Some(episode(31501)),
                version: 1,
                ..video
            })
        );
    }

    #[test]
    fn it_should_assign_the_first_index_of_the_day() {
        let db = TestDatabase::new();
        let repo = SqliteVideoRepository::new(db.database());
        let video = video("First", fixed_timestamp());
        repo.insert(&video).unwrap();
        link_to_channel(&db, &video, "@somechannel");

        let number = repo.assign_episode_number(&video.id, published_at());

        assert_eq!(number.unwrap(), episode(31501));
        assert_eq!(
            repo.find(&video.id).unwrap(),
            Some(Video {
                version: 1,
                episode_number: Some(episode(31501)),
                ..video
            })
        );
    }

    #[test]
    fn it_should_assign_the_next_free_index_within_the_same_channel_and_day() {
        let db = TestDatabase::new();
        let repo = SqliteVideoRepository::new(db.database());
        let sibling = video("Sibling", fixed_timestamp());
        let other_channel_video = video("Other channel video", fixed_timestamp());
        let video = video("Video", fixed_timestamp());
        [&sibling, &other_channel_video, &video]
            .into_iter()
            .for_each(|video| repo.insert(video).unwrap());
        let _ = numbered(&repo, sibling.clone(), episode(31503));
        let _ = numbered(&repo, other_channel_video.clone(), episode(31507));
        link_to_channel(&db, &sibling, "@somechannel");
        link_to_channel(&db, &other_channel_video, "@otherchannel");
        link_to_channel(&db, &video, "@somechannel");

        let number = repo.assign_episode_number(&video.id, published_at());

        assert_eq!(number.unwrap(), episode(31504));
    }

    #[test]
    fn it_should_fail_to_number_an_unnumbered_playlist_video_by_publish_date() {
        let db = TestDatabase::new();
        let repo = SqliteVideoRepository::new(db.database());
        let video = video("Playlist video", fixed_timestamp());
        repo.insert(&video).unwrap();
        link_to_playlist(&db, &video, "PL1");

        let number = repo.assign_episode_number(&video.id, published_at());

        assert_eq!(
            number.map_err(|e| e.to_string()),
            Err(format!(
                "video {} has no episode number and is in no channel: only a channel video is numbered by publish date",
                video.id
            ))
        );
        assert_eq!(repo.find(&video.id).unwrap(), Some(video));
    }

    #[test]
    fn it_should_number_copies_in_different_shows_independently() {
        let db = TestDatabase::new();
        let repo = SqliteVideoRepository::new(db.database());
        let first_copy = video("Copy", fixed_timestamp());
        let second_copy = video("Copy", fixed_timestamp());
        repo.insert(&first_copy).unwrap();
        repo.insert(&second_copy).unwrap();
        link_to_channel(&db, &first_copy, "@somechannel");
        link_to_channel(&db, &second_copy, "@otherchannel");

        let numbers = [&first_copy, &second_copy].map(|video| {
            repo.assign_episode_number(&video.id, published_at())
                .unwrap()
        });

        assert_eq!(numbers, [episode(31501), episode(31501)]);
    }

    #[test]
    fn it_should_return_the_existing_number_when_already_assigned() {
        let db = TestDatabase::new();
        let repo = SqliteVideoRepository::new(db.database());
        let video = video("First", fixed_timestamp());
        repo.insert(&video).unwrap();
        let video = numbered(&repo, video, episode(31503));
        link_to_playlist(&db, &video, "PL1");

        let number = repo.assign_episode_number(
            &video.id,
            DateTime::parse_from_rfc3339("2026-04-01T10:00:00Z")
                .unwrap()
                .to_utc(),
        );

        assert_eq!(number.unwrap(), episode(31503));
        assert_eq!(
            repo.find(&video.id).unwrap(),
            Some(Video {
                version: 1,
                ..video
            })
        );
    }

    #[test]
    fn it_should_persist_and_read_the_episode_number() {
        let repo = repo();
        let saved = video("Saved", fixed_timestamp())
            .with_episode_number(episode(31501), fixed_timestamp());
        let updated = video("Updated", fixed_timestamp());
        repo.insert(&saved).unwrap();
        repo.insert(&updated).unwrap();
        let updated = updated.with_episode_number(episode(31502), fixed_timestamp());

        repo.update(&updated).unwrap();

        assert_eq!(
            [
                repo.find(&saved.id).unwrap(),
                repo.find(&updated.id).unwrap()
            ],
            [
                Some(saved),
                Some(Video {
                    version: 1,
                    ..updated
                })
            ]
        );
    }

    #[test]
    fn it_should_persist_and_read_a_playlist_episode_number() {
        let repo = repo();
        let video = video("Playlist video", fixed_timestamp()).with_episode_number(
            PlaylistEpisodeNumber::new(7).unwrap().into(),
            fixed_timestamp(),
        );

        repo.insert(&video).unwrap();

        assert_eq!(repo.find(&video.id).unwrap(), Some(video));
    }

    #[test]
    fn it_should_list_downloaded_videos() {
        let repo = repo();
        let pending = video("Pending", fixed_timestamp());
        let downloaded = video("Downloaded", fixed_timestamp())
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                "Season 2026/S2026E031501 - Downloaded.mp4",
                None,
                None,
                fixed_timestamp(),
            );
        let errored = video("Errored", fixed_timestamp())
            .mark_download_failed("HTTP Error 503", fixed_timestamp());
        [&pending, &downloaded, &errored]
            .into_iter()
            .for_each(|video| repo.insert(video).unwrap());

        let listed = repo.list_downloaded();

        assert_eq!(listed.unwrap(), vec![downloaded]);
    }

    #[test]
    fn it_should_find_downloaded_videos_by_title_ignoring_case() {
        let repo = repo();
        let tactics = downloaded("Chess Tactics: Spot the Trigger", 100);
        let endgame = downloaded("Endgame Skills", 200);
        let accented = downloaded("Víctor Falcón explica", 300);
        [&tactics, &endgame, &accented]
            .into_iter()
            .for_each(|video| repo.insert(video).unwrap());

        let found = ["tactics", "TACTICS", "falcon"]
            .map(|text| repo.find_downloaded_by_title(text, 20).unwrap());

        assert_eq!(found, [vec![tactics.clone()], vec![tactics], vec![]]);
    }

    #[test]
    fn it_should_match_wildcards_in_a_title_search_literally() {
        let repo = repo();
        let percent = downloaded("100% Accuracy", 100);
        let underscore = downloaded("snake_case explained", 200);
        let backslash = downloaded("C:\\Windows tour", 300);
        let other = downloaded("1000 Accuracy snakeXcase C:Windows", 400);
        [&percent, &underscore, &backslash, &other]
            .into_iter()
            .for_each(|video| repo.insert(video).unwrap());

        let found =
            ["0%", "e_c", ":\\W"].map(|text| repo.find_downloaded_by_title(text, 20).unwrap());

        assert_eq!(found, [vec![percent], vec![underscore], vec![backslash]]);
    }

    #[test]
    fn it_should_find_only_downloaded_videos_by_title() {
        let repo = repo();
        let pending = video("Tactics pending", fixed_timestamp());
        let downloaded = downloaded("Tactics downloaded", 100);
        [&pending, &downloaded]
            .into_iter()
            .for_each(|video| repo.insert(video).unwrap());

        let found = repo.find_downloaded_by_title("tactics", 20);

        assert_eq!(found.unwrap(), vec![downloaded]);
    }

    #[test]
    fn it_should_find_the_newest_synced_videos_by_title_up_to_the_limit() {
        let repo = repo();
        let videos = (0..5)
            .map(|index| downloaded(&format!("Tactics {index}"), index * 100))
            .collect::<Vec<_>>();
        videos.iter().for_each(|video| repo.insert(video).unwrap());

        let found = repo.find_downloaded_by_title("tactics", 3);

        assert_eq!(
            found.unwrap(),
            vec![videos[4].clone(), videos[3].clone(), videos[2].clone()]
        );
    }

    #[test]
    fn it_should_peek_the_next_episode_number_without_recording_it() {
        let db = TestDatabase::new();
        let repo = SqliteVideoRepository::new(db.database());
        let sibling = video("Sibling", fixed_timestamp());
        let video = video("Next", fixed_timestamp());
        repo.insert(&sibling).unwrap();
        repo.insert(&video).unwrap();
        let _ = numbered(&repo, sibling.clone(), episode(31501));
        link_to_channel(&db, &sibling, "@somechannel");
        link_to_channel(&db, &video, "@somechannel");

        let number = repo.next_episode_number(&video.id, published_at());

        assert_eq!(number.unwrap(), episode(31502));
        assert_eq!(repo.find(&video.id).unwrap(), Some(video));
    }

    fn downloaded(title: &str, created_at_seconds: i64) -> Video {
        let created_at = DateTime::<Utc>::from_timestamp(created_at_seconds, 0).unwrap();
        Video::create(YoutubeVideoId::new("yt1").unwrap(), title, created_at)
            .start_download(created_at)
            .mark_downloaded(
                Quality::High,
                format!("{title}.mp4"),
                None,
                None,
                created_at,
            )
    }

    fn link_to_playlist(db: &TestDatabase, video: &Video, playlist_id: &str) {
        SqlitePlaylistVideoRepository::new(db.database())
            .save(&PlaylistVideo::create(
                PlaylistId::new(playlist_id).unwrap(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
    }

    fn link_to_channel(db: &TestDatabase, video: &Video, channel_id: &str) {
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                ChannelHandle::new(channel_id).unwrap(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
    }

    /// Records `number` on a stored `video`.
    fn numbered(repo: &SqliteVideoRepository, video: Video, number: EpisodeNumber) -> Video {
        let numbered = video.with_episode_number(number, fixed_timestamp());
        repo.update(&numbered).unwrap();
        numbered
    }

    fn episode(episode: u32) -> EpisodeNumber {
        EpisodeNumber::from_columns(2026, episode).unwrap()
    }

    fn published_at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-03-15T10:00:00Z")
            .unwrap()
            .to_utc()
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }
}
