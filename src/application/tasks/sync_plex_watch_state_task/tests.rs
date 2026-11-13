use super::*;
use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
use crate::domain::channel_video::ChannelVideo;
use crate::domain::playlist::{Playlist, PlaylistId, PlaylistKind, PlaylistName, PlaylistPath};
use crate::domain::playlist_video::PlaylistVideo;
use crate::domain::plex::PlexWatchState;
use crate::domain::plex_watch::EpisodeWatch;
use crate::domain::services::plex_watch_syncer::BATCH_SIZE;
use crate::domain::services::{
    CategoryVideoCollector, PlexEpisodeFinder, VideoOwnerFinder, VideoWatchStateUpdater,
};
use crate::domain::shared::{Quality, ReconcileInterval};
use crate::domain::task::{ScheduledTask, TaskStatus};
use crate::domain::video::{PlaybackPosition, Video, WatchProgress, WatchState, YoutubeVideoId};
use crate::infrastructure::repositories::plex_library_repository::FakePlexLibraryRepository;
use crate::infrastructure::repositories::sqlite_category_repository::SqliteCategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::{
    ChannelRepository, SqliteChannelRepository,
};
use crate::infrastructure::repositories::sqlite_channel_video_repository::{
    ChannelVideoRepository, SqliteChannelVideoRepository,
};
use crate::infrastructure::repositories::sqlite_episode_watch_repository::{
    EpisodeWatchRepository, SqliteEpisodeWatchRepository,
};
use crate::infrastructure::repositories::sqlite_playlist_repository::{
    PlaylistRepository, SqlitePlaylistRepository,
};
use crate::infrastructure::repositories::sqlite_playlist_video_repository::{
    PlaylistVideoRepository, SqlitePlaylistVideoRepository,
};
use crate::infrastructure::repositories::sqlite_task_repository::{
    SqliteTaskRepository, TaskRepository,
};
use crate::infrastructure::repositories::sqlite_video_categories_repository::SqliteVideoCategoriesRepository;
use crate::infrastructure::repositories::sqlite_video_repository::{
    SqliteVideoRepository, VideoRepository,
};
use crate::infrastructure::shared::sqlite_connection::TestDatabase;
use crate::infrastructure::shared::system_clock::FixedClock;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

#[test]
fn it_should_push_a_video_watched_in_yarrtube() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[]),
    );
    let video = seed_video(
        &db,
        downloaded_video("yt1", "S2023E1114 - One.mp4").mark_watched(an_hour_ago()),
    );
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(plex.mutations(), vec!["scrobble:101".to_string()]);
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        vec![agreed(&video, "101", WatchProgress::Watched)]
    );
}

#[test]
fn it_should_pull_an_episode_watched_in_plex() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
            .with_episode_watch("101", plex_watched(an_hour_ago())),
    );
    let video = seed_video(&db, downloaded_video("yt1", "S2023E1114 - One.mp4"));
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        watch_state(&db, &video),
        WatchState {
            watched_at: Some(an_hour_ago()),
            position: PlaybackPosition::start(),
            last_played_at: Some(an_hour_ago()),
        }
    );
    assert_eq!(plex.mutations(), Vec::<String>::new());
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        vec![agreed(&video, "101", WatchProgress::Watched)]
    );
}

#[test]
fn it_should_push_a_playback_position() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[]),
    );
    let video = seed_video(
        &db,
        Video {
            watch: in_progress(300, an_hour_ago()),
            ..downloaded_video("yt1", "S2023E1114 - One.mp4")
        },
    );
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(plex.mutations(), vec!["progress:101:300000".to_string()]);
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        vec![agreed(
            &video,
            "101",
            WatchProgress::InProgress(position(300))
        )]
    );
}

#[test]
fn it_should_pull_a_playback_position() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
            .with_episode_watch("101", plex_in_progress(215_540, an_hour_ago())),
    );
    let video = seed_video(&db, downloaded_video("yt1", "S2023E1114 - One.mp4"));
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(watch_state(&db, &video), in_progress(215, an_hour_ago()));
    assert_eq!(plex.mutations(), Vec::<String>::new());
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        vec![agreed(
            &video,
            "101",
            WatchProgress::InProgress(position(215))
        )]
    );
}

