use crate::domain::shared::VideoRecordId;
use crate::domain::video_metadata::{VideoMetadata, render_movie_nfo};
use anyhow::Context;
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::sync::Mutex;

pub const MOVIE_NFO_FILENAME: &str = "movie.nfo";

/// Persists a video's generated metadata: writing `movie.nfo` into its
/// output folder first, and only recording a `video_metadata` row once that
/// write succeeds. The row is the sole source of truth for "has this
/// video's metadata been generated" — `find` never inspects the filesystem,
/// so a process crash between the file write and the DB write simply looks
/// unpopulated and gets harmlessly regenerated (overwriting the file again)
/// on the next reconcile pass. See design.md's "VideoMetadataRepository:
/// one domain-facing port, one composed implementation" decision.
pub trait VideoMetadataRepository: Send + Sync {
    fn save(
        &self,
        video_id: &VideoRecordId,
        metadata: &VideoMetadata,
        video_dir: &Path,
    ) -> anyhow::Result<()>;
    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoMetadata>>;
}

pub struct SqliteVideoMetadataRepository {
    conn: Mutex<Connection>,
}

impl SqliteVideoMetadataRepository {
    pub fn new(conn: Connection) -> anyhow::Result<Self> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS video_metadata (
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
            )",
            [],
        )
        .inspect_err(|e| tracing::error!(error = %e, "failed to create video_metadata table"))
        .context("failed to create video_metadata table")?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn write_movie_nfo(metadata: &VideoMetadata, video_dir: &Path) -> anyhow::Result<()> {
        let content = render_movie_nfo(metadata);
        std::fs::write(video_dir.join(MOVIE_NFO_FILENAME), content).map_err(|e| {
            anyhow::anyhow!("failed to write {MOVIE_NFO_FILENAME} in {video_dir:?}: {e}")
        })
    }
}

