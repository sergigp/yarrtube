use anyhow::Context;
use rusqlite::Connection;
use rusqlite_migration::{HookResult, M, Migrations};

use crate::domain::category::CategoryColor;

const BASELINE_SQL: &str = include_str!("../../../migrations/0001_baseline.sql");
const WATCH_STATE_SQL: &str = include_str!("../../../migrations/0002_watch_state.sql");
const PUBLISHED_AT_AND_SYNCED_AT_SQL: &str =
    include_str!("../../../migrations/0003_published_at_and_synced_at.sql");
const LAST_ERRORED_AT_SQL: &str = include_str!("../../../migrations/0004_last_errored_at.sql");
const LAST_PLAYED_AT_SQL: &str = include_str!("../../../migrations/0005_last_played_at.sql");
const PLAYLIST_VIDEO_POSITION_REQUIRED_SQL: &str =
    include_str!("../../../migrations/0006_playlist_video_position_required.sql");
const PLAYLIST_EXCLUDE_FROM_HOME_SQL: &str =
    include_str!("../../../migrations/0007_playlist_exclude_from_home.sql");
const EPISODE_NUMBERS_SQL: &str = include_str!("../../../migrations/0008_episode_numbers.sql");
const RECONCILE_INTERVAL_SQL: &str =
    include_str!("../../../migrations/0009_reconcile_interval.sql");
const VIDEO_SUBTITLES_SQL: &str = include_str!("../../../migrations/0010_video_subtitles.sql");
const CATEGORIES_SQL: &str = include_str!("../../../migrations/0011_categories.sql");
const PLAYLIST_HIDE_UNWATCHED_COUNT_SQL: &str =
    include_str!("../../../migrations/0012_playlist_hide_unwatched_count.sql");
const CATEGORY_COLOR_SQL: &str = include_str!("../../../migrations/0013_category_color.sql");
const VIDEO_CATEGORIES_SQL: &str = include_str!("../../../migrations/0014_video_categories.sql");
const PLEX_LABELS_SQL: &str = include_str!("../../../migrations/0015_plex_labels.sql");
const EPISODE_LABELS_GENERATION_SQL: &str =
    include_str!("../../../migrations/0016_episode_labels_generation.sql");
const PREFERENCES_SQL: &str = include_str!("../../../migrations/0017_preferences.sql");
const CATEGORY_COLOR_STRENGTHEN_SQL: &str =
    include_str!("../../../migrations/0018_category_color_strengthen.sql");
const CATEGORY_PALETTE_REFRESH_SQL: &str =
    include_str!("../../../migrations/0019_category_palette_refresh.sql");

pub fn apply(conn: &mut Connection) -> anyhow::Result<()> {
    Migrations::new(vec![
        M::up(BASELINE_SQL),
        M::up(WATCH_STATE_SQL),
        M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
        M::up(LAST_ERRORED_AT_SQL),
        M::up(LAST_PLAYED_AT_SQL),
        M::up(PLAYLIST_VIDEO_POSITION_REQUIRED_SQL),
        M::up(PLAYLIST_EXCLUDE_FROM_HOME_SQL),
        M::up(EPISODE_NUMBERS_SQL),
        M::up(RECONCILE_INTERVAL_SQL),
        M::up(VIDEO_SUBTITLES_SQL),
        M::up(CATEGORIES_SQL),
        M::up(PLAYLIST_HIDE_UNWATCHED_COUNT_SQL),
        M::up(CATEGORY_COLOR_SQL),
        M::up(VIDEO_CATEGORIES_SQL),
        M::up(PLEX_LABELS_SQL),
        M::up(EPISODE_LABELS_GENERATION_SQL),
        M::up(PREFERENCES_SQL),
        M::up_with_hook(
            CATEGORY_COLOR_STRENGTHEN_SQL,
            strengthen_light_category_colors,
        ),
        M::up(CATEGORY_PALETTE_REFRESH_SQL),
    ])
    .to_latest(conn)
    .inspect_err(|e| tracing::error!(error = %e, "failed to apply database migrations"))
    .context("failed to apply database migrations")
}