#[test]
fn it_should_unwatch_an_episode_before_saving_a_rewatch_position() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
            .with_episode_watch("101", plex_watched(two_hours_ago())),
    );
    let video = seed_video(
        &db,
        Video {
            watch: in_progress(600, an_hour_ago()),
            ..downloaded_video("yt1", "S2023E1114 - One.mp4")
        },
    );
    episode_watch_repository
        .save_many(&[agreed(&video, "101", WatchProgress::Watched)])
        .unwrap();
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        plex.mutations(),
        vec![
            "unscrobble:101".to_string(),
            "progress:101:600000".to_string()
        ]
    );
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        vec![agreed(
            &video,
            "101",
            WatchProgress::InProgress(position(600))
        )]
    );
}

#[test]
fn it_should_pull_an_episode_unwatched_in_plex() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[]),
    );
    let video = seed_video(
        &db,
        downloaded_video("yt1", "S2023E1114 - One.mp4").mark_watched(two_hours_ago()),
    );
    episode_watch_repository
        .save_many(&[agreed(&video, "101", WatchProgress::Watched)])
        .unwrap();
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(watch_state(&db, &video), WatchState::unwatched());
    assert_eq!(plex.mutations(), Vec::<String>::new());
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        vec![agreed(&video, "101", WatchProgress::Unplayed)]
    );
}

#[test]
fn it_should_push_a_video_unplayed_after_being_watched() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
            .with_episode_watch("101", plex_watched(two_hours_ago())),
    );
    let video = seed_video(&db, downloaded_video("yt1", "S2023E1114 - One.mp4"));
    episode_watch_repository
        .save_many(&[agreed(&video, "101", WatchProgress::Watched)])
        .unwrap();
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(plex.mutations(), vec!["unscrobble:101".to_string()]);
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        vec![agreed(&video, "101", WatchProgress::Unplayed)]
    );
}

#[test]
fn it_should_bring_every_copy_in_line_with_a_plex_change() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_location("21", "/plex/videos/music")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
            .with_episode_watch("101", plex_watched(an_hour_ago()))
            .with_episode("21", "201", &playlist_plex_file("S01E001 - One.mp4"), &[])
            .with_episode_watch("201", plex_in_progress(60_000, two_hours_ago())),
    );
    let channel_copy = seed_video(&db, downloaded_video("yt1", "S2023E1114 - One.mp4"));
    let playlist_copy = seed_playlist_video(&db, "yt1", "S01E001 - One.mp4");
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    let watched = WatchState {
        watched_at: Some(an_hour_ago()),
        position: PlaybackPosition::start(),
        last_played_at: Some(an_hour_ago()),
    };
    assert_eq!(
        (
            watch_state(&db, &channel_copy),
            watch_state(&db, &playlist_copy)
        ),
        (watched, watched)
    );
    assert_eq!(plex.mutations(), vec!["scrobble:201".to_string()]);
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        by_video_id(vec![
            agreed(&channel_copy, "101", WatchProgress::Watched),
            agreed(&playlist_copy, "201", WatchProgress::Watched),
        ])
    );
}

#[test]
fn it_should_not_echo_a_pushed_change() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .at(now())
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[]),
    );
    let video = seed_video(
        &db,
        Video {
            watch: in_progress(300, an_hour_ago()),
            ..downloaded_video("yt1", "S2023E1114 - One.mp4")
        },
    );
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));
    run(&task).unwrap();

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(plex.mutations(), vec!["progress:101:300000".to_string()]);
    assert_eq!(watch_state(&db, &video), in_progress(300, an_hour_ago()));
}

