use crate::domain::video::{EpisodeNumber, VideoRecordId};
use crate::domain::video_metadata::{VideoMetadata, render_episode_nfo};
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use chrono::{DateTime, Utc};
#[cfg(test)]
use rusqlite::Connection;
use rusqlite::{OptionalExtension, Row, params};
use std::path::Path;

/// Persists a video's generated metadata: writing its episode NFO at
/// `nfo_path` first, and only recording a `video_metadata` row once that
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
        number: &EpisodeNumber,
        nfo_path: &Path,
    ) -> anyhow::Result<()>;
    fn find(&self, video_id: &VideoRecordId) -> anyhow::Result<Option<VideoMetadata>>;
    /// Writes only the episode NFO at `nfo_path`, recording nothing, so it
    /// can be in place before the video's media file is moved next to it.
    fn write_nfo(
        &self,
        metadata: &VideoMetadata,
        number: &EpisodeNumber,
        nfo_path: &Path,
    ) -> anyhow::Result<()>;
    /// Removes the NFO at `nfo_path`; a missing file is not an error.
    fn remove_nfo(&self, nfo_path: &Path) -> anyhow::Result<()>;
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
    db: Database,
}

impl SqliteVideoMetadataRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl VideoMetadataRepository for SqliteVideoMetadataRepository {
    fn save(
        &self,
        video_id: &VideoRecordId,
        metadata: &VideoMetadata,
        number: &EpisodeNumber,
        nfo_path: &Path,
    ) -> anyhow::Result<()> {
        Self::write_episode_nfo(metadata, number, nfo_path)?;

        let conn = self.db.write()?;
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
        let conn = self.db.read()?;
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

    fn write_nfo(
        &self,
        metadata: &VideoMetadata,
        number: &EpisodeNumber,
        nfo_path: &Path,
    ) -> anyhow::Result<()> {
        Self::write_episode_nfo(metadata, number, nfo_path)
    }

    fn remove_nfo(&self, nfo_path: &Path) -> anyhow::Result<()> {
        match std::fs::remove_file(nfo_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => {
                tracing::error!(path = ?nfo_path, error = %e, "failed to remove NFO");
                Err(anyhow::anyhow!("failed to remove {nfo_path:?}: {e}"))
            }
        }
    }
}

impl SqliteVideoMetadataRepository {
    /// Writes the episode NFO, creating its season folder first.
    fn write_episode_nfo(
        metadata: &VideoMetadata,
        number: &EpisodeNumber,
        nfo_path: &Path,
    ) -> anyhow::Result<()> {
        if let Some(season_dir) = nfo_path.parent() {
            std::fs::create_dir_all(season_dir)
                .map_err(|e| anyhow::anyhow!("failed to create {season_dir:?}: {e}"))?;
        }
        std::fs::write(nfo_path, render_episode_nfo(metadata, number))
            .map_err(|e| anyhow::anyhow!("failed to write {nfo_path:?}: {e}"))
    }

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
    use crate::domain::video::{MOVIE_NFO_FILENAME, VideoId};

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

    const NFO: &str = "S2024E010201 - My Video.nfo";

    fn number() -> EpisodeNumber {
        EpisodeNumber::from_columns(2024, 10201).unwrap()
    }

