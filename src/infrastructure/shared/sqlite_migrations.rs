use anyhow::Context;
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

const BASELINE_SQL: &str = include_str!("../../../migrations/0001_baseline.sql");
const WATCH_STATE_SQL: &str = include_str!("../../../migrations/0002_watch_state.sql");
const PUBLISHED_AT_AND_SYNCED_AT_SQL: &str =
    include_str!("../../../migrations/0003_published_at_and_synced_at.sql");

pub fn apply(conn: &mut Connection) -> anyhow::Result<()> {
    Migrations::new(vec![
        M::up(BASELINE_SQL),
        M::up(WATCH_STATE_SQL),
        M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
    ])
    .to_latest(conn)
    .inspect_err(|e| tracing::error!(error = %e, "failed to apply database migrations"))
    .context("failed to apply database migrations")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_create_every_table_on_a_fresh_in_memory_database() {
        let mut conn = Connection::open_in_memory().unwrap();

        apply(&mut conn).unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
            )
            .unwrap();
        let mut tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        tables.sort();

        assert_eq!(
            tables,
            vec![
                "channel_videos",
                "channels",
                "domain_events_dead_letter",
                "events",
                "playlist_videos",
                "playlists",
                "tasks",
                "tasks_dead_letter",
                "video_metadata",
                "videos",
            ]
        );
    }

    #[test]
    fn it_should_be_a_no_op_when_applied_twice() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply(&mut conn).unwrap();

        apply(&mut conn).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'channels'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn it_should_recognize_an_already_deployed_database_as_up_to_date_without_touching_its_data() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(BASELINE_SQL).unwrap();
        conn.execute(
            "INSERT INTO channels (id, name, youtube_channel_id, quality, video_limit, path, created_at)
             VALUES ('@somechannel', 'Some Channel', 'UC123', 'high', 10, 'creators/somechannel', '2024-01-01T00:00:00+00:00')",
            [],
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let name: String = conn
            .query_row(
                "SELECT name FROM channels WHERE id = '@somechannel'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(name, "Some Channel");
    }

    #[test]
    fn it_should_default_existing_videos_to_unwatched_when_migrating() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![M::up(BASELINE_SQL)])
            .to_latest(&mut conn)
            .unwrap();
        conn.execute(
            "INSERT INTO videos (id, youtube_id, title, status, created_at, updated_at)
             VALUES ('rec1', 'yt1', 'My Video', 'DOWNLOADED', '2024-01-01T00:00:00+00:00', '2024-01-01T00:00:00+00:00')",
            [],
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let watch_state = conn.query_row(
            "SELECT watched_at, playback_position_seconds FROM videos WHERE id = 'rec1'",
            [],
            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)?)),
        );
        assert_eq!(watch_state, Ok((None, 0)));
    }

    #[test]
    fn it_should_backfill_the_sync_time_of_downloaded_videos_when_migrating() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![M::up(BASELINE_SQL), M::up(WATCH_STATE_SQL)])
            .to_latest(&mut conn)
            .unwrap();
        conn.execute_batch(
            "INSERT INTO videos (id, youtube_id, title, status, created_at, updated_at)
             VALUES ('rec1', 'yt1', 'Downloaded Video', 'DOWNLOADED', '2024-01-01T00:00:00+00:00', '2024-01-02T00:00:00+00:00');
             INSERT INTO videos (id, youtube_id, title, status, created_at, updated_at)
             VALUES ('rec2', 'yt2', 'Pending Video', 'PENDING', '2024-01-01T00:00:00+00:00', '2024-01-03T00:00:00+00:00');",
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let mut stmt = conn
            .prepare("SELECT id, synced_at FROM videos ORDER BY id")
            .unwrap();
        let synced_at: Vec<(String, Option<String>)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            synced_at,
            vec![
                (
                    "rec1".to_string(),
                    Some("2024-01-02T00:00:00+00:00".to_string())
                ),
                ("rec2".to_string(), None),
            ]
        );
    }
}