#[test]
fn it_should_keep_the_most_recently_played_side_when_both_changed() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
            .with_episode_watch("101", plex_watched(an_hour_ago()))
            .with_episode("19", "102", &plex_file("S2023E1115 - Two.mp4"), &[])
            .with_episode_watch("102", plex_watched(two_hours_ago())),
    );
    let played_later_in_plex = seed_video(
        &db,
        Video {
            watch: in_progress(120, two_hours_ago()),
            ..downloaded_video("yt1", "S2023E1114 - One.mp4")
        },
    );
    let played_later_in_yarrtube = seed_video(
        &db,
        Video {
            watch: in_progress(120, an_hour_ago()),
            ..downloaded_video("yt2", "S2023E1115 - Two.mp4")
        },
    );
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        (
            watch_state(&db, &played_later_in_plex),
            watch_state(&db, &played_later_in_yarrtube)
        ),
        (
            WatchState {
                watched_at: Some(an_hour_ago()),
                position: PlaybackPosition::start(),
                last_played_at: Some(an_hour_ago()),
            },
            in_progress(120, an_hour_ago())
        )
    );
    assert_eq!(
        plex.mutations(),
        vec![
            "unscrobble:102".to_string(),
            "progress:102:120000".to_string()
        ]
    );
}

#[test]
fn it_should_seed_from_whichever_side_was_played_on_the_first_sync() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
            .with_episode_watch("101", plex_watched(an_hour_ago()))
            .with_episode("19", "102", &plex_file("S2023E1115 - Two.mp4"), &[])
            .with_episode("19", "103", &plex_file("S2023E1116 - Three.mp4"), &[]),
    );
    let watched_in_plex = seed_video(&db, downloaded_video("yt1", "S2023E1114 - One.mp4"));
    let watched_in_yarrtube = seed_video(
        &db,
        downloaded_video("yt2", "S2023E1115 - Two.mp4").mark_watched(an_hour_ago()),
    );
    let never_played = seed_video(&db, downloaded_video("yt3", "S2023E1116 - Three.mp4"));
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        watch_state(&db, &watched_in_plex).progress(),
        WatchProgress::Watched
    );
    assert_eq!(plex.mutations(), vec!["scrobble:102".to_string()]);
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        by_video_id(vec![
            agreed(&watched_in_plex, "101", WatchProgress::Watched),
            agreed(&watched_in_yarrtube, "102", WatchProgress::Watched),
            agreed(&never_played, "103", WatchProgress::Unplayed),
        ])
    );
}

#[test]
fn it_should_count_a_short_rewatch_in_plex_as_still_watched() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
            .with_episode_watch(
                "101",
                PlexWatchState {
                    view_count: 1,
                    ..plex_in_progress(120_000, an_hour_ago())
                },
            ),
    );
    let video = seed_video(
        &db,
        downloaded_video("yt1", "S2023E1114 - One.mp4").mark_watched(two_hours_ago()),
    );
    episode_watch_repository
        .save_many(&[agreed(&video, "101", WatchProgress::Watched)])
        .unwrap();
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        watch_state(&db, &video),
        WatchState::unwatched().mark_watched(two_hours_ago())
    );
    assert_eq!(plex.mutations(), Vec::<String>::new());
}

#[test]
fn it_should_skip_a_video_plex_has_not_imported() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex =
        Arc::new(FakePlexLibraryRepository::default().with_location("19", "/plex/videos/creators"));
    seed_video(
        &db,
        downloaded_video("yt1", "S2023E1114 - One.mp4").mark_watched(an_hour_ago()),
    );
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(plex.mutations(), Vec::<String>::new());
    assert_eq!(episode_watch_repository.list().unwrap(), vec![]);
}

#[test]
fn it_should_push_to_an_episode_plex_reimported() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[]),
    );
    let video = seed_video(
        &db,
        downloaded_video("yt1", "S2023E1114 - One.mp4").mark_watched(two_hours_ago()),
    );
    episode_watch_repository
        .save_many(&[agreed(&video, "99", WatchProgress::Watched)])
        .unwrap();
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(plex.mutations(), vec!["scrobble:101".to_string()]);
    assert_eq!(
        watch_state(&db, &video),
        WatchState::unwatched().mark_watched(two_hours_ago())
    );
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        vec![agreed(&video, "101", WatchProgress::Watched)]
    );
}