    fn repo() -> SqliteVideoMetadataRepository {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::shared::sqlite_migrations::apply(&mut conn).unwrap();
        SqliteVideoMetadataRepository::new(Database::single(conn))
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
    fn it_should_write_an_episode_nfo_with_season_episode_and_aired() {
        let repo = repo();
        let show_dir = unique_temp_dir("video-metadata-episode-nfo");
        let nfo_path = show_dir.join("Season 2024").join(NFO);
        let metadata = VideoMetadata {
            title: "Fish & Chips <live>".to_string(),
            ..metadata()
        };

        repo.write_nfo(&metadata, &number(), &nfo_path).unwrap();

        assert_eq!(
            std::fs::read_to_string(&nfo_path).ok(),
            Some(
                concat!(
                    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><episodedetails>"#,
                    "<title>Fish &amp; Chips &lt;live&gt;</title>",
                    "<plot>A description</plot>",
                    "<season>2024</season>",
                    "<episode>10201</episode>",
                    "<aired>2024-01-02</aired>",
                    "<premiered>2024-01-02</premiered>",
                    "<year>2024</year>",
                    "<studio>My Channel</studio>",
                    "<director>My Channel</director>",
                    r#"<uniqueid type="youtube">yt1</uniqueid>"#,
                    "<thumb>My Video.jpg</thumb>",
                    "</episodedetails>",
                )
                .to_string()
            )
        );
        std::fs::remove_dir_all(&show_dir).unwrap();
    }

    #[test]
    fn it_should_leave_no_db_row_when_the_file_write_fails() {
        let repo = repo();
        let dir = unique_temp_dir("video-metadata-unwritable-season");
        std::fs::write(dir.join("Season 2024"), b"not a folder").unwrap();
        let video_id = VideoRecordId::new_generated();

        let result = repo.save(
            &video_id,
            &metadata(),
            &number(),
            &dir.join("Season 2024").join(NFO),
        );

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
        repo.save(&video_id, &metadata(), &number(), &video_dir.join(NFO))
            .unwrap();

        let updated = VideoMetadata {
            title: "Renamed".to_string(),
            ..metadata()
        };
        repo.save(&video_id, &updated, &number(), &video_dir.join(NFO))
            .unwrap();

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

        repo.save(&video_id, &minimal, &number(), &video_dir.join(NFO))
            .unwrap();

        let found = repo.find(&video_id).unwrap().unwrap();
        assert_eq!(found.genre, None);
        assert!(found.tags.is_empty());
        assert_eq!(found.thumb, None);
        std::fs::remove_dir_all(&video_dir).unwrap();
    }

    #[test]
    fn it_should_write_the_nfo_at_the_given_path() {
        let repo = repo();
        let show_dir = unique_temp_dir("video-metadata-write-nfo");
        let nfo_path = show_dir.join("Season 2024").join(NFO);

        repo.write_nfo(&metadata(), &number(), &nfo_path).unwrap();

        assert_eq!(
            std::fs::read_to_string(&nfo_path).ok(),
            Some(render_episode_nfo(&metadata(), &number()))
        );
        assert_eq!(repo.find(&VideoRecordId::new_generated()).unwrap(), None);
        std::fs::remove_dir_all(&show_dir).unwrap();
    }

    #[test]
    fn it_should_remove_the_nfo_at_the_given_path() {
        let repo = repo();
        let season_dir = unique_temp_dir("video-metadata-remove-nfo");
        std::fs::write(season_dir.join(NFO), "<episodedetails/>").unwrap();
        std::fs::write(season_dir.join("S2024E010201 - My Video.jpg"), "jpg").unwrap();

        repo.remove_nfo(&season_dir.join(NFO)).unwrap();

        assert_eq!(
            std::fs::read_dir(&season_dir)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().into_string().unwrap())
                .collect::<Vec<_>>(),
            vec!["S2024E010201 - My Video.jpg".to_string()]
        );
        std::fs::remove_dir_all(&season_dir).unwrap();
    }

    #[test]
    fn it_should_ignore_removing_a_missing_nfo() {
        let repo = repo();
        let video_dir = unique_temp_dir("video-metadata-remove-missing-nfo");

        let result = repo.remove_nfo(&video_dir.join(NFO));

        assert!(result.is_ok());
        std::fs::remove_dir_all(&video_dir).unwrap();
    }

    #[test]
    fn it_should_fail_to_remove_an_nfo_it_cannot_reach() {
        let repo = repo();
        let video_dir = unique_temp_dir("video-metadata-remove-unreachable-nfo");
        std::fs::write(video_dir.join("not-a-folder"), "file").unwrap();
        let nfo_path = video_dir.join("not-a-folder").join(NFO);

        let result = repo.remove_nfo(&nfo_path);

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err(format!(
                "failed to remove {nfo_path:?}: Not a directory (os error 20)"
            ))
        );
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

        repo.save(&video_id, &metadata_for_id, &number(), &video_dir.join(NFO))
            .unwrap();

        let found = repo.find(&video_id).unwrap().unwrap();
        assert_eq!(found.uniqueid, "yt-specific");
        std::fs::remove_dir_all(&video_dir).unwrap();
    }
}
