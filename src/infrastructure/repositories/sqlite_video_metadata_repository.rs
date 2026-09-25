use crate::domain::video::VideoRecordId;
use crate::domain::video_metadata::{VideoMetadata, render_movie_nfo};
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params};
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

/// A `video_metadata` row as read, before its values are parsed into a
/// `VideoMetadata`.
struct VideoMetadataRow {
    title: String,
    plot: String,
    studio: String,
    director: String,
    published_at: String,
    genre: Option<String>,
    tags: String,
    uniqueid: String,
    thumb: Option<String>,
    sorttitle: String,
    created_at: String,
    updated_at: String,
}

pub struct SqliteVideoMetadataRepository {
    conn: Mutex<Connection>,
}

impl SqliteVideoMetadataRepository {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
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
            "INSERT INTO video_metadata (video_id, title, plot, studio, director, published_at, genre, tags, uniqueid, thumb, sorttitle, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT (video_id) DO UPDATE SET
                title = excluded.title,
                plot = excluded.plot,
                studio = excluded.studio,
                director = excluded.director,
                published_at = excluded.published_at,
                genre = excluded.genre,
                tags = excluded.tags,
                uniqueid = excluded.uniqueid,
                thumb = excluded.thumb,
                sorttitle = excluded.sorttitle,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at",
            params![
                video_id.as_str(),
                metadata.title,
                metadata.plot,
                metadata.studio,
                metadata.director,
                metadata.published_at.to_rfc3339(),
                metadata.genre,
                tags,
                metadata.uniqueid,
                metadata.thumb,
                metadata.sorttitle,
                metadata.created_at.to_rfc3339(),
                metadata.updated_at.to_rfc3339(),
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
            "SELECT title, plot, studio, director, published_at, genre, tags, uniqueid, thumb, sorttitle, created_at, updated_at
             FROM video_metadata WHERE video_id = ?1",
            params![video_id.as_str()],
            Self::read_row,
        )
        .optional()
        .inspect_err(|e| tracing::error!(video_id = %video_id, error = %e, "failed to find video metadata"))
        .context("failed to find video metadata")?
        .map(Self::row_to_video_metadata)
        .transpose()
    }
}

impl SqliteVideoMetadataRepository {
    fn read_row(row: &Row<'_>) -> rusqlite::Result<VideoMetadataRow> {
        Ok(VideoMetadataRow {
            title: row.get(0)?,
            plot: row.get(1)?,
            studio: row.get(2)?,
            director: row.get(3)?,
            published_at: row.get(4)?,
            genre: row.get(5)?,
            tags: row.get(6)?,
            uniqueid: row.get(7)?,
            thumb: row.get(8)?,
            sorttitle: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    }

    fn row_to_video_metadata(row: VideoMetadataRow) -> anyhow::Result<VideoMetadata> {
        Ok(VideoMetadata {
            title: row.title,
            plot: row.plot,
            studio: row.studio,
            director: row.director,
            published_at: Self::parse_timestamp(&row.published_at, "published_at")?,
            genre: row.genre,
            tags: serde_json::from_str(&row.tags)
                .context("failed to deserialize stored video metadata tags")?,
            uniqueid: row.uniqueid,
            thumb: row.thumb,
            sorttitle: row.sorttitle,
            created_at: Self::parse_timestamp(&row.created_at, "created_at")?,
            updated_at: Self::parse_timestamp(&row.updated_at, "updated_at")?,
        })
    }

    fn parse_timestamp(value: &str, column: &str) -> anyhow::Result<DateTime<Utc>> {
        Ok(DateTime::parse_from_rfc3339(value)
            .with_context(|| format!("failed to parse stored {column}"))?
            .with_timezone(&Utc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::video::VideoId;

    fn metadata() -> VideoMetadata {
        VideoMetadata::new(
            "My Video",
            "A description",
            "My Channel",
            "My Channel",
            DateTime::parse_from_rfc3339("2024-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&Utc),
            Some("Music".to_string()),
            vec!["tag1".to_string(), "tag2".to_string()],
            "yt1",
            Some("My Video.jpg".to_string()),
            "0001 My Video",
            DateTime::<Utc>::UNIX_EPOCH,
        )
    }

    fn repo() -> SqliteVideoMetadataRepository {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::shared::sqlite_migrations::apply(&mut conn).unwrap();
        SqliteVideoMetadataRepository::new(conn)
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
