use crate::domain::playlist::PlaylistId;
use crate::domain::playlist::{Playlist, PlaylistKind, PlaylistName, PlaylistPath};
use crate::domain::shared::{Quality, ReconcileInterval};
use crate::domain::video::PlaylistEpisodeNumber;
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use chrono::{DateTime, Utc};
#[cfg(test)]
use rusqlite::Connection;
use rusqlite::{OptionalExtension, Row, params};

pub trait PlaylistRepository: Send + Sync {
    fn find(&self, id: &PlaylistId) -> anyhow::Result<Option<Playlist>>;
    fn insert(&self, playlist: &Playlist) -> anyhow::Result<()>;
    /// Writes every field but `last_episode`: a full-row write from a stale
    /// read (e.g. the home-visibility toggle racing a reconcile that numbers
    /// new videos) must not move the counter back and reuse an episode.
    fn update(&self, playlist: &Playlist) -> anyhow::Result<()>;
    /// Raises the playlist's episode counter to `number`; never lowers it.
    fn record_last_episode(
        &self,
        id: &PlaylistId,
        number: PlaylistEpisodeNumber,
    ) -> anyhow::Result<()>;
    fn delete(&self, id: &PlaylistId) -> anyhow::Result<()>;
    fn list(&self) -> anyhow::Result<Vec<Playlist>>;
}

pub struct SqlitePlaylistRepository {
    db: Database,
}

impl SqlitePlaylistRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl PlaylistRepository for SqlitePlaylistRepository {
    fn find(&self, id: &PlaylistId) -> anyhow::Result<Option<Playlist>> {
        let conn = self.db.read()?;
        conn.query_row(
            &format!("SELECT {PLAYLIST_COLUMNS} FROM playlists WHERE id = ?1"),
            params![id.as_str()],
            Self::read_row,
        )
        .optional()
        .inspect_err(|e| tracing::error!(playlist_id = %id, error = %e, "failed to query playlist"))
        .context("failed to query playlist")?
        .map(Self::row_to_playlist)
        .transpose()
    }

    fn insert(&self, playlist: &Playlist) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            "INSERT INTO playlists (id, name, path, quality, kind, exclude_from_home, created_at, last_episode, reconcile_interval_minutes, hide_unwatched_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                playlist.id.as_str(),
                playlist.name.as_str(),
                playlist.path.as_str(),
                playlist.quality.as_str(),
                playlist.kind.as_str(),
                playlist.exclude_from_home,
                playlist.created_at.to_rfc3339(),
                playlist.last_episode.map(|number| number.episode()),
                playlist.reconcile_interval.minutes(),
                playlist.hide_unwatched_count,
            ],
        )
        .inspect_err(|e| tracing::error!(playlist_id = %playlist.id, error = %e, "failed to insert playlist"))
        .context("failed to insert playlist")?;
        Ok(())
    }

    fn update(&self, playlist: &Playlist) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            "UPDATE playlists SET name = ?2, path = ?3, quality = ?4, kind = ?5, exclude_from_home = ?6, created_at = ?7, reconcile_interval_minutes = ?8, hide_unwatched_count = ?9 WHERE id = ?1",
            params![
                playlist.id.as_str(),
                playlist.name.as_str(),
                playlist.path.as_str(),
                playlist.quality.as_str(),
                playlist.kind.as_str(),
                playlist.exclude_from_home,
                playlist.created_at.to_rfc3339(),
                playlist.reconcile_interval.minutes(),
                playlist.hide_unwatched_count,
            ],
        )
        .inspect_err(|e| tracing::error!(playlist_id = %playlist.id, error = %e, "failed to update playlist"))
        .context("failed to update playlist")?;
        Ok(())
    }

    fn record_last_episode(
        &self,
        id: &PlaylistId,
        number: PlaylistEpisodeNumber,
    ) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            "UPDATE playlists SET last_episode = MAX(COALESCE(last_episode, 0), ?2) WHERE id = ?1",
            params![id.as_str(), number.episode()],
        )
        .inspect_err(
            |e| tracing::error!(playlist_id = %id, error = %e, "failed to record the last episode"),
        )
        .context("failed to record the last episode")?;
        Ok(())
    }

    fn delete(&self, id: &PlaylistId) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute("DELETE FROM playlists WHERE id = ?1", params![id.as_str()])
            .inspect_err(
                |e| tracing::error!(playlist_id = %id, error = %e, "failed to delete playlist"),
            )
            .context("failed to delete playlist")?;
        Ok(())
    }

    fn list(&self) -> anyhow::Result<Vec<Playlist>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(&format!(
                "SELECT {PLAYLIST_COLUMNS} FROM playlists ORDER BY name COLLATE NOCASE, rowid"
            ))
            .inspect_err(|e| tracing::error!(error = %e, "failed to prepare list query"))
            .context("failed to prepare list query")?;
        let rows = stmt
            .query_map([], Self::read_row)
            .inspect_err(|e| tracing::error!(error = %e, "failed to list playlists"))
            .context("failed to list playlists")?;

        rows.map(|row| {
            row.inspect_err(|e| tracing::error!(error = %e, "failed to read playlist row"))
                .context("failed to read playlist row")
                .and_then(Self::row_to_playlist)
        })
        .collect()
    }
}

