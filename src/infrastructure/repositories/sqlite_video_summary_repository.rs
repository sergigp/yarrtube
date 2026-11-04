use crate::domain::video::VideoRecordId;
use crate::domain::video_summary::{SummaryText, VideoSummary};
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, Row, params};

/// Stores each video's AI summary, at most one per video.
pub trait VideoSummaryRepository: Send + Sync {
    /// Inserts the summary, replacing the video's previous one.
    fn save(&self, summary: &VideoSummary) -> anyhow::Result<()>;
    /// No production reader yet: nothing displays summaries.
    #[allow(dead_code)]
    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoSummary>>;
    fn delete(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
}

/// A `video_summaries` row as read, before its values are parsed into a
/// `VideoSummary`.
struct VideoSummaryRow {
    video_id: String,
    text: String,
    model: String,
    created_at: String,
}

pub struct SqliteVideoSummaryRepository {
    db: Database,
}

impl SqliteVideoSummaryRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl VideoSummaryRepository for SqliteVideoSummaryRepository {
    fn save(&self, summary: &VideoSummary) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            "INSERT INTO video_summaries (video_id, text, model, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (video_id) DO UPDATE SET
                text = excluded.text,
                model = excluded.model,
                created_at = excluded.created_at",
            params![
                summary.video_id.as_str(),
                summary.text.as_str(),
                summary.model,
                summary.created_at.to_rfc3339(),
            ],
        )
        .inspect_err(|e| {
            tracing::error!(video_id = %summary.video_id, error = %e, "failed to save video summary")
        })
        .context("failed to save video summary")?;
        Ok(())
    }

    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoSummary>> {
        let conn = self.db.read()?;
        conn.query_row(
            "SELECT video_id, text, model, created_at FROM video_summaries WHERE video_id = ?1",
            params![video_id.as_str()],
            Self::read_row,
        )
        .optional()
        .inspect_err(
            |e| tracing::error!(video_id = %video_id, error = %e, "failed to find video summary"),
        )
        .context("failed to find video summary")?
        .map(Self::row_to_video_summary)
        .transpose()
    }

    fn delete(&self, _video_id: &VideoRecordId) -> anyhow::Result<()> {
        Ok(())
    }
}

impl SqliteVideoSummaryRepository {
    fn read_row(row: &Row<'_>) -> rusqlite::Result<VideoSummaryRow> {
        Ok(VideoSummaryRow {
            video_id: row.get(0)?,
            text: row.get(1)?,
            model: row.get(2)?,
            created_at: row.get(3)?,
        })
    }

    fn row_to_video_summary(row: VideoSummaryRow) -> anyhow::Result<VideoSummary> {
        Ok(VideoSummary {
            video_id: VideoRecordId::new(row.video_id)?,
            text: SummaryText::new(row.text),
            model: row.model,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .context("failed to parse stored created_at")?
                .with_timezone(&Utc),
        })
    }
}