#[test]
fn it_should_forget_a_video_no_longer_downloaded() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex =
        Arc::new(FakePlexLibraryRepository::default().with_location("19", "/plex/videos/creators"));
    let video = seed_video(
        &db,
        Video::create(YoutubeVideoId::new("yt1").unwrap(), "yt1", now()),
    );
    episode_watch_repository
        .save_many(&[agreed(&video, "101", WatchProgress::Watched)])
        .unwrap();
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(episode_watch_repository.list().unwrap(), vec![]);
}

#[test]
fn it_should_push_the_others_if_plex_rejects_one() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
            .with_episode("19", "102", &plex_file("S2023E1115 - Two.mp4"), &[])
            .rejecting("101"),
    );
    seed_video(
        &db,
        downloaded_video("yt1", "S2023E1114 - One.mp4").mark_watched(an_hour_ago()),
    );
    let accepted = seed_video(
        &db,
        downloaded_video("yt2", "S2023E1115 - Two.mp4").mark_watched(an_hour_ago()),
    );
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(plex.mutations(), vec!["scrobble:102".to_string()]);
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        vec![agreed(&accepted, "102", WatchProgress::Watched)]
    );
}

#[test]
fn it_should_retry_later_if_plex_is_unreachable() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let task_repository = task_repository(&db);
    let video = seed_video(
        &db,
        downloaded_video("yt1", "S2023E1114 - One.mp4").mark_watched(an_hour_ago()),
    );
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        Arc::new(FakePlexLibraryRepository::failing()),
        episode_watch_repository.clone(),
        task_repository.clone(),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        watch_state(&db, &video),
        WatchState::unwatched().mark_watched(an_hour_ago())
    );
    assert_eq!(episode_watch_repository.list().unwrap(), vec![]);
    assert_eq!(
        task_repository.list_non_completed().unwrap(),
        vec![pending_sync(1, now() + Duration::minutes(5))]
    );
}

#[test]
fn it_should_schedule_the_next_sync_in_five_minutes() {
    let db = TestDatabase::new();
    let task_repository = task_repository(&db);
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        Arc::new(FakePlexLibraryRepository::default().with_location("19", "/plex/videos/creators")),
        Arc::new(SqliteEpisodeWatchRepository::new(db.database())),
        task_repository.clone(),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        task_repository.list_non_completed().unwrap(),
        vec![pending_sync(1, now() + Duration::minutes(5))]
    );
}

#[test]
fn it_should_schedule_the_next_sync_now_if_pushes_exceeded_the_batch() {
    let db = TestDatabase::new();
    let task_repository = task_repository(&db);
    let files: Vec<String> = (0..=BATCH_SIZE).map(|i| format!("V{i}.mp4")).collect();
    let plex = Arc::new(files.iter().enumerate().fold(
        FakePlexLibraryRepository::default().with_location("19", "/plex/videos/creators"),
        |plex, (i, file)| plex.with_episode("19", &i.to_string(), &plex_file(file), &[]),
    ));
    files.iter().enumerate().for_each(|(i, file)| {
        seed_video(
            &db,
            downloaded_video(&format!("yt{i}"), file).mark_watched(an_hour_ago()),
        );
    });
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        Arc::new(SqliteEpisodeWatchRepository::new(db.database())),
        task_repository.clone(),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(plex.mutations().len(), BATCH_SIZE);
    assert_eq!(
        task_repository.list_non_completed().unwrap(),
        vec![pending_sync(1, now())]
    );
}

#[test]
fn it_should_count_an_episode_stopped_under_a_second_as_unplayed() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
            .with_episode_watch("101", plex_in_progress(500, an_hour_ago())),
    );
    let video = seed_video(&db, downloaded_video("yt1", "S2023E1114 - One.mp4"));
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(watch_state(&db, &video), WatchState::unwatched());
    assert_eq!(plex.mutations(), Vec::<String>::new());
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        vec![agreed(&video, "101", WatchProgress::Unplayed)]
    );
}

#[test]
fn it_should_pull_a_long_rewatch_in_plex_as_in_progress() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
            .with_episode_watch(
                "101",
                PlexWatchState {
                    view_count: 1,
                    ..plex_in_progress(600_000, an_hour_ago())
                },
            ),
    );
    let video = seed_video(
        &db,
        downloaded_video("yt1", "S2023E1114 - One.mp4").mark_watched(two_hours_ago()),
    );
    episode_watch_repository
        .save_many(&[agreed(&video, "101", WatchProgress::Watched)])
        .unwrap();
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(watch_state(&db, &video), in_progress(600, an_hour_ago()));
    assert_eq!(plex.mutations(), Vec::<String>::new());
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        vec![agreed(
            &video,
            "101",
            WatchProgress::InProgress(position(600))
        )]
    );
}