impl SqlitePlaylistRepository {
    fn read_row(row: &Row<'_>) -> rusqlite::Result<PlaylistRow> {
        Ok(PlaylistRow {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            quality: row.get(3)?,
            kind: row.get(4)?,
            exclude_from_home: row.get(5)?,
            created_at: row.get(6)?,
            last_episode: row.get(7)?,
            reconcile_interval: row.get(8)?,
            hide_unwatched_count: row.get(9)?,
        })
    }

    fn row_to_playlist(row: PlaylistRow) -> anyhow::Result<Playlist> {
        Ok(Playlist {
            id: PlaylistId::new(row.id)?,
            name: PlaylistName::new(row.name)?,
            path: PlaylistPath::new(row.path)?,
            quality: Quality::new(row.quality)?,
            kind: PlaylistKind::new(row.kind)?,
            exclude_from_home: row.exclude_from_home,
            hide_unwatched_count: row.hide_unwatched_count,
            reconcile_interval: ReconcileInterval::new(row.reconcile_interval)?,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .context("failed to parse stored created_at")?
                .with_timezone(&Utc),
            last_episode: row
                .last_episode
                .map(PlaylistEpisodeNumber::new)
                .transpose()?,
        })
    }
}

const PLAYLIST_COLUMNS: &str = "id, name, path, quality, kind, exclude_from_home, created_at, last_episode, reconcile_interval_minutes, hide_unwatched_count";

struct PlaylistRow {
    id: String,
    name: String,
    path: String,
    quality: String,
    kind: String,
    exclude_from_home: bool,
    created_at: String,
    last_episode: Option<u32>,
    reconcile_interval: i64,
    hide_unwatched_count: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> SqlitePlaylistRepository {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::shared::sqlite_migrations::apply(&mut conn).unwrap();
        SqlitePlaylistRepository::new(Database::single(conn))
    }

    fn playlist(id: &str, name: &str) -> Playlist {
        Playlist::create(
            PlaylistId::new(id).unwrap(),
            PlaylistName::new(name).unwrap(),
            PlaylistPath::new("my-playlist").unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            false,
            ReconcileInterval::default_interval(),
            DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        )
    }

    #[test]
    fn it_should_return_none_when_the_playlist_does_not_exist() {
        let repo = repo();
        let id = PlaylistId::new("PL404").unwrap();

        assert_eq!(repo.find(&id).unwrap(), None);
    }

    #[test]
    fn it_should_return_the_playlist_after_inserting_it() {
        let repo = repo();
        let playlist = playlist("PL1", "First");

        repo.insert(&playlist).unwrap();

        assert_eq!(repo.find(&playlist.id).unwrap(), Some(playlist));
    }

    #[test]
    fn it_should_return_the_exclude_from_home_of_an_inserted_playlist() {
        let repo = repo();
        let playlist = Playlist {
            exclude_from_home: true,
            ..playlist("PL1", "First")
        };

        repo.insert(&playlist).unwrap();

        assert_eq!(repo.find(&playlist.id).unwrap(), Some(playlist));
    }

    #[test]
    fn it_should_round_trip_the_playlists_reconcile_interval() {
        let repo = repo();
        let playlist =
            playlist("PL1", "First").with_reconcile_interval(ReconcileInterval::new(30).unwrap());
        repo.insert(&playlist).unwrap();
        let updated = playlist.with_reconcile_interval(ReconcileInterval::new(1440).unwrap());

        repo.update(&updated).unwrap();

        assert_eq!(repo.find(&updated.id).unwrap(), Some(updated));
    }

