use super::*;
use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
use crate::domain::channel_video::ChannelVideo;
use crate::domain::playlist::PlaylistPath;
use crate::domain::plex::PlexWatchState;
use crate::domain::plex_watch::EpisodeWatch;
use crate::domain::services::{
    CategoryVideoCollector, PlexEpisodeFinder, VideoOwnerFinder, VideoWatchStateUpdater,
};
use crate::domain::shared::{Quality, ReconcileInterval};
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
use crate::infrastructure::repositories::sqlite_playlist_repository::SqlitePlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
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

/// Watched once in Plex, last viewed `at`.
fn plex_watched(at: DateTime<Utc>) -> PlexWatchState {
    PlexWatchState {
        view_count: 1,
        last_viewed_at: Some(at),
        ..PlexWatchState::unplayed()
    }
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