#[test]
fn it_should_keep_the_yarrtube_side_if_both_changed_at_the_same_time() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
            .with_episode_watch("101", plex_watched(an_hour_ago())),
    );
    let video = seed_video(
        &db,
        Video {
            watch: in_progress(120, an_hour_ago()),
            ..downloaded_video("yt1", "S2023E1114 - One.mp4")
        },
    );
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(watch_state(&db, &video), in_progress(120, an_hour_ago()));
    assert_eq!(
        plex.mutations(),
        vec![
            "unscrobble:101".to_string(),
            "progress:101:120000".to_string()
        ]
    );
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        vec![agreed(
            &video,
            "101",
            WatchProgress::InProgress(position(120))
        )]
    );
}

#[test]
fn it_should_schedule_the_next_sync_in_five_minutes_if_pushes_filled_the_batch() {
    let db = TestDatabase::new();
    let task_repository = task_repository(&db);
    let files: Vec<String> = (0..BATCH_SIZE).map(|i| format!("V{i}.mp4")).collect();
    let plex = Arc::new(files.iter().enumerate().fold(
        FakePlexLibraryRepository::default().with_location("19", "/plex/videos/creators"),
        |plex, (i, file)| plex.with_episode("19", &i.to_string(), &plex_file(file), &[]),
    ));
    files.iter().enumerate().for_each(|(i, file)| {
        seed_video(
            &db,
            downloaded_video(&format!("yt{i}"), file).mark_watched(an_hour_ago()),
        );
    });
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        Arc::new(SqliteEpisodeWatchRepository::new(db.database())),
        task_repository.clone(),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(
        plex.mutations(),
        (0..BATCH_SIZE)
            .map(|i| format!("scrobble:{i}"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        task_repository.list_non_completed().unwrap(),
        vec![pending_sync(1, now() + Duration::minutes(5))]
    );
}

#[test]
fn it_should_bring_every_copy_in_line_with_the_latest_plex_change_if_listed_last() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_location("21", "/plex/videos/music")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
            .with_episode_watch("101", plex_in_progress(60_000, two_hours_ago()))
            .with_episode("21", "201", &playlist_plex_file("S01E001 - One.mp4"), &[])
            .with_episode_watch("201", plex_watched(an_hour_ago())),
    );
    let channel_copy = seed_video(&db, downloaded_video("yt1", "S2023E1114 - One.mp4"));
    let playlist_copy = seed_playlist_video(&db, "yt1", "S01E001 - One.mp4");
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    let watched = WatchState {
        watched_at: Some(an_hour_ago()),
        position: PlaybackPosition::start(),
        last_played_at: Some(an_hour_ago()),
    };
    assert_eq!(
        (
            watch_state(&db, &channel_copy),
            watch_state(&db, &playlist_copy)
        ),
        (watched, watched)
    );
    assert_eq!(plex.mutations(), vec!["scrobble:101".to_string()]);
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        by_video_id(vec![
            agreed(&channel_copy, "101", WatchProgress::Watched),
            agreed(&playlist_copy, "201", WatchProgress::Watched),
        ])
    );
}

#[test]
fn it_should_rewind_a_video_unwatched_in_plex_while_in_progress() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[]),
    );
    let video = seed_video(
        &db,
        Video {
            watch: in_progress(300, an_hour_ago()),
            ..downloaded_video("yt1", "S2023E1114 - One.mp4")
        },
    );
    episode_watch_repository
        .save_many(&[agreed(
            &video,
            "101",
            WatchProgress::InProgress(position(300)),
        )])
        .unwrap();
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(watch_state(&db, &video), in_progress(0, an_hour_ago()));
    assert_eq!(plex.mutations(), Vec::<String>::new());
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        vec![agreed(&video, "101", WatchProgress::Unplayed)]
    );
}