    #[test]
    fn it_should_round_trip_the_playlists_hide_unwatched_count() {
        let repo = repo();
        let playlist = playlist("PL1", "First").with_hide_unwatched_count(true);
        repo.insert(&playlist).unwrap();
        assert_eq!(repo.find(&playlist.id).unwrap(), Some(playlist.clone()));
        let updated = playlist.with_hide_unwatched_count(false);

        repo.update(&updated).unwrap();

        assert_eq!(repo.find(&updated.id).unwrap(), Some(updated));
    }

    #[test]
    fn it_should_update_an_existing_playlist() {
        let repo = repo();
        let original = playlist("PL1", "First");
        repo.insert(&original).unwrap();
        let updated = original.with_exclude_from_home(true);

        repo.update(&updated).unwrap();

        assert_eq!(repo.find(&updated.id).unwrap(), Some(updated));
    }

    #[test]
    fn it_should_raise_the_last_episode() {
        let repo = repo();
        let inserted = Playlist {
            last_episode: Some(PlaylistEpisodeNumber::new(3).unwrap()),
            ..playlist("PL1", "First")
        };
        repo.insert(&inserted).unwrap();

        repo.record_last_episode(&inserted.id, PlaylistEpisodeNumber::new(5).unwrap())
            .unwrap();

        let raised = Playlist {
            last_episode: Some(PlaylistEpisodeNumber::new(5).unwrap()),
            ..inserted
        };
        assert_eq!(
            (repo.find(&raised.id).unwrap(), repo.list().unwrap()),
            (Some(raised.clone()), vec![raised])
        );
    }

    #[test]
    fn it_should_never_lower_the_last_episode() {
        let repo = repo();
        let inserted = Playlist {
            last_episode: Some(PlaylistEpisodeNumber::new(5).unwrap()),
            ..playlist("PL1", "First")
        };
        repo.insert(&inserted).unwrap();

        repo.record_last_episode(&inserted.id, PlaylistEpisodeNumber::new(4).unwrap())
            .unwrap();

        assert_eq!(repo.find(&inserted.id).unwrap(), Some(inserted));
    }

    #[test]
    fn it_should_keep_the_last_episode_when_updating_from_a_stale_read() {
        let repo = repo();
        let stale = playlist("PL1", "First");
        repo.insert(&stale).unwrap();
        repo.record_last_episode(&stale.id, PlaylistEpisodeNumber::new(6).unwrap())
            .unwrap();

        repo.update(&stale.clone().with_exclude_from_home(true))
            .unwrap();

        assert_eq!(
            repo.find(&stale.id).unwrap(),
            Some(Playlist {
                exclude_from_home: true,
                last_episode: Some(PlaylistEpisodeNumber::new(6).unwrap()),
                ..stale
            })
        );
    }

    #[test]
    fn it_should_fail_when_inserting_a_duplicate_id() {
        let repo = repo();
        let playlist = playlist("PL1", "First");
        repo.insert(&playlist).unwrap();

        assert!(repo.insert(&playlist).is_err());
    }

    #[test]
    fn it_should_remove_an_existing_playlist() {
        let repo = repo();
        let playlist = playlist("PL1", "First");
        repo.insert(&playlist).unwrap();

        repo.delete(&playlist.id).unwrap();

        assert_eq!(repo.find(&playlist.id).unwrap(), None);
    }

    #[test]
    fn it_should_succeed_when_deleting_a_missing_playlist() {
        let repo = repo();
        let id = PlaylistId::new("PL404").unwrap();

        assert!(repo.delete(&id).is_ok());
    }

    #[test]
    fn it_should_return_an_empty_list_when_no_playlists_are_saved() {
        let repo = repo();

        assert_eq!(repo.list().unwrap(), Vec::new());
    }

    #[test]
    fn it_should_return_all_saved_playlists() {
        let repo = repo();
        let first = playlist("PL1", "First");
        let second = playlist("PL2", "Second");
        repo.insert(&first).unwrap();
        repo.insert(&second).unwrap();

        assert_eq!(repo.list().unwrap(), vec![first, second]);
    }
}
