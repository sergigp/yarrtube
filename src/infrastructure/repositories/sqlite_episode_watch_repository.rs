use crate::domain::plex_watch::EpisodeWatch;
use crate::domain::video::{PlaybackPosition, VideoId, WatchProgress};
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use rusqlite::{Connection, params, params_from_iter};

/// The watch state each stored video last agreed on with its Plex episode.
pub trait EpisodeWatchRepository: Send + Sync {
    /// Every row, by video id.
    fn list(&self) -> anyhow::Result<Vec<EpisodeWatch>>;
    /// Inserts or replaces each row by video id, all in one transaction.
    fn save_many(&self, rows: &[EpisodeWatch]) -> anyhow::Result<()>;
    fn delete_many(&self, video_ids: &[VideoId]) -> anyhow::Result<()>;
}

pub struct SqliteEpisodeWatchRepository {
    db: Database,
}

impl SqliteEpisodeWatchRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl EpisodeWatchRepository for SqliteEpisodeWatchRepository {
    fn list(&self) -> anyhow::Result<Vec<EpisodeWatch>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(
                "SELECT video_id, rating_key, viewer_id, agreed_watched, agreed_position_seconds
                 FROM episode_watch ORDER BY video_id",
            )
            .context("failed to prepare episode watch query")?;
        let rows = stmt
            .query_map([], Self::read_row)
            .inspect_err(|e| tracing::error!(error = %e, "failed to query episode watch"))
            .context("failed to query episode watch")?
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to read episode watch row")?;
        rows.into_iter().map(Self::to_episode_watch).collect()
    }

    fn save_many(&self, rows: &[EpisodeWatch]) -> anyhow::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut conn = self.db.write()?;
        let tx = conn
            .transaction()
            .context("failed to begin episode watch save")?;
        rows.iter().try_for_each(|row| Self::upsert(&tx, row))?;
        tx.commit().context("failed to commit episode watch save")
    }

    fn delete_many(&self, video_ids: &[VideoId]) -> anyhow::Result<()> {
        if video_ids.is_empty() {
            return Ok(());
        }
        let placeholders = vec!["?"; video_ids.len()].join(", ");
        let conn = self.db.write()?;
        conn.execute(
            &format!("DELETE FROM episode_watch WHERE video_id IN ({placeholders})"),
            params_from_iter(video_ids.iter().map(|id| id.as_str())),
        )
        .inspect_err(|e| tracing::error!(error = %e, "failed to delete episode watch rows"))
        .context("failed to delete episode watch rows")
        .map(drop)
    }
}

impl SqliteEpisodeWatchRepository {
    fn upsert(conn: &Connection, row: &EpisodeWatch) -> anyhow::Result<()> {
        let (watched, position) = match row.agreed {
            WatchProgress::Unplayed => (false, 0),
            WatchProgress::InProgress(position) => (false, position.seconds()),
            WatchProgress::Watched => (true, 0),
        };
        conn.execute(
            "INSERT INTO episode_watch
                 (video_id, rating_key, viewer_id, agreed_watched, agreed_position_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (video_id) DO UPDATE SET
                 rating_key = excluded.rating_key,
                 viewer_id = excluded.viewer_id,
                 agreed_watched = excluded.agreed_watched,
                 agreed_position_seconds = excluded.agreed_position_seconds",
            params![
                row.video_id.as_str(),
                row.rating_key,
                row.viewer_id,
                watched,
                position
            ],
        )
        .inspect_err(|e| tracing::error!(video_id = %row.video_id, error = %e, "failed to save episode watch"))
        .context("failed to save episode watch")
        .map(drop)
    }

    fn read_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawRow> {
        Ok(RawRow {
            video_id: row.get(0)?,
            rating_key: row.get(1)?,
            viewer_id: row.get(2)?,
            watched: row.get(3)?,
            position_seconds: row.get(4)?,
        })
    }

    fn to_episode_watch(row: RawRow) -> anyhow::Result<EpisodeWatch> {
        let agreed = match (row.watched, row.position_seconds) {
            (true, _) => WatchProgress::Watched,
            (false, 0) => WatchProgress::Unplayed,
            (false, seconds) => WatchProgress::InProgress(
                PlaybackPosition::new(seconds).context("invalid stored agreed position")?,
            ),
        };
        Ok(EpisodeWatch {
            video_id: VideoId::new(row.video_id)?,
            rating_key: row.rating_key,
            viewer_id: row.viewer_id,
            agreed,
        })
    }
}

/// An `episode_watch` row as stored.
struct RawRow {
    video_id: String,
    rating_key: String,
    viewer_id: String,
    watched: bool,
    position_seconds: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;

    #[test]
    fn it_should_round_trip_each_agreed_progress() {
        let db = TestDatabase::new();
        let repository = SqliteEpisodeWatchRepository::new(db.database());
        let rows = vec![
            row("rec1", "101", WatchProgress::Unplayed),
            row(
                "rec2",
                "102",
                WatchProgress::InProgress(PlaybackPosition::new(215).unwrap()),
            ),
            row("rec3", "103", WatchProgress::Watched),
        ];
        repository.save_many(&rows).unwrap();

        let listed = repository.list();

        assert_eq!(listed.map_err(|e| e.to_string()), Ok(rows));
    }

    #[test]
    fn it_should_replace_a_row_on_save() {
        let db = TestDatabase::new();
        let repository = SqliteEpisodeWatchRepository::new(db.database());
        repository
            .save_many(&[row("rec1", "101", WatchProgress::Unplayed)])
            .unwrap();

        repository
            .save_many(&[row("rec1", "201", WatchProgress::Watched)])
            .unwrap();

        assert_eq!(
            repository.list().map_err(|e| e.to_string()),
            Ok(vec![row("rec1", "201", WatchProgress::Watched)])
        );
    }

    #[test]
    fn it_should_delete_rows_by_video_id() {
        let db = TestDatabase::new();
        let repository = SqliteEpisodeWatchRepository::new(db.database());
        repository
            .save_many(&[
                row("rec1", "101", WatchProgress::Watched),
                row("rec2", "102", WatchProgress::Watched),
                row("rec3", "103", WatchProgress::Watched),
            ])
            .unwrap();

        repository
            .delete_many(&[VideoId::new("rec1").unwrap(), VideoId::new("rec3").unwrap()])
            .unwrap();

        assert_eq!(
            repository.list().map_err(|e| e.to_string()),
            Ok(vec![row("rec2", "102", WatchProgress::Watched)])
        );
    }

    fn row(video_id: &str, rating_key: &str, agreed: WatchProgress) -> EpisodeWatch {
        EpisodeWatch {
            video_id: VideoId::new(video_id).unwrap(),
            rating_key: rating_key.to_string(),
            viewer_id: "abc123/viewer".to_string(),
            agreed,
        }
    }
}