#[test]
fn it_should_not_push_a_position_of_30_seconds_or_less() {
    let db = TestDatabase::new();
    let episode_watch_repository = Arc::new(SqliteEpisodeWatchRepository::new(db.database()));
    let plex = Arc::new(
        FakePlexLibraryRepository::default()
            .with_location("19", "/plex/videos/creators")
            .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[]),
    );
    let video = seed_video(
        &db,
        Video {
            watch: in_progress(12, an_hour_ago()),
            ..downloaded_video("yt1", "S2023E1114 - One.mp4")
        },
    );
    let task = SyncPlexWatchStateTask::new(syncer(
        &db,
        plex.clone(),
        episode_watch_repository.clone(),
        task_repository(&db),
    ));

    let result = run(&task);

    assert_eq!(result, Ok(TaskOutcome::Done));
    assert_eq!(plex.mutations(), Vec::<String>::new());
    assert_eq!(
        episode_watch_repository.list().unwrap(),
        vec![agreed(&video, "101", WatchProgress::Unplayed)]
    );
}

/// The syncer over the test database, with Plex seeing `/videos` at
/// `/plex/videos`.
fn syncer(
    db: &TestDatabase,
    plex: Arc<FakePlexLibraryRepository>,
    episode_watch_repository: Arc<dyn EpisodeWatchRepository>,
    task_repository: Arc<dyn TaskRepository>,
) -> PlexWatchSyncer {
    let clock = Arc::new(FixedClock(now()));
    PlexWatchSyncer::new(
        plex.clone(),
        PlexEpisodeFinder::new(
            plex,
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            "/videos",
            "/plex/videos",
        ),
        episode_watch_repository,
        Arc::new(SqliteVideoRepository::new(db.database())),
        VideoWatchStateUpdater::new(
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            clock.clone(),
            Arc::new(SqliteCategoryRepository::new(db.database())),
            CategoryVideoCollector::new(
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteVideoRepository::new(db.database())),
                Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            ),
        ),
        task_repository,
        clock,
    )
}

fn task_repository(db: &TestDatabase) -> Arc<SqliteTaskRepository> {
    Arc::new(SqliteTaskRepository::new(
        db.database(),
        Arc::new(FixedClock(now())),
    ))
}

/// The watch sync pending as task `id`, due at `run_at`.
fn pending_sync(id: i64, run_at: DateTime<Utc>) -> ScheduledTask {
    ScheduledTask {
        id,
        task: Task::SyncPlexWatchState,
        status: TaskStatus::Pending,
        retries: 0,
        run_at,
        created_at: now(),
        updated_at: now(),
        last_error: None,
    }
}

/// A video of `my_channel()` downloaded to `Season 2023/<file_name>`.
fn downloaded_video(youtube_id: &str, file_name: &str) -> Video {
    Video::create(YoutubeVideoId::new(youtube_id).unwrap(), youtube_id, now()).mark_downloaded(
        Quality::High,
        format!("Season 2023/{file_name}"),
        None,
        None,
        now(),
    )
}

/// Stores `video` with `my_channel()`.
fn seed_video(db: &TestDatabase, video: Video) -> Video {
    let channel_repository = SqliteChannelRepository::new(db.database());
    if channel_repository.find(&my_channel_id()).unwrap().is_none() {
        channel_repository.insert(&my_channel()).unwrap();
    }
    SqliteVideoRepository::new(db.database())
        .insert(&video)
        .unwrap();
    SqliteChannelVideoRepository::new(db.database())
        .save(&ChannelVideo::create(
            my_channel_id(),
            video.id.clone(),
            0,
            now(),
        ))
        .unwrap();
    video
}

/// The stored watch state of `video`.
fn watch_state(db: &TestDatabase, video: &Video) -> WatchState {
    SqliteVideoRepository::new(db.database())
        .find(&video.id)
        .unwrap()
        .unwrap()
        .watch
}

/// Unwatched in yarrtube, stopped at `seconds`, last played `at`.
fn in_progress(seconds: i64, at: DateTime<Utc>) -> WatchState {
    WatchState {
        watched_at: None,
        position: position(seconds),
        last_played_at: Some(at),
    }
}

