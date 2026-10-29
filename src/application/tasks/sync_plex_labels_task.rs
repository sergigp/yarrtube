use crate::domain::services::{PlexLabelSyncer, PlexLabelSyncerApi};
use crate::domain::task::Task;
use crate::infrastructure::repositories::task_handler::TaskHandler;

/// Runs one Plex label sync (`PlexLabelSyncer::sync`), which requests the
/// next one itself. Registered only when the Plex integration is configured.
pub struct SyncPlexLabelsTask {
    plex_label_syncer: PlexLabelSyncer,
}

impl SyncPlexLabelsTask {
    pub fn new(plex_label_syncer: PlexLabelSyncer) -> Self {
        Self { plex_label_syncer }
    }
}

impl TaskHandler for SyncPlexLabelsTask {
    fn handle(&self, payload: &str, _is_last_attempt: bool) -> anyhow::Result<()> {
        Task::decode_sync_plex_labels_payload(payload)?;
        self.plex_label_syncer.sync()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::category::{Category, CategoryColor, CategoryId, CategoryName};
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::plex::PlexLabels;
    use crate::domain::plex_label::{CategoryPlaylist, EpisodeLabels};
    use crate::domain::services::VideoOwnerFinder;
    use crate::domain::services::plex_label_syncer::BATCH_SIZE;
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::task::{ScheduledTask, TaskStatus};
    use crate::domain::video::{Video, VideoId, VideoRecordId};
    use crate::domain::video_category::VideoCategories;
    use crate::infrastructure::repositories::plex_library_repository::{
        FakePlaylist, FakePlexLibraryRepository,
    };
    use crate::infrastructure::repositories::sqlite_category_playlist_repository::{
        CategoryPlaylistRepository, SqliteCategoryPlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_category_repository::{
        CategoryRepository, SqliteCategoryRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_repository::{
        ChannelRepository, SqliteChannelRepository,
    };
    use crate::infrastructure::repositories::sqlite_channel_video_repository::{
        ChannelVideoRepository, SqliteChannelVideoRepository,
    };
    use crate::infrastructure::repositories::sqlite_episode_labels_repository::{
        EpisodeLabelsRepository, SqliteEpisodeLabelsRepository,
    };
    use crate::infrastructure::repositories::sqlite_playlist_repository::SqlitePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::{
        SqliteTaskRepository, TaskRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_categories_repository::{
        SqliteVideoCategoriesRepository, VideoCategoriesRepository,
    };
    use crate::infrastructure::repositories::sqlite_video_repository::{
        SqliteVideoRepository, VideoRepository,
    };
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Duration, Utc};
    use std::sync::Arc;

    #[test]
    fn it_should_label_the_episode_of_a_categorized_video() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[]),
        );
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "chess", "Chess", &[&video]);
        episode_labels_repository
            .save_many(&[EpisodeLabels::pending(video.id.clone())])
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository.clone(),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(plex.labels("101"), vec!["Chess".to_string()]);
        assert_eq!(
            episode_labels_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![synced(&video, "101", &["Chess"])]
        );
    }

    #[test]
    fn it_should_label_an_episode_with_every_category() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[]),
        );
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "funny", "Funny", &[&video]);
        seed_category(&db, "chess", "Chess", &[&video]);
        episode_labels_repository
            .save_many(&[EpisodeLabels::pending(video.id.clone())])
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository.clone(),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex.labels("101"),
            vec!["Chess".to_string(), "Funny".to_string()]
        );
        assert_eq!(
            episode_labels_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![synced(&video, "101", &["Chess", "Funny"])]
        );
    }

    #[test]
    fn it_should_clear_the_labels_of_an_uncategorized_video() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &["Chess"]),
        );
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        episode_labels_repository
            .save_many(&[EpisodeLabels::pending(video.id.clone())])
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository.clone(),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(plex.labels("101"), Vec::<String>::new());
        assert_eq!(plex.label_edits(), vec!["labels:101::-Chess".to_string()]);
        assert_eq!(
            episode_labels_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![synced(&video, "101", &[])]
        );
    }

    #[test]
    fn it_should_not_rewrite_labels_already_in_sync() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &["Chess"]),
        );
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "chess", "chess", &[&video]);
        episode_labels_repository
            .save_many(&[EpisodeLabels::pending(video.id.clone())])
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository.clone(),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(plex.label_edits(), Vec::<String>::new());
        assert_eq!(
            episode_labels_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![synced(&video, "101", &["chess"])]
        );
    }

    #[test]
    fn it_should_replace_the_label_of_a_renamed_category() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &["Funny"]),
        );
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "funny", "Humor", &[&video]);
        episode_labels_repository
            .save_many(&[EpisodeLabels {
                dirty: true,
                ..synced(&video, "101", &["Funny"])
            }])
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository.clone(),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(plex.labels("101"), vec!["Humor".to_string()]);
        assert_eq!(
            plex.label_edits(),
            vec!["labels:101:Humor:-Funny".to_string()]
        );
        assert_eq!(
            episode_labels_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![synced(&video, "101", &["Humor"])]
        );
    }

    #[test]
    fn it_should_keep_a_video_dirty_until_plex_imports_it() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default().with_location("19", "/plex/videos/creators"),
        );
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "chess", "Chess", &[&video]);
        episode_labels_repository
            .save_many(&[EpisodeLabels::pending(video.id.clone())])
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository.clone(),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(plex.label_edits(), Vec::<String>::new());
        assert_eq!(
            episode_labels_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![checked(&video)]
        );
    }

    #[test]
    fn it_should_keep_a_video_dirty_until_it_is_downloaded() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[]),
        );
        let video = seed_video(
            &db,
            Video::create(VideoId::new("yt1").unwrap(), "One", now()),
        );
        seed_category(&db, "chess", "Chess", &[&video]);
        episode_labels_repository
            .save_many(&[EpisodeLabels::pending(video.id.clone())])
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository.clone(),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(plex.label_edits(), Vec::<String>::new());
        assert_eq!(
            episode_labels_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![checked(&video)]
        );
    }

    #[test]
    fn it_should_forget_a_deleted_video() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default().with_location("19", "/plex/videos/creators"),
        );
        let deleted = VideoRecordId::new("deleted").unwrap();
        episode_labels_repository
            .save_many(&[EpisodeLabels::pending(deleted.clone())])
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository.clone(),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(
            episode_labels_repository.find_many(&[deleted]).unwrap(),
            vec![]
        );
    }

    #[test]
    fn it_should_label_categorized_videos_never_synced() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[]),
        );
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "chess", "Chess", &[&video]);
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository.clone(),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(plex.labels("101"), vec!["Chess".to_string()]);
        assert_eq!(
            episode_labels_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![synced(&video, "101", &["Chess"])]
        );
    }

    #[test]
    fn it_should_relabel_an_episode_plex_reimported() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "201", &plex_file("S2023E1114 - One.mp4"), &[]),
        );
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "chess", "Chess", &[&video]);
        episode_labels_repository
            .save_many(&[synced(&video, "101", &["Chess"])])
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository.clone(),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(plex.labels("201"), vec!["Chess".to_string()]);
        assert_eq!(
            episode_labels_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![EpisodeLabels {
                generation: 1,
                ..synced(&video, "201", &["Chess"])
            }]
        );
    }

    #[test]
    fn it_should_label_the_others_if_plex_rejects_one() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
                .with_episode("19", "102", &plex_file("S2023E1115 - Two.mp4"), &[])
                .rejecting("101"),
        );
        let rejected = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        let other = seed_downloaded_video(&db, "yt2", "S2023E1115 - Two.mp4");
        seed_category(&db, "chess", "Chess", &[&rejected, &other]);
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository.clone(),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(plex.labels("102"), vec!["Chess".to_string()]);
        assert_eq!(
            episode_labels_repository
                .find_many(&[rejected.id.clone(), other.id.clone()])
                .unwrap(),
            vec![checked(&rejected), synced(&other, "102", &["Chess"])]
        );
    }

    #[test]
    fn it_should_keep_videos_dirty_if_plex_is_unreachable() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let task_repository = task_repository(&db);
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "chess", "Chess", &[&video]);
        episode_labels_repository
            .save_many(&[EpisodeLabels::pending(video.id.clone())])
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            Arc::new(FakePlexLibraryRepository::failing()),
            episode_labels_repository.clone(),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository.clone(),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(
            episode_labels_repository
                .find_many(std::slice::from_ref(&video.id))
                .unwrap(),
            vec![EpisodeLabels::pending(video.id.clone())]
        );
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_sync(1, now() + Duration::minutes(15))]
        );
    }

    #[test]
    fn it_should_schedule_the_next_sync() {
        let db = TestDatabase::new();
        let task_repository = task_repository(&db);
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            Arc::new(
                FakePlexLibraryRepository::default().with_location("19", "/plex/videos/creators"),
            ),
            Arc::new(SqliteEpisodeLabelsRepository::new(db.database())),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository.clone(),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_sync(1, now() + Duration::minutes(15))]
        );
    }

    #[test]
    fn it_should_schedule_the_next_sync_now_if_the_batch_was_full() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let task_repository = task_repository(&db);
        episode_labels_repository
            .save_many(
                &(0..BATCH_SIZE)
                    .map(|i| EpisodeLabels::pending(VideoRecordId::new(format!("rec{i}")).unwrap()))
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            Arc::new(
                FakePlexLibraryRepository::default().with_location("19", "/plex/videos/creators"),
            ),
            episode_labels_repository,
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository.clone(),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_sync(1, now())]
        );
    }

    #[test]
    fn it_should_not_sync_again_right_away_if_no_row_of_a_full_batch_could_be_labelled() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let task_repository = task_repository(&db);
        episode_labels_repository
            .save_many(
                &(0..BATCH_SIZE)
                    .map(|i| {
                        seed_video(
                            &db,
                            Video::create(VideoId::new(format!("w{i}")).unwrap(), "Waiting", now()),
                        )
                    })
                    .map(|video| EpisodeLabels::pending(video.id))
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            Arc::new(
                FakePlexLibraryRepository::default().with_location("19", "/plex/videos/creators"),
            ),
            episode_labels_repository,
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository.clone(),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(
            task_repository.list_non_completed().unwrap(),
            vec![pending_sync(1, now() + Duration::minutes(15))]
        );
    }

    #[test]
    fn it_should_rotate_videos_waiting_for_plex() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[]),
        );
        let waiting: Vec<EpisodeLabels> = (0..BATCH_SIZE)
            .map(|i| {
                seed_video(
                    &db,
                    Video::create(VideoId::new(format!("w{i}")).unwrap(), "Waiting", now()),
                )
            })
            .map(|video| EpisodeLabels::pending(video.id))
            .collect();
        let importable = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "chess", "Chess", &[&importable]);
        episode_labels_repository
            .save_many(
                &[
                    waiting,
                    vec![EpisodeLabels {
                        checked_at: Some(now() - Duration::minutes(1)),
                        ..EpisodeLabels::pending(importable.id.clone())
                    }],
                ]
                .concat(),
            )
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository.clone(),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository(&db),
        ));

        let results = [run(&task), run(&task)];

        assert_eq!(results, [Ok(()), Ok(())]);
        assert_eq!(plex.labels("101"), vec!["Chess".to_string()]);
        assert_eq!(
            episode_labels_repository
                .find_many(std::slice::from_ref(&importable.id))
                .unwrap(),
            vec![synced(&importable, "101", &["Chess"])]
        );
    }

    #[test]
    fn it_should_create_a_smart_playlist_for_a_category() {
        let db = TestDatabase::new();
        let category_playlist_repository =
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[]),
        );
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "chess", "Chess", &[&video]);
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            Arc::new(SqliteEpisodeLabelsRepository::new(db.database())),
            category_playlist_repository.clone(),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex.playlists(),
            vec![FakePlaylist {
                rating_key: "pl1".to_string(),
                title: "Chess".to_string(),
                filter: Some(("19".to_string(), "Chess".to_string())),
            }]
        );
        assert_eq!(
            category_playlist_repository.list().unwrap(),
            vec![category_playlist("19", "Chess", "pl1", "Chess")]
        );
    }

    #[test]
    fn it_should_keep_a_playlist_already_in_sync() {
        let db = TestDatabase::new();
        let category_playlist_repository =
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[]),
        );
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "chess", "Chess", &[&video]);
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            Arc::new(SqliteEpisodeLabelsRepository::new(db.database())),
            category_playlist_repository.clone(),
            task_repository(&db),
        ));
        run(&task).unwrap();
        let first_sync_mutations = plex.mutations();

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(plex.mutations(), first_sync_mutations);
        assert_eq!(
            plex.playlists(),
            vec![FakePlaylist {
                rating_key: "pl1".to_string(),
                title: "Chess".to_string(),
                filter: Some(("19".to_string(), "Chess".to_string())),
            }]
        );
        assert_eq!(
            category_playlist_repository.list().unwrap(),
            vec![category_playlist("19", "Chess", "pl1", "Chess")]
        );
    }

    #[test]
    fn it_should_title_playlists_by_library_if_a_category_spans_libraries() {
        let db = TestDatabase::new();
        let category_playlist_repository =
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_section_title("19", "Subscriptions")
                .with_location("21", "/plex/videos/creators")
                .with_section_title("21", "Sergi's Playlists")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
                .with_episode("21", "201", &plex_file("S2023E1115 - Two.mp4"), &[]),
        );
        let one = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        let two = seed_downloaded_video(&db, "yt2", "S2023E1115 - Two.mp4");
        seed_category(&db, "chess", "Chess", &[&one, &two]);
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            Arc::new(SqliteEpisodeLabelsRepository::new(db.database())),
            category_playlist_repository.clone(),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex.playlists(),
            vec![
                FakePlaylist {
                    rating_key: "pl1".to_string(),
                    title: "Chess · Subscriptions".to_string(),
                    filter: Some(("19".to_string(), "Chess".to_string())),
                },
                FakePlaylist {
                    rating_key: "pl2".to_string(),
                    title: "Chess · Sergi's Playlists".to_string(),
                    filter: Some(("21".to_string(), "Chess".to_string())),
                },
            ]
        );
        assert_eq!(
            category_playlist_repository.list().unwrap(),
            vec![
                category_playlist("19", "Chess", "pl1", "Chess · Subscriptions"),
                category_playlist("21", "Chess", "pl2", "Chess · Sergi's Playlists"),
            ]
        );
    }

    #[test]
    fn it_should_delete_the_playlist_of_a_category_without_labelled_episodes() {
        let db = TestDatabase::new();
        let category_playlist_repository =
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_playlist("pl7", "Funny"),
        );
        category_playlist_repository
            .save(&category_playlist("19", "Funny", "pl7", "Funny"))
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            Arc::new(SqliteEpisodeLabelsRepository::new(db.database())),
            category_playlist_repository.clone(),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(plex.playlists(), vec![]);
        assert_eq!(category_playlist_repository.list().unwrap(), vec![]);
    }

    #[test]
    fn it_should_replace_the_playlist_of_a_renamed_category() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let category_playlist_repository =
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &["Funny"])
                .with_playlist("pl7", "Funny"),
        );
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "funny", "Humor", &[&video]);
        episode_labels_repository
            .save_many(&[EpisodeLabels {
                dirty: true,
                ..synced(&video, "101", &["Funny"])
            }])
            .unwrap();
        category_playlist_repository
            .save(&category_playlist("19", "Funny", "pl7", "Funny"))
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository,
            category_playlist_repository.clone(),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex.playlists(),
            vec![FakePlaylist {
                rating_key: "pl1".to_string(),
                title: "Humor".to_string(),
                filter: Some(("19".to_string(), "Humor".to_string())),
            }]
        );
        assert_eq!(
            category_playlist_repository.list().unwrap(),
            vec![category_playlist("19", "Humor", "pl1", "Humor")]
        );
    }

    #[test]
    fn it_should_delete_the_playlist_of_a_category_whose_episodes_left_plex() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let category_playlist_repository =
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_playlist("pl7", "Chess"),
        );
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "chess", "Chess", &[&video]);
        episode_labels_repository
            .save_many(&[synced(&video, "101", &["Chess"])])
            .unwrap();
        category_playlist_repository
            .save(&category_playlist("19", "Chess", "pl7", "Chess"))
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository,
            category_playlist_repository.clone(),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(plex.playlists(), vec![]);
        assert_eq!(category_playlist_repository.list().unwrap(), vec![]);
    }

    #[test]
    fn it_should_keep_one_playlist_for_a_category_renamed_by_case() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let category_playlist_repository =
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &["chess"])
                .with_episode("19", "102", &plex_file("S2023E1115 - Two.mp4"), &["chess"])
                .with_playlist("pl7", "chess"),
        );
        let relabelled = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        let not_yet_relabelled = seed_downloaded_video(&db, "yt2", "S2023E1115 - Two.mp4");
        seed_category(&db, "chess", "Chess", &[&relabelled, &not_yet_relabelled]);
        episode_labels_repository
            .save_many(&[
                EpisodeLabels {
                    dirty: true,
                    ..synced(&relabelled, "101", &["chess"])
                },
                synced(&not_yet_relabelled, "102", &["chess"]),
            ])
            .unwrap();
        category_playlist_repository
            .save(&category_playlist("19", "chess", "pl7", "chess"))
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository,
            category_playlist_repository.clone(),
            task_repository(&db),
        ));
        run(&task).unwrap();

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex.playlists(),
            vec![FakePlaylist {
                rating_key: "pl1".to_string(),
                title: "Chess".to_string(),
                filter: Some(("19".to_string(), "Chess".to_string())),
            }]
        );
        assert_eq!(
            category_playlist_repository.list().unwrap(),
            vec![category_playlist("19", "Chess", "pl1", "Chess")]
        );
    }

    #[test]
    fn it_should_recreate_a_playlist_deleted_in_plex() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let category_playlist_repository =
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &["Chess"]),
        );
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "chess", "Chess", &[&video]);
        episode_labels_repository
            .save_many(&[synced(&video, "101", &["Chess"])])
            .unwrap();
        category_playlist_repository
            .save(&category_playlist("19", "Chess", "pl7", "Chess"))
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository,
            category_playlist_repository.clone(),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex.playlists(),
            vec![FakePlaylist {
                rating_key: "pl1".to_string(),
                title: "Chess".to_string(),
                filter: Some(("19".to_string(), "Chess".to_string())),
            }]
        );
        assert_eq!(
            category_playlist_repository.list().unwrap(),
            vec![category_playlist("19", "Chess", "pl1", "Chess")]
        );
    }

    #[test]
    fn it_should_not_touch_playlists_it_did_not_create() {
        let db = TestDatabase::new();
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &[])
                .with_playlist("900", "Chess")
                .with_playlist("901", "Funny"),
        );
        let video = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        seed_category(&db, "chess", "Chess", &[&video]);
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            Arc::new(SqliteEpisodeLabelsRepository::new(db.database())),
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database())),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(
            plex.playlists(),
            vec![
                FakePlaylist {
                    rating_key: "900".to_string(),
                    title: "Chess".to_string(),
                    filter: None,
                },
                FakePlaylist {
                    rating_key: "901".to_string(),
                    title: "Funny".to_string(),
                    filter: None,
                },
                FakePlaylist {
                    rating_key: "pl3".to_string(),
                    title: "Chess".to_string(),
                    filter: Some(("19".to_string(), "Chess".to_string())),
                },
            ]
        );
    }

    #[test]
    fn it_should_retitle_a_playlist_once_its_category_spans_libraries() {
        let db = TestDatabase::new();
        let episode_labels_repository = Arc::new(SqliteEpisodeLabelsRepository::new(db.database()));
        let category_playlist_repository =
            Arc::new(SqliteCategoryPlaylistRepository::new(db.database()));
        let plex = Arc::new(
            FakePlexLibraryRepository::default()
                .with_location("19", "/plex/videos/creators")
                .with_section_title("19", "Subscriptions")
                .with_location("21", "/plex/videos/creators")
                .with_section_title("21", "Sergi's Playlists")
                .with_episode("19", "101", &plex_file("S2023E1114 - One.mp4"), &["Chess"])
                .with_episode("21", "201", &plex_file("S2023E1115 - Two.mp4"), &[])
                .with_playlist("pl7", "Chess"),
        );
        let one = seed_downloaded_video(&db, "yt1", "S2023E1114 - One.mp4");
        let two = seed_downloaded_video(&db, "yt2", "S2023E1115 - Two.mp4");
        seed_category(&db, "chess", "Chess", &[&one, &two]);
        episode_labels_repository
            .save_many(&[synced(&one, "101", &["Chess"])])
            .unwrap();
        category_playlist_repository
            .save(&category_playlist("19", "Chess", "pl7", "Chess"))
            .unwrap();
        let task = SyncPlexLabelsTask::new(syncer(
            &db,
            plex.clone(),
            episode_labels_repository,
            category_playlist_repository.clone(),
            task_repository(&db),
        ));

        let result = run(&task);

        assert_eq!(result, Ok(()));
        assert_eq!(
            category_playlist_repository.list().unwrap(),
            vec![
                category_playlist("19", "Chess", "pl1", "Chess · Subscriptions"),
                category_playlist("21", "Chess", "pl2", "Chess · Sergi's Playlists"),
            ]
        );
        assert_eq!(
            plex.playlists()
                .into_iter()
                .map(|playlist| playlist.rating_key)
                .collect::<Vec<_>>(),
            vec!["pl1".to_string(), "pl2".to_string()]
        );
    }

    /// The syncer over the test database, with Plex seeing `/videos` at
    /// `/plex/videos`.
    fn syncer(
        db: &TestDatabase,
        plex: Arc<FakePlexLibraryRepository>,
        episode_labels_repository: Arc<dyn EpisodeLabelsRepository>,
        category_playlist_repository: Arc<dyn CategoryPlaylistRepository>,
        task_repository: Arc<dyn TaskRepository>,
    ) -> PlexLabelSyncer {
        PlexLabelSyncer::new(
            plex,
            episode_labels_repository,
            category_playlist_repository,
            Arc::new(SqliteVideoCategoriesRepository::new(db.database())),
            Arc::new(SqliteCategoryRepository::new(db.database())),
            Arc::new(SqliteVideoRepository::new(db.database())),
            Arc::new(VideoOwnerFinder::new(
                Arc::new(SqliteChannelVideoRepository::new(db.database())),
                Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
                Arc::new(SqliteChannelRepository::new(db.database())),
                Arc::new(SqlitePlaylistRepository::new(db.database())),
                "/videos",
            )),
            task_repository,
            Arc::new(FixedClock(now())),
            "/videos",
            "/plex/videos",
        )
    }

    fn task_repository(db: &TestDatabase) -> Arc<SqliteTaskRepository> {
        Arc::new(SqliteTaskRepository::new(
            db.database(),
            Arc::new(FixedClock(now())),
        ))
    }

    /// The label sync pending as task `id`, due at `run_at`.
    fn pending_sync(id: i64, run_at: DateTime<Utc>) -> ScheduledTask {
        ScheduledTask {
            id,
            task_type: "sync_plex_labels".to_string(),
            payload: "{}".to_string(),
            status: TaskStatus::Pending,
            retries: 0,
            run_at,
            created_at: now(),
            updated_at: now(),
            last_error: None,
        }
    }

    /// A video of `my_channel()` downloaded to `Season 2023/<file_name>`,
    /// stored with its channel.
    fn seed_downloaded_video(db: &TestDatabase, youtube_id: &str, file_name: &str) -> Video {
        seed_video(
            db,
            Video::create(VideoId::new(youtube_id).unwrap(), youtube_id, now()).mark_downloaded(
                Quality::High,
                format!("Season 2023/{file_name}"),
                None,
                None,
                now(),
            ),
        )
    }

    /// Stores `video` with `my_channel()`.
    fn seed_video(db: &TestDatabase, video: Video) -> Video {
        let channel_repository = SqliteChannelRepository::new(db.database());
        if channel_repository.find(&my_channel_id()).unwrap().is_none() {
            channel_repository.insert(&my_channel()).unwrap();
        }
        SqliteVideoRepository::new(db.database())
            .save(&video)
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

    /// Stores the category `id` named `name` and puts `videos` in it.
    fn seed_category(db: &TestDatabase, id: &str, name: &str, videos: &[&Video]) {
        let category = Category {
            id: CategoryId::new(id).unwrap(),
            ..Category::create(
                CategoryName::new(name).unwrap(),
                CategoryColor::new(CategoryColor::PALETTE[0]).unwrap(),
                now(),
            )
        };
        SqliteCategoryRepository::new(db.database())
            .insert(&category)
            .unwrap();
        let video_categories_repository = SqliteVideoCategoriesRepository::new(db.database());
        let with_category: Vec<VideoCategories> = video_categories_repository
            .find_many(
                &videos
                    .iter()
                    .map(|video| video.id.clone())
                    .collect::<Vec<_>>(),
            )
            .unwrap()
            .into_iter()
            .map(|current| {
                VideoCategories::new(
                    current.video_id,
                    [current.category_ids, vec![category.id.clone()]].concat(),
                )
            })
            .collect();
        video_categories_repository
            .save_many(&with_category)
            .unwrap();
    }

    /// Still dirty, looked at by the sync.
    fn checked(video: &Video) -> EpisodeLabels {
        EpisodeLabels {
            checked_at: Some(now()),
            ..EpisodeLabels::pending(video.id.clone())
        }
    }

    fn category_playlist(
        section_id: &str,
        label: &str,
        rating_key: &str,
        title: &str,
    ) -> CategoryPlaylist {
        CategoryPlaylist {
            section_id: section_id.to_string(),
            label: label.to_string(),
            rating_key: rating_key.to_string(),
            title: title.to_string(),
        }
    }

    fn synced(video: &Video, rating_key: &str, labels: &[&str]) -> EpisodeLabels {
        EpisodeLabels {
            video_id: video.id.clone(),
            dirty: false,
            rating_key: Some(rating_key.to_string()),
            section_id: Some("19".to_string()),
            labels: PlexLabels::new(labels.iter().map(|label| label.to_string()).collect()),
            checked_at: Some(now()),
            generation: 0,
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

    fn now() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn run(task: &SyncPlexLabelsTask) -> Result<(), String> {
        task.handle("{}", false).map_err(|e| e.to_string())
    }
}
