use crate::domain::video::VideoRecordId;
use crate::domain::video_transcript::VideoTranscript;
use crate::infrastructure::shared::sqlite_connection::Database;

/// Stores each video's plain-text transcript, at most one per video.
pub trait VideoTranscriptRepository: Send + Sync {
    /// Inserts the transcript, replacing the video's previous one.
    fn save(&self, transcript: &VideoTranscript) -> anyhow::Result<()>;
    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoTranscript>>;
    fn delete(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
}

pub struct SqliteVideoTranscriptRepository {
    #[allow(dead_code)]
    db: Database,
}

impl SqliteVideoTranscriptRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl VideoTranscriptRepository for SqliteVideoTranscriptRepository {
    fn save(&self, _transcript: &VideoTranscript) -> anyhow::Result<()> {
        Ok(())
    }

    fn find(&self, _video_id: &VideoRecordId) -> anyhow::Result<Option<VideoTranscript>> {
        Ok(None)
    }

    fn delete(&self, _video_id: &VideoRecordId) -> anyhow::Result<()> {
        Ok(())
    }
}