fn position(seconds: i64) -> PlaybackPosition {
    PlaybackPosition::new(seconds).unwrap()
}

/// Never finished in Plex, stopped at `offset_ms` of a 40-minute episode,
/// last viewed `at`.
fn plex_in_progress(offset_ms: i64, at: DateTime<Utc>) -> PlexWatchState {
    PlexWatchState {
        view_offset_ms: Some(offset_ms),
        last_viewed_at: Some(at),
        duration_ms: Some(2_400_000),
        ..PlexWatchState::unplayed()
    }
}

/// Watched once in Plex, last viewed `at`.
fn plex_watched(at: DateTime<Utc>) -> PlexWatchState {
    PlexWatchState {
        view_count: 1,
        last_viewed_at: Some(at),
        ..PlexWatchState::unplayed()
    }
}

/// A copy of `youtube_id` in `my_playlist()`, downloaded to
/// `Season 1/<file_name>`.
fn seed_playlist_video(db: &TestDatabase, youtube_id: &str, file_name: &str) -> Video {
    let playlist_repository = SqlitePlaylistRepository::new(db.database());
    if playlist_repository
        .find(&my_playlist_id())
        .unwrap()
        .is_none()
    {
        playlist_repository.insert(&my_playlist()).unwrap();
    }
    let video = Video::create(YoutubeVideoId::new(youtube_id).unwrap(), youtube_id, now())
        .mark_downloaded(
            Quality::High,
            format!("Season 1/{file_name}"),
            None,
            None,
            now(),
        );
    SqliteVideoRepository::new(db.database())
        .insert(&video)
        .unwrap();
    SqlitePlaylistVideoRepository::new(db.database())
        .save(&PlaylistVideo::create(
            my_playlist_id(),
            video.id.clone(),
            1,
            now(),
        ))
        .unwrap();
    video
}

/// `rows` in the order the repository lists them.
fn by_video_id(mut rows: Vec<EpisodeWatch>) -> Vec<EpisodeWatch> {
    rows.sort_by(|a, b| a.video_id.as_str().cmp(b.video_id.as_str()));
    rows
}

fn agreed(video: &Video, rating_key: &str, progress: WatchProgress) -> EpisodeWatch {
    EpisodeWatch {
        video_id: video.id.clone(),
        rating_key: rating_key.to_string(),
        agreed: progress,
    }
}

/// Where Plex sees `my_channel()`'s episode `file_name`.
fn plex_file(file_name: &str) -> String {
    format!("/plex/videos/creators/somechannel/Season 2023/{file_name}")
}

/// Where Plex sees `my_playlist()`'s episode `file_name`.
fn playlist_plex_file(file_name: &str) -> String {
    format!("/plex/videos/music/chill/Season 1/{file_name}")
}

fn my_playlist_id() -> PlaylistId {
    PlaylistId::new("PL1").unwrap()
}

fn my_playlist() -> Playlist {
    Playlist::create(
        my_playlist_id(),
        PlaylistName::new("Chill").unwrap(),
        PlaylistPath::new("music/chill").unwrap(),
        Quality::High,
        PlaylistKind::YoutubeLinked,
        false,
        ReconcileInterval::default_interval(),
        now(),
    )
}

fn my_channel_id() -> ChannelHandle {
    ChannelHandle::new("@somechannel").unwrap()
}

fn my_channel() -> Channel {
    Channel::create(
        my_channel_id(),
        "Some Channel",
        "UC123",
        Quality::High,
        VideoLimit::new(10).unwrap(),
        ReconcileInterval::default_interval(),
        PlaylistPath::new("creators/somechannel").unwrap(),
        None,
        now(),
    )
}

fn two_hours_ago() -> DateTime<Utc> {
    now() - Duration::hours(2)
}

fn an_hour_ago() -> DateTime<Utc> {
    now() - Duration::hours(1)
}

fn now() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
}

fn run(task: &SyncPlexWatchStateTask) -> Result<TaskOutcome, String> {
    task.handle(&Task::SyncPlexWatchState)
        .map_err(|e| e.to_string())
}
