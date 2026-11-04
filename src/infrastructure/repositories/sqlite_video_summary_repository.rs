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

    fn delete(&self, video_id: &VideoRecordId) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            "DELETE FROM video_summaries WHERE video_id = ?1",
            params![video_id.as_str()],
        )
        .inspect_err(
            |e| tracing::error!(video_id = %video_id, error = %e, "failed to delete video summary"),
        )
        .context("failed to delete video summary")?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;

    #[test]
    fn it_should_save_and_find_a_summary() {
        let db = TestDatabase::new();
        let repository = SqliteVideoSummaryRepository::new(db.database());
        let stored = summary("rec1", "Three games.");

        let result = repository.save(&stored);

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        assert_eq!(repository.find(&stored.video_id).unwrap(), Some(stored));
    }

    #[test]
    fn it_should_replace_a_videos_summary() {
        let db = TestDatabase::new();
        let repository = SqliteVideoSummaryRepository::new(db.database());
        let replacement = summary("rec1", "Four games.");
        repository.save(&summary("rec1", "Three games.")).unwrap();

        repository.save(&replacement).unwrap();

        assert_eq!(
            repository.find(&replacement.video_id).unwrap(),
            Some(replacement)
        );
    }

    #[test]
    fn it_should_delete_a_summary() {
        let db = TestDatabase::new();
        let repository = SqliteVideoSummaryRepository::new(db.database());
        let (deleted, kept) = (summary("rec1", "Gone."), summary("rec2", "Kept."));
        repository.save(&deleted).unwrap();
        repository.save(&kept).unwrap();

        let result = repository.delete(&deleted.video_id);

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        assert_eq!(
            [&deleted, &kept].map(|summary| repository.find(&summary.video_id).unwrap()),
            [None, Some(kept.clone())]
        );
    }

    fn summary(video_id: &str, text: &str) -> VideoSummary {
        VideoSummary {
            video_id: VideoRecordId::new(video_id).unwrap(),
            text: SummaryText::new(text),
            model: "qwen3:4b".to_string(),
            created_at: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
        }
    }
}
