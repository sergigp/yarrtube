use crate::domain::video::VideoRecordId;
use crate::domain::video_transcript::{
    CaptionLanguage, CaptionSource, TranscriptText, VideoTranscript,
};
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, Row, params};

/// Stores each video's plain-text transcript, at most one per video.
pub trait VideoTranscriptRepository: Send + Sync {
    /// Inserts the transcript, replacing the video's previous one.
    fn save(&self, transcript: &VideoTranscript) -> anyhow::Result<()>;
    /// No production reader yet: transcripts are stored for upcoming
    /// summaries and recommendations.
    #[allow(dead_code)]
    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoTranscript>>;
    fn delete(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
}

/// A `video_transcripts` row as read, before its values are parsed into a
/// `VideoTranscript`.
struct VideoTranscriptRow {
    video_id: String,
    language: String,
    source: String,
    text: String,
    created_at: String,
}

pub struct SqliteVideoTranscriptRepository {
    db: Database,
}

impl SqliteVideoTranscriptRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl VideoTranscriptRepository for SqliteVideoTranscriptRepository {
    fn save(&self, transcript: &VideoTranscript) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            "INSERT INTO video_transcripts (video_id, language, source, text, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (video_id) DO UPDATE SET
                language = excluded.language,
                source = excluded.source,
                text = excluded.text,
                created_at = excluded.created_at",
            params![
                transcript.video_id.as_str(),
                transcript.language.as_str(),
                transcript.source.as_str(),
                transcript.text.as_str(),
                transcript.created_at.to_rfc3339(),
            ],
        )
        .inspect_err(|e| {
            tracing::error!(video_id = %transcript.video_id, error = %e, "failed to save video transcript")
        })
        .context("failed to save video transcript")?;
        Ok(())
    }

    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoTranscript>> {
        let conn = self.db.read()?;
        conn.query_row(
            "SELECT video_id, language, source, text, created_at
             FROM video_transcripts WHERE video_id = ?1",
            params![video_id.as_str()],
            Self::read_row,
        )
        .optional()
        .inspect_err(
            |e| tracing::error!(video_id = %video_id, error = %e, "failed to find video transcript"),
        )
        .context("failed to find video transcript")?
        .map(Self::row_to_video_transcript)
        .transpose()
    }

    fn delete(&self, video_id: &VideoRecordId) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            "DELETE FROM video_transcripts WHERE video_id = ?1",
            params![video_id.as_str()],
        )
        .inspect_err(|e| {
            tracing::error!(video_id = %video_id, error = %e, "failed to delete video transcript")
        })
        .context("failed to delete video transcript")?;
        Ok(())
    }
}

impl SqliteVideoTranscriptRepository {
    fn read_row(row: &Row<'_>) -> rusqlite::Result<VideoTranscriptRow> {
        Ok(VideoTranscriptRow {
            video_id: row.get(0)?,
            language: row.get(1)?,
            source: row.get(2)?,
            text: row.get(3)?,
            created_at: row.get(4)?,
        })
    }

    fn row_to_video_transcript(row: VideoTranscriptRow) -> anyhow::Result<VideoTranscript> {
        Ok(VideoTranscript {
            video_id: VideoRecordId::new(row.video_id)?,
            language: CaptionLanguage::new(&row.language)?,
            source: CaptionSource::parse(&row.source)?,
            text: TranscriptText::new(&row.text),
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .context("failed to parse stored created_at")?
                .with_timezone(&Utc),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::video_transcript::CaptionTrack;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;

    #[test]
    fn it_should_replace_the_transcript_of_a_video_on_save() {
        let db = TestDatabase::new();
        let repository = SqliteVideoTranscriptRepository::new(db.database());
        let video_id = VideoRecordId::new("rec1").unwrap();
        let replacement = transcript(&video_id, "es-orig", "es", CaptionSource::Automatic, "Hola");
        repository
            .save(&transcript(
                &video_id,
                "en",
                "en",
                CaptionSource::Manual,
                "Hello",
            ))
            .unwrap();

        let result = repository.save(&replacement);

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        assert_eq!(repository.find(&video_id).unwrap(), Some(replacement));
    }

    #[test]
    fn it_should_delete_a_video_transcript() {
        let db = TestDatabase::new();
        let repository = SqliteVideoTranscriptRepository::new(db.database());
        let deleted_id = VideoRecordId::new("rec1").unwrap();
        let kept_id = VideoRecordId::new("rec2").unwrap();
        let kept = transcript(&kept_id, "en", "en", CaptionSource::Manual, "Kept");
        repository
            .save(&transcript(
                &deleted_id,
                "en",
                "en",
                CaptionSource::Manual,
                "Gone",
            ))
            .unwrap();
        repository.save(&kept).unwrap();

        let result = repository.delete(&deleted_id);

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        assert_eq!(
            [&deleted_id, &kept_id].map(|id| repository.find(id).unwrap()),
            [None, Some(kept)]
        );
    }

    fn transcript(
        video_id: &VideoRecordId,
        key: &str,
        language: &str,
        source: CaptionSource,
        text: &str,
    ) -> VideoTranscript {
        VideoTranscript::create(
            video_id.clone(),
            &CaptionTrack {
                key: key.to_string(),
                language: CaptionLanguage::new(language).unwrap(),
                source,
            },
            TranscriptText::new(text),
            DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
        )
    }
}