/// Rewrites every light category color (OKLCH lightness above 0.85) left after
/// the palette mapping to the solid color of the same hue.
fn strengthen_light_category_colors(tx: &rusqlite::Transaction) -> HookResult {
    let colors = tx
        .prepare("SELECT id, color FROM categories")?
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    colors
        .into_iter()
        .filter_map(|(id, color)| {
            let color = CategoryColor::new(color).ok()?;
            let strong = color.clone().strengthened();
            (strong != color).then_some((id, strong))
        })
        .try_for_each(|(id, color)| {
            tx.execute(
                "UPDATE categories SET color = ?1 WHERE id = ?2",
                rusqlite::params![color.as_str(), id],
            )
            .map(|_| ())
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Duration, Utc};

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
                "categories",
                "category_members",
                "category_playlists",
                "channel_videos",
                "channels",
                "domain_events_dead_letter",
                "episode_labels",
                "events",
                "playlist_videos",
                "playlists",
                "preferences",
                "tasks",
                "tasks_dead_letter",
                "video_categories",
                "video_metadata",
                "video_transcripts",
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
    fn it_should_keep_playlist_videos_when_making_position_required() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![
            M::up(BASELINE_SQL),
            M::up(WATCH_STATE_SQL),
            M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
            M::up(LAST_ERRORED_AT_SQL),
            M::up(LAST_PLAYED_AT_SQL),
        ])
        .to_latest(&mut conn)
        .unwrap();
        conn.execute_batch(
            "INSERT INTO playlist_videos (id, playlist_id, video_id, position, created_at)
             VALUES (7, 'PL1', 'rec1', 0, '2024-01-01T00:00:00+00:00');
             INSERT INTO playlist_videos (id, playlist_id, video_id, position, created_at)
             VALUES (9, 'PL1', 'rec2', 1, '2024-01-02T00:00:00+00:00');",
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT id, playlist_id, video_id, position, created_at FROM playlist_videos ORDER BY id",
            )
            .unwrap();
        let rows: Vec<(i64, String, String, i64, String)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![
                (
                    7,
                    "PL1".to_string(),
                    "rec1".to_string(),
                    0,
                    "2024-01-01T00:00:00+00:00".to_string()
                ),
                (
                    9,
                    "PL1".to_string(),
                    "rec2".to_string(),
                    1,
                    "2024-01-02T00:00:00+00:00".to_string()
                ),
            ]
        );
    }

    #[test]
    fn it_should_reject_a_playlist_video_without_position() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply(&mut conn).unwrap();

        let result = conn
            .execute(
                "INSERT INTO playlist_videos (playlist_id, video_id, position, created_at)
                 VALUES ('PL1', 'rec1', NULL, '2024-01-01T00:00:00+00:00')",
                [],
            )
            .map_err(|e| e.to_string());

        assert_eq!(
            result,
            Err("NOT NULL constraint failed: playlist_videos.position".to_string())
        );
    }

    #[test]
    fn it_should_default_existing_playlists_to_shown_on_home_when_migrating() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![
            M::up(BASELINE_SQL),
            M::up(WATCH_STATE_SQL),
            M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
            M::up(LAST_ERRORED_AT_SQL),
            M::up(LAST_PLAYED_AT_SQL),
            M::up(PLAYLIST_VIDEO_POSITION_REQUIRED_SQL),
        ])
        .to_latest(&mut conn)
        .unwrap();
        conn.execute(
            "INSERT INTO playlists (id, name, path, quality, kind, created_at)
             VALUES ('PL1', 'My Playlist', 'my-playlist', 'high', 'youtube_linked', '2024-01-01T00:00:00+00:00')",
            [],
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let exclude_from_home = conn.query_row(
            "SELECT exclude_from_home FROM playlists WHERE id = 'PL1'",
            [],
            |row| row.get::<_, i64>(0),
        );
        assert_eq!(exclude_from_home, Ok(0));
    }

    #[test]
    fn it_should_default_existing_playlists_to_show_their_unwatched_count_when_migrating() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![
            M::up(BASELINE_SQL),
            M::up(WATCH_STATE_SQL),
            M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
            M::up(LAST_ERRORED_AT_SQL),
            M::up(LAST_PLAYED_AT_SQL),
            M::up(PLAYLIST_VIDEO_POSITION_REQUIRED_SQL),
            M::up(PLAYLIST_EXCLUDE_FROM_HOME_SQL),
            M::up(EPISODE_NUMBERS_SQL),
            M::up(RECONCILE_INTERVAL_SQL),
            M::up(VIDEO_SUBTITLES_SQL),
        ])
        .to_latest(&mut conn)
        .unwrap();
        conn.execute(
            "INSERT INTO playlists (id, name, path, quality, kind, created_at)
             VALUES ('PL1', 'My Playlist', 'my-playlist', 'high', 'youtube_linked', '2024-01-01T00:00:00+00:00')",
            [],
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let hide_unwatched_count = conn.query_row(
            "SELECT hide_unwatched_count FROM playlists WHERE id = 'PL1'",
            [],
            |row| row.get::<_, i64>(0),
        );
        assert_eq!(hide_unwatched_count, Ok(0));
    }

    #[test]
    fn it_should_backfill_existing_categories_with_palette_colors_in_creation_order() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![
            M::up(BASELINE_SQL),
            M::up(WATCH_STATE_SQL),
            M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
            M::up(LAST_ERRORED_AT_SQL),
            M::up(LAST_PLAYED_AT_SQL),
            M::up(PLAYLIST_VIDEO_POSITION_REQUIRED_SQL),
            M::up(PLAYLIST_EXCLUDE_FROM_HOME_SQL),
            M::up(EPISODE_NUMBERS_SQL),
            M::up(RECONCILE_INTERVAL_SQL),
            M::up(VIDEO_SUBTITLES_SQL),
            M::up(CATEGORIES_SQL),
            M::up(PLAYLIST_HIDE_UNWATCHED_COUNT_SQL),
        ])
        .to_latest(&mut conn)
        .unwrap();
        // Inserted out of creation order, named so name order differs too.
        let created_minutes = [2, 0, 1, 3, 4, 5, 6, 7, 8, 9, 10];
        created_minutes.iter().for_each(|minute| {
            conn.execute(
                "INSERT INTO categories (id, name, created_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![
                    format!("category-{minute:02}"),
                    format!("Category {}", 10 - minute),
                    format!("2024-01-01T00:{minute:02}:00+00:00"),
                ],
            )
            .unwrap();
        });

        apply(&mut conn).unwrap();

        let mut stmt = conn
            .prepare("SELECT color FROM categories ORDER BY id")
            .unwrap();
        let colors: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            colors,
            vec![
                "#9f8401", "#bd7123", "#c8645b", "#bd6396", "#9870c5", "#6a80d4", "#0193bf",
                "#019b8d", "#79922c", "#978364", "#9f8401",
            ]
        );
    }

    #[test]
    fn it_should_strengthen_a_former_palette_color() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate_to_preferences(&mut conn);
        insert_colored_category(&conn, "Chess", "#d3ecf8");

        apply(&mut conn).unwrap();

        assert_eq!(category_colors(&conn), vec!["#0193bf"]);
    }

    #[test]
    fn it_should_strengthen_a_custom_pastel_keeping_its_hue() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate_to_preferences(&mut conn);
        insert_colored_category(&conn, "Garden", "#d9f3dd");

        apply(&mut conn).unwrap();

        assert_eq!(category_colors(&conn), vec!["#429b5a"]);
    }

    #[test]
    fn it_should_leave_an_already_strong_color_unchanged() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate_to_preferences(&mut conn);
        insert_colored_category(&conn, "Sports", "#c8645b");

        apply(&mut conn).unwrap();

        assert_eq!(category_colors(&conn), vec!["#c8645b"]);
    }

    #[test]
    fn it_should_refresh_a_former_solid_palette_color() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate_to_category_color_strengthen(&mut conn);
        insert_colored_category(&conn, "Chess", "#0193bf");

        apply(&mut conn).unwrap();

        assert_eq!(category_colors(&conn), vec!["#0ea5e9"]);
    }

    #[test]
    fn it_should_refresh_every_former_solid_palette_color_by_position() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate_to_category_color_strengthen(&mut conn);
        let former = [
            "#9f8401", "#bd7123", "#c8645b", "#bd6396", "#9870c5", "#6a80d4", "#0193bf", "#019b8d",
            "#79922c", "#978364",
        ];
        former
            .iter()
            .enumerate()
            .for_each(|(index, color)| insert_colored_category(&conn, &format!("c{index}"), color));

        apply(&mut conn).unwrap();

        assert_eq!(
            category_colors(&conn),
            vec![
                "#f59e0b", "#f97316", "#ef4444", "#ec4899", "#8b5cf6", "#3b82f6", "#0ea5e9",
                "#14b8a6", "#22c55e", "#64748b",
            ]
        );
    }

    #[test]
    fn it_should_backfill_video_categories_from_members() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![
            M::up(BASELINE_SQL),
            M::up(WATCH_STATE_SQL),
            M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
            M::up(LAST_ERRORED_AT_SQL),
            M::up(LAST_PLAYED_AT_SQL),
            M::up(PLAYLIST_VIDEO_POSITION_REQUIRED_SQL),
            M::up(PLAYLIST_EXCLUDE_FROM_HOME_SQL),
            M::up(EPISODE_NUMBERS_SQL),
            M::up(RECONCILE_INTERVAL_SQL),
            M::up(VIDEO_SUBTITLES_SQL),
            M::up(CATEGORIES_SQL),
            M::up(PLAYLIST_HIDE_UNWATCHED_COUNT_SQL),
            M::up(CATEGORY_COLOR_SQL),
        ])
        .to_latest(&mut conn)
        .unwrap();
        conn.execute_batch(
            "INSERT INTO channel_videos (channel_id, video_id, position, created_at)
             VALUES ('@GothamChess', 'chess-rec1', 0, '2024-01-01T00:00:00+00:00'),
                    ('@GothamChess', 'chess-rec2', 1, '2024-01-01T00:00:00+00:00'),
                    ('@Uncategorized', 'other-rec1', 0, '2024-01-01T00:00:00+00:00');
             INSERT INTO playlist_videos (playlist_id, video_id, position, created_at)
             VALUES ('PLendgames', 'endgames-rec1', 0, '2024-01-01T00:00:00+00:00'),
                    ('PLuntracked', 'untracked-rec1', 0, '2024-01-01T00:00:00+00:00');
             INSERT INTO category_members (category_id, member_kind, member_id)
             VALUES ('chess', 'channel', '@GothamChess'),
                    ('chess', 'playlist', 'PLendgames'),
                    ('funny', 'channel', '@GothamChess');",
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT video_id, category_id FROM video_categories ORDER BY video_id, category_id",
            )
            .unwrap();
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let expected = [
            ("chess-rec1", "chess"),
            ("chess-rec1", "funny"),
            ("chess-rec2", "chess"),
            ("chess-rec2", "funny"),
            ("endgames-rec1", "chess"),
        ]
        .map(|(video_id, category_id)| (video_id.to_string(), category_id.to_string()));
        assert_eq!(rows, expected.to_vec());
    }

    #[test]
    fn it_should_default_existing_videos_to_missing_subtitles() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![
            M::up(BASELINE_SQL),
            M::up(WATCH_STATE_SQL),
            M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
            M::up(LAST_ERRORED_AT_SQL),
            M::up(LAST_PLAYED_AT_SQL),
            M::up(PLAYLIST_VIDEO_POSITION_REQUIRED_SQL),
            M::up(PLAYLIST_EXCLUDE_FROM_HOME_SQL),
            M::up(EPISODE_NUMBERS_SQL),
            M::up(RECONCILE_INTERVAL_SQL),
        ])
        .to_latest(&mut conn)
        .unwrap();
        conn.execute(
            "INSERT INTO videos (id, youtube_id, title, status, created_at, updated_at)
             VALUES ('rec1', 'yt1', 'My Video', 'DOWNLOADED', '2024-01-01T00:00:00+00:00', '2024-01-01T00:00:00+00:00')",
            [],
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let subtitles = conn.query_row(
            "SELECT subtitle_state, subtitle_filename FROM videos WHERE id = 'rec1'",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        );
        assert_eq!(subtitles, Ok(("MISSING".to_string(), None)));
    }

    #[test]
    fn it_should_drop_queued_plex_collection_reconciles_when_migrating() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![
            M::up(BASELINE_SQL),
            M::up(WATCH_STATE_SQL),
            M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
            M::up(LAST_ERRORED_AT_SQL),
            M::up(LAST_PLAYED_AT_SQL),
            M::up(PLAYLIST_VIDEO_POSITION_REQUIRED_SQL),
            M::up(PLAYLIST_EXCLUDE_FROM_HOME_SQL),
        ])
        .to_latest(&mut conn)
        .unwrap();
        conn.execute(
            "INSERT INTO tasks (task_type, payload, status, run_at, created_at, updated_at)
             VALUES ('reconcile_plex_collections', '{}', 'pending', '2024-01-01T00:00:00+00:00', '2024-01-01T00:00:00+00:00', '2024-01-01T00:00:00+00:00'),
                    ('reconcile_channel', '{}', 'pending', '2024-01-01T00:00:00+00:00', '2024-01-01T00:00:00+00:00', '2024-01-01T00:00:00+00:00')",
            [],
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let mut stmt = conn.prepare("SELECT task_type FROM tasks").unwrap();
        let task_types: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(task_types, vec!["reconcile_channel"]);
    }

    #[test]
    fn it_should_add_the_episode_number_columns() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![
            M::up(BASELINE_SQL),
            M::up(WATCH_STATE_SQL),
            M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
            M::up(LAST_ERRORED_AT_SQL),
            M::up(LAST_PLAYED_AT_SQL),
            M::up(PLAYLIST_VIDEO_POSITION_REQUIRED_SQL),
            M::up(PLAYLIST_EXCLUDE_FROM_HOME_SQL),
        ])
        .to_latest(&mut conn)
        .unwrap();
        conn.execute(
            "INSERT INTO videos (id, youtube_id, title, status, created_at, updated_at)
             VALUES ('rec1', 'yt1', 'My Video', 'DOWNLOADED', '2024-01-01T00:00:00+00:00', '2024-01-01T00:00:00+00:00')",
            [],
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let episode_number = conn.query_row(
            "SELECT season, episode FROM videos WHERE id = 'rec1'",
            [],
            |row| Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, Option<i64>>(1)?)),
        );
        assert_eq!(episode_number, Ok((None, None)));
    }

    #[test]
    fn it_should_number_existing_playlist_videos_by_position() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![
            M::up(BASELINE_SQL),
            M::up(WATCH_STATE_SQL),
            M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
            M::up(LAST_ERRORED_AT_SQL),
            M::up(LAST_PLAYED_AT_SQL),
            M::up(PLAYLIST_VIDEO_POSITION_REQUIRED_SQL),
            M::up(PLAYLIST_EXCLUDE_FROM_HOME_SQL),
        ])
        .to_latest(&mut conn)
        .unwrap();
        conn.execute_batch(
            "INSERT INTO playlists (id, name, path, quality, kind, created_at)
             VALUES ('PL1', 'Full', 'full', 'high', 'youtube_linked', '2024-01-01T00:00:00+00:00');
             INSERT INTO playlists (id, name, path, quality, kind, created_at)
             VALUES ('PL2', 'Empty', 'empty', 'high', 'youtube_linked', '2024-01-01T00:00:00+00:00');
             INSERT INTO videos (id, youtube_id, title, status, created_at, updated_at)
             VALUES ('rec1', 'yt1', 'Last', 'DOWNLOADED', '2024-01-01T00:00:00+00:00', '2024-01-01T00:00:00+00:00');
             INSERT INTO videos (id, youtube_id, title, status, created_at, updated_at)
             VALUES ('rec2', 'yt2', 'Tied first', 'PENDING', '2024-01-01T00:00:00+00:00', '2024-01-01T00:00:00+00:00');
             INSERT INTO videos (id, youtube_id, title, status, created_at, updated_at)
             VALUES ('rec3', 'yt3', 'Tied second', 'DOWNLOADED', '2024-01-01T00:00:00+00:00', '2024-01-01T00:00:00+00:00');
             INSERT INTO videos (id, youtube_id, title, status, created_at, updated_at)
             VALUES ('rec4', 'yt4', 'Channel video', 'DOWNLOADED', '2024-01-01T00:00:00+00:00', '2024-01-01T00:00:00+00:00');
             INSERT INTO playlist_videos (playlist_id, video_id, position, created_at)
             VALUES ('PL1', 'rec1', 1, '2024-01-01T00:00:00+00:00');
             INSERT INTO playlist_videos (playlist_id, video_id, position, created_at)
             VALUES ('PL1', 'rec2', 0, '2024-01-01T00:00:00+00:00');
             INSERT INTO playlist_videos (playlist_id, video_id, position, created_at)
             VALUES ('PL1', 'rec3', 0, '2024-01-01T00:00:00+00:00');
             INSERT INTO channel_videos (channel_id, video_id, position, created_at)
             VALUES ('UC1', 'rec4', 0, '2024-01-01T00:00:00+00:00');",
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let mut videos = conn
            .prepare("SELECT id, season, episode FROM videos ORDER BY id")
            .unwrap();
        let episode_numbers: Vec<(String, Option<i64>, Option<i64>)> = videos
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            episode_numbers,
            vec![
                ("rec1".to_string(), Some(1), Some(3)),
                ("rec2".to_string(), Some(1), Some(1)),
                ("rec3".to_string(), Some(1), Some(2)),
                ("rec4".to_string(), None, None),
            ]
        );
        let mut playlists = conn
            .prepare("SELECT id, last_episode FROM playlists ORDER BY id")
            .unwrap();
        let last_episodes: Vec<(String, Option<i64>)> = playlists
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            last_episodes,
            vec![("PL1".to_string(), Some(3)), ("PL2".to_string(), None)]
        );
    }

    #[test]
    fn it_should_give_existing_channels_and_playlists_the_default_interval() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![
            M::up(BASELINE_SQL),
            M::up(WATCH_STATE_SQL),
            M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
            M::up(LAST_ERRORED_AT_SQL),
            M::up(LAST_PLAYED_AT_SQL),
            M::up(PLAYLIST_VIDEO_POSITION_REQUIRED_SQL),
            M::up(PLAYLIST_EXCLUDE_FROM_HOME_SQL),
            M::up(EPISODE_NUMBERS_SQL),
        ])
        .to_latest(&mut conn)
        .unwrap();
        conn.execute_batch(
            "INSERT INTO channels (id, name, youtube_channel_id, quality, video_limit, path, created_at)
             VALUES ('@somechannel', 'Some Channel', 'UC123', 'high', 10, 'creators/somechannel', '2024-01-01T00:00:00+00:00');
             INSERT INTO playlists (id, name, path, quality, kind, created_at)
             VALUES ('PL1', 'My Playlist', 'my-playlist', 'high', 'youtube_linked', '2024-01-01T00:00:00+00:00');",
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let intervals = conn.query_row(
            "SELECT (SELECT reconcile_interval_minutes FROM channels WHERE id = '@somechannel'),
                    (SELECT reconcile_interval_minutes FROM playlists WHERE id = 'PL1')",
            [],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        );
        assert_eq!(intervals, Ok((180, 180)));
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

    #[test]
    fn it_should_backfill_the_last_played_time_of_part_watched_videos_when_migrating() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![
            M::up(BASELINE_SQL),
            M::up(WATCH_STATE_SQL),
            M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
            M::up(LAST_ERRORED_AT_SQL),
        ])
        .to_latest(&mut conn)
        .unwrap();
        conn.execute_batch(
            "INSERT INTO videos (id, youtube_id, title, status, created_at, updated_at, watched_at, playback_position_seconds)
             VALUES ('rec1', 'yt1', 'Part Watched', 'DOWNLOADED', '2024-01-01T00:00:00+00:00', '2024-01-02T00:00:00+00:00', NULL, 120);
             INSERT INTO videos (id, youtube_id, title, status, created_at, updated_at, watched_at, playback_position_seconds)
             VALUES ('rec2', 'yt2', 'Watched', 'DOWNLOADED', '2024-01-01T00:00:00+00:00', '2024-01-03T00:00:00+00:00', '2024-01-03T00:00:00+00:00', 50);
             INSERT INTO videos (id, youtube_id, title, status, created_at, updated_at, watched_at, playback_position_seconds)
             VALUES ('rec3', 'yt3', 'Not Started', 'DOWNLOADED', '2024-01-01T00:00:00+00:00', '2024-01-04T00:00:00+00:00', NULL, 0);",
        )
        .unwrap();

        let before = Utc::now() - Duration::seconds(1);

        apply(&mut conn).unwrap();

        let after = Utc::now();

        let mut stmt = conn
            .prepare("SELECT id, last_played_at FROM videos ORDER BY id")
            .unwrap();
        let played_at_migration: Vec<(String, Option<bool>)> = stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get::<_, Option<String>>(1)?))
            })
            .unwrap()
            .map(|row| {
                let (id, last_played_at) = row.unwrap();
                let at_migration = last_played_at.map(|value| {
                    let played_at = DateTime::parse_from_rfc3339(&value).unwrap();
                    played_at >= before && played_at <= after
                });
                (id, at_migration)
            })
            .collect();
        assert_eq!(
            played_at_migration,
            vec![
                ("rec1".to_string(), Some(true)),
                ("rec2".to_string(), None),
                ("rec3".to_string(), None),
            ]
        );
    }

    #[test]
    fn it_should_backfill_the_publish_time_from_premiered_when_migrating() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![M::up(BASELINE_SQL), M::up(WATCH_STATE_SQL)])
            .to_latest(&mut conn)
            .unwrap();
        conn.execute(
            "INSERT INTO video_metadata (video_id, title, plot, studio, director, premiered, year, tags, uniqueid, sorttitle, created_at)
             VALUES ('rec1', 'My Video', 'A description', 'My Channel', 'My Channel', '2024-01-02', 2024, '[]', 'yt1', '20240102 My Video', '2024-02-01T00:00:00+00:00')",
            [],
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let published_at = conn.query_row(
            "SELECT published_at FROM video_metadata WHERE video_id = 'rec1'",
            [],
            |row| row.get::<_, String>(0),
        );
        assert_eq!(published_at, Ok("2024-01-02T00:00:00+00:00".to_string()));
    }

    #[test]
    fn it_should_backfill_the_metadata_update_time_from_its_creation_time_when_migrating() {
        let mut conn = Connection::open_in_memory().unwrap();
        Migrations::new(vec![M::up(BASELINE_SQL), M::up(WATCH_STATE_SQL)])
            .to_latest(&mut conn)
            .unwrap();
        conn.execute(
            "INSERT INTO video_metadata (video_id, title, plot, studio, director, premiered, year, tags, uniqueid, sorttitle, created_at)
             VALUES ('rec1', 'My Video', 'A description', 'My Channel', 'My Channel', '2024-01-02', 2024, '[]', 'yt1', '20240102 My Video', '2024-02-01T00:00:00+00:00')",
            [],
        )
        .unwrap();

        apply(&mut conn).unwrap();

        let timestamps = conn.query_row(
            "SELECT created_at, updated_at FROM video_metadata WHERE video_id = 'rec1'",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        );
        assert_eq!(
            timestamps,
            Ok((
                "2024-02-01T00:00:00+00:00".to_string(),
                "2024-02-01T00:00:00+00:00".to_string()
            ))
        );
    }
    fn migrate_to_preferences(conn: &mut Connection) {
        Migrations::new(vec![
            M::up(BASELINE_SQL),
            M::up(WATCH_STATE_SQL),
            M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
            M::up(LAST_ERRORED_AT_SQL),
            M::up(LAST_PLAYED_AT_SQL),
            M::up(PLAYLIST_VIDEO_POSITION_REQUIRED_SQL),
            M::up(PLAYLIST_EXCLUDE_FROM_HOME_SQL),
            M::up(EPISODE_NUMBERS_SQL),
            M::up(RECONCILE_INTERVAL_SQL),
            M::up(VIDEO_SUBTITLES_SQL),
            M::up(CATEGORIES_SQL),
            M::up(PLAYLIST_HIDE_UNWATCHED_COUNT_SQL),
            M::up(CATEGORY_COLOR_SQL),
            M::up(VIDEO_CATEGORIES_SQL),
            M::up(PLEX_LABELS_SQL),
            M::up(EPISODE_LABELS_GENERATION_SQL),
            M::up(PREFERENCES_SQL),
        ])
        .to_latest(conn)
        .unwrap();
    }

    fn migrate_to_category_color_strengthen(conn: &mut Connection) {
        Migrations::new(vec![
            M::up(BASELINE_SQL),
            M::up(WATCH_STATE_SQL),
            M::up(PUBLISHED_AT_AND_SYNCED_AT_SQL),
            M::up(LAST_ERRORED_AT_SQL),
            M::up(LAST_PLAYED_AT_SQL),
            M::up(PLAYLIST_VIDEO_POSITION_REQUIRED_SQL),
            M::up(PLAYLIST_EXCLUDE_FROM_HOME_SQL),
            M::up(EPISODE_NUMBERS_SQL),
            M::up(RECONCILE_INTERVAL_SQL),
            M::up(VIDEO_SUBTITLES_SQL),
            M::up(CATEGORIES_SQL),
            M::up(PLAYLIST_HIDE_UNWATCHED_COUNT_SQL),
            M::up(CATEGORY_COLOR_SQL),
            M::up(VIDEO_CATEGORIES_SQL),
            M::up(PLEX_LABELS_SQL),
            M::up(EPISODE_LABELS_GENERATION_SQL),
            M::up(PREFERENCES_SQL),
            M::up_with_hook(
                CATEGORY_COLOR_STRENGTHEN_SQL,
                strengthen_light_category_colors,
            ),
        ])
        .to_latest(conn)
        .unwrap();
    }

    fn insert_colored_category(conn: &Connection, name: &str, color: &str) {
        conn.execute(
            "INSERT INTO categories (id, name, created_at, color) VALUES (?1, ?1, ?2, ?3)",
            rusqlite::params![name, "2024-01-01T00:00:00+00:00", color],
        )
        .unwrap();
    }

    fn category_colors(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT color FROM categories ORDER BY id")
            .unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }
}