impl VideoMetadataRepository for SqliteVideoMetadataRepository {
    fn save(
        &self,
        video_id: &VideoRecordId,
        metadata: &VideoMetadata,
        video_dir: &Path,
    ) -> anyhow::Result<()> {
        Self::write_movie_nfo(metadata, video_dir)?;

        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(video_id = %video_id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        let tags = serde_json::to_string(&metadata.tags)
            .context("failed to serialize video metadata tags")?;
        conn.execute(
            "INSERT INTO video_metadata (video_id, title, plot, studio, director, premiered, year, genre, tags, uniqueid, thumb, sorttitle, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT (video_id) DO UPDATE SET
                title = excluded.title,
                plot = excluded.plot,
                studio = excluded.studio,
                director = excluded.director,
                premiered = excluded.premiered,
                year = excluded.year,
                genre = excluded.genre,
                tags = excluded.tags,
                uniqueid = excluded.uniqueid,
                thumb = excluded.thumb,
                sorttitle = excluded.sorttitle,
                created_at = excluded.created_at",
            params![
                video_id.as_str(),
                metadata.title,
                metadata.plot,
                metadata.studio,
                metadata.director,
                metadata.premiered,
                metadata.year,
                metadata.genre,
                tags,
                metadata.uniqueid,
                metadata.thumb,
                metadata.sorttitle,
                Utc::now().to_rfc3339(),
            ],
        )
        .inspect_err(|e| {
            tracing::error!(video_id = %video_id, error = %e, "failed to save video metadata")
        })
        .context("failed to save video metadata")?;
        Ok(())
    }

    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoMetadata>> {
        let conn = self
            .conn
            .lock()
            .inspect_err(|_| tracing::error!(video_id = %video_id, "database lock poisoned"))
            .map_err(|_| anyhow::anyhow!("database lock poisoned"))?;
        conn.query_row(
            "SELECT title, plot, studio, director, premiered, year, genre, tags, uniqueid, thumb, sorttitle
             FROM video_metadata WHERE video_id = ?1",
            params![video_id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i32>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, String>(10)?,
                ))
            },
        )
        .optional()
        .inspect_err(|e| tracing::error!(video_id = %video_id, error = %e, "failed to find video metadata"))
        .context("failed to find video metadata")?
        .map(|(title, plot, studio, director, premiered, year, genre, tags, uniqueid, thumb, sorttitle)| {
            let tags: Vec<String> = serde_json::from_str(&tags)
                .context("failed to deserialize stored video metadata tags")?;
            Ok(VideoMetadata::new(
                title, plot, studio, director, premiered, year, genre, tags, uniqueid, thumb, sorttitle,
            ))
        })
        .transpose()
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeVideoMetadataRepository {
    #[allow(clippy::type_complexity)]
    pub(crate) entries: Mutex<Vec<(VideoRecordId, VideoMetadata)>>,
}

#[cfg(test)]
impl VideoMetadataRepository for FakeVideoMetadataRepository {
    fn save(
        &self,
        video_id: &VideoRecordId,
        metadata: &VideoMetadata,
        _video_dir: &Path,
    ) -> anyhow::Result<()> {
        let mut entries = self.entries.lock().unwrap();
        if let Some(existing) = entries.iter_mut().find(|(id, _)| id == video_id) {
            existing.1 = metadata.clone();
        } else {
            entries.push((video_id.clone(), metadata.clone()));
        }
        Ok(())
    }

    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoMetadata>> {
        Ok(self
            .entries
            .lock()
            .unwrap()
            .iter()
            .find(|(id, _)| id == video_id)
            .map(|(_, metadata)| metadata.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::shared::VideoId;

    fn metadata() -> VideoMetadata {
        VideoMetadata::new(
            "My Video",
            "A description",
            "My Channel",
            "My Channel",
            "2024-01-02",
            2024,
            Some("Music".to_string()),
            vec!["tag1".to_string(), "tag2".to_string()],
            "yt1",
            Some("My Video.jpg".to_string()),
            "0001 My Video",
        )
    }

    fn repo() -> SqliteVideoMetadataRepository {
        SqliteVideoMetadataRepository::new(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn unique_temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "yarrtube-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn it_should_write_movie_nfo_and_record_a_row_on_save() {
        let repo = repo();
        let video_dir = unique_temp_dir("video-metadata-save");
        let video_id = VideoRecordId::new_generated();

        repo.save(&video_id, &metadata(), &video_dir).unwrap();

        assert!(video_dir.join(MOVIE_NFO_FILENAME).is_file());
        let found = repo.find(&video_id).unwrap().unwrap();
        assert_eq!(found.title, "My Video");
        assert_eq!(found.tags, vec!["tag1".to_string(), "tag2".to_string()]);
        std::fs::remove_dir_all(&video_dir).unwrap();
    }

    #[test]
    fn it_should_leave_no_db_row_when_the_file_write_fails() {
        let repo = repo();
        let missing_dir = unique_temp_dir("video-metadata-missing-parent").join("does-not-exist");
        let video_id = VideoRecordId::new_generated();

        let result = repo.save(&video_id, &metadata(), &missing_dir);

        assert!(result.is_err());
        assert!(repo.find(&video_id).unwrap().is_none());
    }

    #[test]
    fn it_should_return_none_when_no_row_exists_even_if_movie_nfo_is_present_on_disk() {
        let repo = repo();
        let video_dir = unique_temp_dir("video-metadata-orphan-file");
        std::fs::write(video_dir.join(MOVIE_NFO_FILENAME), "<movie></movie>").unwrap();
        let video_id = VideoRecordId::new_generated();

        let found = repo.find(&video_id).unwrap();

        assert!(found.is_none());
        std::fs::remove_dir_all(&video_dir).unwrap();
    }

    #[test]
    fn it_should_overwrite_an_existing_row_on_a_second_save() {
        let repo = repo();
        let video_dir = unique_temp_dir("video-metadata-overwrite");
        let video_id = VideoRecordId::new_generated();
        repo.save(&video_id, &metadata(), &video_dir).unwrap();

        let updated = VideoMetadata {
            title: "Renamed".to_string(),
            ..metadata()
        };
        repo.save(&video_id, &updated, &video_dir).unwrap();

        let found = repo.find(&video_id).unwrap().unwrap();
        assert_eq!(found.title, "Renamed");
        std::fs::remove_dir_all(&video_dir).unwrap();
    }

    #[test]
    fn it_should_round_trip_a_video_with_no_genre_no_tags_and_no_thumbnail() {
        let repo = repo();
        let video_dir = unique_temp_dir("video-metadata-minimal");
        let video_id = VideoRecordId::new_generated();
        let minimal = VideoMetadata {
            genre: None,
            tags: Vec::new(),
            thumb: None,
            ..metadata()
        };

        repo.save(&video_id, &minimal, &video_dir).unwrap();

        let found = repo.find(&video_id).unwrap().unwrap();
        assert_eq!(found.genre, None);
        assert!(found.tags.is_empty());
        assert_eq!(found.thumb, None);
        std::fs::remove_dir_all(&video_dir).unwrap();
    }

    #[test]
    fn it_should_return_none_when_finding_an_untracked_video() {
        let repo = repo();

        let found = repo.find(&VideoRecordId::new_generated()).unwrap();

        assert!(found.is_none());
    }

    #[test]
    fn it_should_scope_metadata_lookup_by_youtube_id() {
        let repo = repo();
        let video_dir = unique_temp_dir("video-metadata-youtube-id");
        let video_id = VideoRecordId::new_generated();
        let metadata_for_id = VideoMetadata {
            uniqueid: VideoId::new("yt-specific").unwrap().as_str().to_string(),
            ..metadata()
        };

        repo.save(&video_id, &metadata_for_id, &video_dir).unwrap();

        let found = repo.find(&video_id).unwrap().unwrap();
        assert_eq!(found.uniqueid, "yt-specific");
        std::fs::remove_dir_all(&video_dir).unwrap();
    }
}
