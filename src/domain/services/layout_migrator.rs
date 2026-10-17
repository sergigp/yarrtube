use crate::domain::channel::Channel;
use crate::domain::playlist::{Playlist, PlaylistPath};
use crate::domain::services::{
    Relocation, ShowFileWriter, ShowFileWriterApi, SkipReason, VideoRelocator, VideoRelocatorApi,
};
use crate::domain::video::{Video, VideoRecordId, resolve_output_dir};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::sync::Arc;

/// What a `migrate-layout` run moved, skipped and failed, each keyed by the
/// video's recorded path: moves as `(from, to)`, skips with their reason,
/// failures with their error.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub moved: Vec<(String, String)>,
    pub skipped: Vec<(String, SkipReason)>,
    pub failed: Vec<(String, String)>,
}

/// Moves an existing library into the TV-show layout from the database
/// alone: per source, writes its show files, then relocates its downloaded
/// legacy-layout videos, per publish day in publish-time order, so same-day
/// indexes follow publish order. Idempotent and resumable.
pub struct LayoutMigrator {
    video_repository: Arc<dyn VideoRepository>,
    playlist_repository: Arc<dyn PlaylistRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    relocator: Arc<VideoRelocator>,
    show_file_writer: Arc<ShowFileWriter>,
    videos_path: String,
}

impl LayoutMigrator {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        playlist_repository: Arc<dyn PlaylistRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
        relocator: Arc<VideoRelocator>,
        show_file_writer: Arc<ShowFileWriter>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_repository,
            playlist_repository,
            channel_repository,
            playlist_video_repository,
            channel_video_repository,
            video_metadata_repository,
            relocator,
            show_file_writer,
            videos_path: videos_path.into(),
        }
    }
}

pub trait LayoutMigratorApi: Send + Sync {
    /// Relocates every downloaded legacy-layout video into its season folder
    /// and writes every source's show files; on a dry run, reports the same
    /// plan and changes nothing. A video that fails to move is reported and
    /// the run continues.
    fn migrate(&self, dry_run: bool) -> anyhow::Result<MigrationReport>;
}

impl LayoutMigratorApi for LayoutMigrator {
    fn migrate(&self, dry_run: bool) -> anyhow::Result<MigrationReport> {
        let legacy = self.legacy_videos()?;
        let playlists = self.playlist_repository.list()?;
        let channels = self.channel_repository.list()?;
        let playlist_reports = playlists
            .iter()
            .map(|playlist| self.migrate_playlist(playlist, &legacy, dry_run));
        let channel_reports = channels
            .iter()
            .map(|channel| self.migrate_channel(channel, &legacy, dry_run));
        playlist_reports
            .chain(channel_reports)
            .try_fold(MigrationReport::default(), |report, source_report| {
                Ok(report.merge(source_report?))
            })
    }
}

impl LayoutMigrator {
    fn migrate_playlist(
        &self,
        playlist: &Playlist,
        legacy: &[Video],
        dry_run: bool,
    ) -> anyhow::Result<MigrationReport> {
        self.migrate_source(
            &playlist.path,
            || self.show_file_writer.write_playlist_show(playlist),
            || {
                Ok(self
                    .playlist_video_repository
                    .list_for_playlist(&playlist.id)?
                    .into_iter()
                    .map(|playlist_video| playlist_video.video_id)
                    .collect())
            },
            legacy,
            dry_run,
        )
    }

    fn migrate_channel(
        &self,
        channel: &Channel,
        legacy: &[Video],
        dry_run: bool,
    ) -> anyhow::Result<MigrationReport> {
        self.migrate_source(
            &channel.path,
            || self.show_file_writer.write_channel_show(channel),
            || {
                Ok(self
                    .channel_video_repository
                    .list_for_channel(&channel.id)?
                    .into_iter()
                    .map(|channel_video| channel_video.video_id)
                    .collect())
            },
            legacy,
            dry_run,
        )
    }

    /// Writes the show files of the source at `path` (not on a dry run),
    /// then relocates its legacy videos among `list_video_ids`.
    fn migrate_source(
        &self,
        path: &PlaylistPath,
        write_show: impl FnOnce() -> anyhow::Result<()>,
        list_video_ids: impl FnOnce() -> anyhow::Result<Vec<VideoRecordId>>,
        legacy: &[Video],
        dry_run: bool,
    ) -> anyhow::Result<MigrationReport> {
        let shows = if dry_run {
            MigrationReport::default()
        } else {
            MigrationReport::default().record_show_files(path, write_show())
        };
        let video_ids = list_video_ids()?;
        Ok(shows.merge(self.migrate_videos(path, &video_ids, legacy, dry_run)?))
    }

    /// Relocates a source's legacy videos in publish-time order, so the
    /// same-day indexes follow it.
    fn migrate_videos(
        &self,
        path: &PlaylistPath,
        video_ids: &[VideoRecordId],
        legacy: &[Video],
        dry_run: bool,
    ) -> anyhow::Result<MigrationReport> {
        let output_dir = resolve_output_dir(&self.videos_path, path.as_str());
        let videos = self.by_publish_time(legacy, video_ids)?;
        let (report, _) = videos.iter().fold(
            (MigrationReport::default(), Vec::new()),
            |(report, planned), (video, published_at)| {
                let relocation = if dry_run {
                    self.relocator
                        .plan(video, *published_at, &output_dir, &planned)
                } else {
                    self.relocator.relocate(video, *published_at, &output_dir)
                };
                let planned = match &relocation {
                    Ok(Relocation::Moved { number, .. }) => [planned, vec![*number]].concat(),
                    _ => planned,
                };
                (report.record(path, video, relocation), planned)
            },
        );
        Ok(report)
    }

    /// Every downloaded video still in the legacy layout.
    fn legacy_videos(&self) -> anyhow::Result<Vec<Video>> {
        Ok(self
            .video_repository
            .list_downloaded()?
            .into_iter()
            .filter(Video::is_in_legacy_layout)
            .collect())
    }

    /// The `legacy` videos among `video_ids`, each with its recorded publish
    /// time, earliest first and unknown last.
    fn by_publish_time(
        &self,
        legacy: &[Video],
        video_ids: &[VideoRecordId],
    ) -> anyhow::Result<Vec<(Video, Option<DateTime<Utc>>)>> {
        let video_ids = video_ids.iter().collect::<HashSet<_>>();
        let mut videos = legacy
            .iter()
            .filter(|video| video_ids.contains(&video.id))
            .map(|video| {
                let published_at = self
                    .video_metadata_repository
                    .find(&video.id)?
                    .map(|metadata| metadata.published_at);
                Ok((video.clone(), published_at))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        videos.sort_by_key(|(_, published_at)| (published_at.is_none(), *published_at));
        Ok(videos)
    }
}

impl MigrationReport {
    fn merge(self, other: MigrationReport) -> Self {
        Self {
            moved: [self.moved, other.moved].concat(),
            skipped: [self.skipped, other.skipped].concat(),
            failed: [self.failed, other.failed].concat(),
        }
    }

    /// Adds a failure to write the show files of the source at `path`, keyed
    /// by the source's directory; its videos are still migrated.
    fn record_show_files(self, path: &PlaylistPath, written: anyhow::Result<()>) -> Self {
        match written {
            Ok(()) => self,
            Err(e) => self.merge(MigrationReport {
                failed: vec![(path.as_str().to_string(), e.to_string())],
                ..Default::default()
            }),
        }
    }

    /// Adds what relocating `video` of the source at `path` did, keyed by
    /// paths relative to the videos root.
    fn record(
        self,
        path: &PlaylistPath,
        video: &Video,
        relocation: anyhow::Result<Relocation>,
    ) -> Self {
        let in_source = |relative: &str| format!("{}/{relative}", path.as_str());
        let recorded = in_source(video.filename.as_deref().unwrap_or_default());
        self.merge(match relocation {
            Ok(Relocation::Moved { from, to, .. }) => MigrationReport {
                moved: vec![(in_source(&from), in_source(&to))],
                ..Default::default()
            },
            Ok(Relocation::Skipped(reason)) => MigrationReport {
                skipped: vec![(recorded, reason)],
                ..Default::default()
            },
            Ok(Relocation::AlreadyInPlace) => MigrationReport::default(),
            Err(e) => MigrationReport {
                failed: vec![(recorded, e.to_string())],
                ..Default::default()
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::playlist::{Playlist, PlaylistId, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::shared::{Quality, ReconcileInterval};
    use crate::domain::video::{
        ChannelEpisodeNumber, EpisodeNumber, PlaylistEpisodeNumber, Video, VideoId,
    };
    use crate::domain::video_metadata::{VideoMetadata, render_episode_nfo};
    use crate::infrastructure::repositories::filesystem_channel_avatar_repository::FakeChannelAvatarRepository;
    use crate::infrastructure::repositories::filesystem_show_file_repository::FakeShowFileRepository;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::SqliteChannelRepository;
    use crate::infrastructure::repositories::sqlite_channel_video_repository::SqliteChannelVideoRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::SqlitePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::SqlitePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::SqliteVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::SqliteVideoRepository;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};
    use std::path::{Path, PathBuf};

    #[test]
    fn it_should_migrate_a_legacy_library_into_the_tv_layout() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_file_exists(true));
        let show_file_repository = Arc::new(FakeShowFileRepository::default());
        let videos_root = tempfile::tempdir().unwrap();
        let playlist_dir = videos_root.path().join("playlists/kids");
        let channel_dir = videos_root.path().join("channels/somechannel");
        SqlitePlaylistRepository::new(db.database())
            .insert(&my_playlist())
            .unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        let alpha = legacy_playlist_video(
            "yt1",
            "Alpha",
            "Alpha/Alpha.mp4",
            Some("Alpha/Alpha.jpg"),
            1,
        );
        let beta = legacy_playlist_video("yt2", "Beta", "Beta [yt2]/Beta [yt2].mp4", None, 2);
        let gamma = legacy_video("yt3", "Gamma", "Gamma/Gamma.mp4", Some("Gamma/Gamma.jpg"));
        seed_playlist_video(&db, &alpha, 0);
        seed_playlist_video(&db, &beta, 1);
        seed_channel_video(&db, &gamma);
        seed_metadata(
            &db,
            &alpha,
            published("2026-03-15T10:00:00Z"),
            &playlist_dir.join("Alpha"),
        );
        seed_metadata(
            &db,
            &beta,
            published("2026-03-16T10:00:00Z"),
            &playlist_dir.join("Beta [yt2]"),
        );
        seed_metadata(
            &db,
            &gamma,
            published("2025-12-01T08:00:00Z"),
            &channel_dir.join("Gamma"),
        );
        let migrator = layout_migrator(
            &db,
            video_file_repository.clone(),
            show_file_repository.clone(),
            videos_root.path(),
        );

        let result = migrator.migrate(false);

        assert_eq!(
            result.unwrap(),
            MigrationReport {
                moved: vec![
                    moved(
                        "playlists/kids/Alpha/Alpha.mp4",
                        "playlists/kids/Season 1/S01E001 - Alpha.mp4"
                    ),
                    moved(
                        "playlists/kids/Beta [yt2]/Beta [yt2].mp4",
                        "playlists/kids/Season 1/S01E002 - Beta.mp4"
                    ),
                    moved(
                        "channels/somechannel/Gamma/Gamma.mp4",
                        "channels/somechannel/Season 2025/S2025E120101 - Gamma.mp4"
                    ),
                ],
                skipped: vec![],
                failed: vec![],
            }
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                alpha.clone().relocate_to_episode(
                    playlist_episode(1),
                    "Season 1/S01E001 - Alpha.mp4",
                    Some("Season 1/S01E001 - Alpha.jpg".to_string()),
                    fixed_timestamp(),
                ),
                beta.clone().relocate_to_episode(
                    playlist_episode(2),
                    "Season 1/S01E002 - Beta.mp4",
                    None,
                    fixed_timestamp(),
                ),
                gamma.clone().relocate_to_episode(
                    episode(2025, 120101),
                    "Season 2025/S2025E120101 - Gamma.mp4",
                    Some("Season 2025/S2025E120101 - Gamma.jpg".to_string()),
                    fixed_timestamp(),
                ),
            ]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![
                (
                    playlist_dir.join("Alpha/Alpha.mp4"),
                    playlist_dir.join("Season 1/S01E001 - Alpha.mp4")
                ),
                (
                    playlist_dir.join("Alpha/Alpha.jpg"),
                    playlist_dir.join("Season 1/S01E001 - Alpha.jpg")
                ),
                (
                    playlist_dir.join("Beta [yt2]/Beta [yt2].mp4"),
                    playlist_dir.join("Season 1/S01E002 - Beta.mp4")
                ),
                (
                    channel_dir.join("Gamma/Gamma.mp4"),
                    channel_dir.join("Season 2025/S2025E120101 - Gamma.mp4")
                ),
                (
                    channel_dir.join("Gamma/Gamma.jpg"),
                    channel_dir.join("Season 2025/S2025E120101 - Gamma.jpg")
                ),
            ]
        );
        assert_eq!(
            *video_file_repository.removed_if_empty.lock().unwrap(),
            vec![
                playlist_dir.join("Alpha"),
                playlist_dir.join("Beta [yt2]"),
                channel_dir.join("Gamma"),
            ]
        );
        assert_eq!(
            [
                playlist_dir.join("Alpha/movie.nfo"),
                playlist_dir.join("Beta [yt2]/movie.nfo"),
                channel_dir.join("Gamma/movie.nfo"),
            ]
            .map(|nfo| nfo.exists()),
            [false, false, false]
        );
        assert_eq!(
            std::fs::read_to_string(playlist_dir.join("Season 1/S01E001 - Alpha.nfo")).unwrap(),
            render_episode_nfo(
                &metadata_of(&alpha, published("2026-03-15T10:00:00Z"))
                    .with_thumb(Some("S01E001 - Alpha.jpg".to_string())),
                &playlist_episode(1)
            )
        );
        assert_eq!(
            show_file_repository
                .show_nfos
                .lock()
                .unwrap()
                .iter()
                .map(|(dir, _)| dir.clone())
                .collect::<Vec<_>>(),
            vec![playlist_dir.clone(), channel_dir.clone()]
        );
    }

    #[test]
    fn it_should_number_a_same_day_batch_in_publish_time_order() {
        let db = TestDatabase::new();
        let videos_root = tempfile::tempdir().unwrap();
        let channel_dir = videos_root.path().join("channels/somechannel");
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        let latest = legacy_video("yt1", "Latest", "Latest/Latest.mp4", None);
        let middle = legacy_video("yt2", "Middle", "Middle/Middle.mp4", None);
        let earliest = legacy_video("yt3", "Earliest", "Earliest/Earliest.mp4", None);
        seed_channel_video(&db, &latest);
        seed_channel_video(&db, &middle);
        seed_channel_video(&db, &earliest);
        seed_metadata(
            &db,
            &latest,
            published("2026-03-15T10:00:02Z"),
            &channel_dir.join("Latest"),
        );
        seed_metadata(
            &db,
            &middle,
            published("2026-03-15T10:00:01Z"),
            &channel_dir.join("Middle"),
        );
        seed_metadata(
            &db,
            &earliest,
            published("2026-03-15T10:00:00Z"),
            &channel_dir.join("Earliest"),
        );
        let migrator = layout_migrator(
            &db,
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            Arc::new(FakeShowFileRepository::default()),
            videos_root.path(),
        );

        let result = migrator.migrate(false);

        assert_eq!(
            result.unwrap(),
            MigrationReport {
                moved: vec![
                    moved(
                        "channels/somechannel/Earliest/Earliest.mp4",
                        "channels/somechannel/Season 2026/S2026E031501 - Earliest.mp4"
                    ),
                    moved(
                        "channels/somechannel/Middle/Middle.mp4",
                        "channels/somechannel/Season 2026/S2026E031502 - Middle.mp4"
                    ),
                    moved(
                        "channels/somechannel/Latest/Latest.mp4",
                        "channels/somechannel/Season 2026/S2026E031503 - Latest.mp4"
                    ),
                ],
                skipped: vec![],
                failed: vec![],
            }
        );
    }

    #[test]
    fn it_should_move_legacy_playlist_videos_under_their_recorded_numbers() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_root = tempfile::tempdir().unwrap();
        let playlist_dir = videos_root.path().join("playlists/kids");
        SqlitePlaylistRepository::new(db.database())
            .insert(&my_playlist())
            .unwrap();
        let first = legacy_playlist_video("yt1", "First", "First/First.mp4", None, 1);
        let second = legacy_playlist_video("yt2", "Second", "Second/Second.mp4", None, 2);
        seed_playlist_video(&db, &first, 0);
        seed_playlist_video(&db, &second, 1);
        seed_metadata(
            &db,
            &first,
            published("2026-03-16T10:00:00Z"),
            &playlist_dir.join("First"),
        );
        seed_metadata(
            &db,
            &second,
            published("2014-06-01T10:00:00Z"),
            &playlist_dir.join("Second"),
        );
        let migrator = layout_migrator(
            &db,
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            Arc::new(FakeShowFileRepository::default()),
            videos_root.path(),
        );

        let result = migrator.migrate(false);

        assert_eq!(
            result.unwrap(),
            MigrationReport {
                moved: vec![
                    moved(
                        "playlists/kids/Second/Second.mp4",
                        "playlists/kids/Season 1/S01E002 - Second.mp4"
                    ),
                    moved(
                        "playlists/kids/First/First.mp4",
                        "playlists/kids/Season 1/S01E001 - First.mp4"
                    ),
                ],
                skipped: vec![],
                failed: vec![],
            }
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                first.clone().relocate_to_episode(
                    playlist_episode(1),
                    "Season 1/S01E001 - First.mp4",
                    None,
                    fixed_timestamp(),
                ),
                second.clone().relocate_to_episode(
                    playlist_episode(2),
                    "Season 1/S01E002 - Second.mp4",
                    None,
                    fixed_timestamp(),
                ),
            ]
        );
    }

    #[test]
    fn it_should_move_a_playlist_video_without_metadata() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_file_exists(true));
        let videos_root = tempfile::tempdir().unwrap();
        let playlist_dir = videos_root.path().join("playlists/kids");
        SqlitePlaylistRepository::new(db.database())
            .insert(&my_playlist())
            .unwrap();
        let video =
            legacy_playlist_video("yt1", "Unpublished", "Unpublished/Unpublished.mp4", None, 3);
        seed_playlist_video(&db, &video, 2);
        let migrator = layout_migrator(
            &db,
            video_file_repository.clone(),
            Arc::new(FakeShowFileRepository::default()),
            videos_root.path(),
        );

        let result = migrator.migrate(false);

        assert_eq!(
            result.unwrap(),
            MigrationReport {
                moved: vec![moved(
                    "playlists/kids/Unpublished/Unpublished.mp4",
                    "playlists/kids/Season 1/S01E003 - Unpublished.mp4"
                )],
                skipped: vec![],
                failed: vec![],
            }
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.clone().relocate_to_episode(
                playlist_episode(3),
                "Season 1/S01E003 - Unpublished.mp4",
                None,
                fixed_timestamp(),
            )]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                playlist_dir.join("Unpublished/Unpublished.mp4"),
                playlist_dir.join("Season 1/S01E003 - Unpublished.mp4")
            )]
        );
        assert_eq!(
            SqliteVideoMetadataRepository::new(db.database())
                .find(&video.id)
                .unwrap(),
            None
        );
    }

    #[test]
    fn it_should_complete_a_video_whose_media_an_interrupted_run_already_moved() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "Season 1/S01E001 - First.mp4".to_string(),
            "First/First.jpg".to_string(),
        ]));
        let videos_root = tempfile::tempdir().unwrap();
        let playlist_dir = videos_root.path().join("playlists/kids");
        SqlitePlaylistRepository::new(db.database())
            .insert(&my_playlist())
            .unwrap();
        let video = legacy_playlist_video(
            "yt1",
            "First",
            "First/First.mp4",
            Some("First/First.jpg"),
            1,
        );
        seed_playlist_video(&db, &video, 0);
        let migrator = layout_migrator(
            &db,
            video_file_repository.clone(),
            Arc::new(FakeShowFileRepository::default()),
            videos_root.path(),
        );

        let result = migrator.migrate(false);

        assert_eq!(
            result.unwrap(),
            MigrationReport {
                moved: vec![moved(
                    "playlists/kids/First/First.mp4",
                    "playlists/kids/Season 1/S01E001 - First.mp4"
                )],
                skipped: vec![],
                failed: vec![],
            }
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.clone().relocate_to_episode(
                playlist_episode(1),
                "Season 1/S01E001 - First.mp4",
                Some("Season 1/S01E001 - First.jpg".to_string()),
                fixed_timestamp(),
            )]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                playlist_dir.join("First/First.jpg"),
                playlist_dir.join("Season 1/S01E001 - First.jpg")
            )]
        );
        assert_eq!(
            *video_file_repository.removed_if_empty.lock().unwrap(),
            vec![playlist_dir.join("First")]
        );
    }

    #[test]
    fn it_should_report_show_files_it_cannot_write_and_still_move_the_videos() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_file_exists(true));
        let videos_root = tempfile::tempdir().unwrap();
        let playlist_dir = videos_root.path().join("playlists/kids");
        SqlitePlaylistRepository::new(db.database())
            .insert(&my_playlist())
            .unwrap();
        let video = legacy_playlist_video("yt1", "First", "First/First.mp4", None, 1);
        seed_playlist_video(&db, &video, 0);
        let migrator = layout_migrator(
            &db,
            video_file_repository.clone(),
            Arc::new(FakeShowFileRepository::failing()),
            videos_root.path(),
        );

        let result = migrator.migrate(false);

        assert_eq!(
            result.unwrap(),
            MigrationReport {
                moved: vec![moved(
                    "playlists/kids/First/First.mp4",
                    "playlists/kids/Season 1/S01E001 - First.mp4"
                )],
                skipped: vec![],
                failed: vec![(
                    "playlists/kids".to_string(),
                    format!("permission denied writing {playlist_dir:?}")
                )],
            }
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![video.clone().relocate_to_episode(
                playlist_episode(1),
                "Season 1/S01E001 - First.mp4",
                None,
                fixed_timestamp(),
            )]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                playlist_dir.join("First/First.mp4"),
                playlist_dir.join("Season 1/S01E001 - First.mp4")
            )]
        );
    }

    #[test]
    fn it_should_move_videos_without_a_publish_time_last() {
        let db = TestDatabase::new();
        let videos_root = tempfile::tempdir().unwrap();
        let playlist_dir = videos_root.path().join("playlists/kids");
        SqlitePlaylistRepository::new(db.database())
            .insert(&my_playlist())
            .unwrap();
        let unpublished =
            legacy_playlist_video("yt1", "Unpublished", "Unpublished/Unpublished.mp4", None, 1);
        let published_video =
            legacy_playlist_video("yt2", "Published", "Published/Published.mp4", None, 2);
        seed_playlist_video(&db, &unpublished, 0);
        seed_playlist_video(&db, &published_video, 1);
        seed_metadata(
            &db,
            &published_video,
            published("2026-03-15T10:00:00Z"),
            &playlist_dir.join("Published"),
        );
        let migrator = layout_migrator(
            &db,
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            Arc::new(FakeShowFileRepository::default()),
            videos_root.path(),
        );

        let result = migrator.migrate(false);

        assert_eq!(
            result.unwrap(),
            MigrationReport {
                moved: vec![
                    moved(
                        "playlists/kids/Published/Published.mp4",
                        "playlists/kids/Season 1/S01E002 - Published.mp4"
                    ),
                    moved(
                        "playlists/kids/Unpublished/Unpublished.mp4",
                        "playlists/kids/Season 1/S01E001 - Unpublished.mp4"
                    ),
                ],
                skipped: vec![],
                failed: vec![],
            }
        );
    }

    #[test]
    fn it_should_skip_videos_without_metadata_or_without_a_file() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_listing(vec![
            "Present/Present.mp4".to_string(),
            "Unpublished/Unpublished.mp4".to_string(),
        ]));
        let videos_root = tempfile::tempdir().unwrap();
        let playlist_dir = videos_root.path().join("playlists/kids");
        SqlitePlaylistRepository::new(db.database())
            .insert(&my_playlist())
            .unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        let present = legacy_playlist_video("yt1", "Present", "Present/Present.mp4", None, 1);
        let unpublished = legacy_video("yt2", "Unpublished", "Unpublished/Unpublished.mp4", None);
        let missing = legacy_playlist_video("yt3", "Missing", "Missing/Missing.mp4", None, 2);
        seed_playlist_video(&db, &present, 0);
        seed_channel_video(&db, &unpublished);
        seed_playlist_video(&db, &missing, 1);
        seed_metadata(
            &db,
            &present,
            published("2026-03-15T10:00:00Z"),
            &playlist_dir.join("Present"),
        );
        seed_metadata(
            &db,
            &missing,
            published("2026-03-15T09:00:00Z"),
            &playlist_dir.join("Missing"),
        );
        let migrator = layout_migrator(
            &db,
            video_file_repository.clone(),
            Arc::new(FakeShowFileRepository::default()),
            videos_root.path(),
        );

        let result = migrator.migrate(false);

        assert_eq!(
            result.unwrap(),
            MigrationReport {
                moved: vec![moved(
                    "playlists/kids/Present/Present.mp4",
                    "playlists/kids/Season 1/S01E001 - Present.mp4"
                )],
                skipped: vec![
                    (
                        "playlists/kids/Missing/Missing.mp4".to_string(),
                        SkipReason::FileMissing
                    ),
                    (
                        "channels/somechannel/Unpublished/Unpublished.mp4".to_string(),
                        SkipReason::NoPublishTimestamp
                    ),
                ],
                failed: vec![],
            }
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                present.clone().relocate_to_episode(
                    playlist_episode(1),
                    "Season 1/S01E001 - Present.mp4",
                    None,
                    fixed_timestamp(),
                ),
                unpublished,
                missing,
            ]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                playlist_dir.join("Present/Present.mp4"),
                playlist_dir.join("Season 1/S01E001 - Present.mp4")
            )]
        );
    }

    #[test]
    fn it_should_move_nothing_on_a_second_run() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_file_exists(true));
        let show_file_repository = Arc::new(FakeShowFileRepository::default());
        let videos_root = tempfile::tempdir().unwrap();
        let playlist_dir = videos_root.path().join("playlists/kids");
        SqlitePlaylistRepository::new(db.database())
            .insert(&my_playlist())
            .unwrap();
        let alpha = legacy_playlist_video("yt1", "Alpha", "Alpha/Alpha.mp4", None, 1);
        seed_playlist_video(&db, &alpha, 0);
        seed_metadata(
            &db,
            &alpha,
            published("2026-03-15T10:00:00Z"),
            &playlist_dir.join("Alpha"),
        );
        let migrator = layout_migrator(
            &db,
            video_file_repository.clone(),
            show_file_repository.clone(),
            videos_root.path(),
        );
        migrator.migrate(false).unwrap();

        let result = migrator.migrate(false);

        assert_eq!(result.unwrap(), MigrationReport::default());
        assert_eq!(
            video_repository.list().unwrap(),
            vec![alpha.relocate_to_episode(
                playlist_episode(1),
                "Season 1/S01E001 - Alpha.mp4",
                None,
                fixed_timestamp(),
            )]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                playlist_dir.join("Alpha/Alpha.mp4"),
                playlist_dir.join("Season 1/S01E001 - Alpha.mp4")
            )]
        );
        assert_eq!(
            show_file_repository
                .show_nfos
                .lock()
                .unwrap()
                .iter()
                .map(|(dir, _)| dir.clone())
                .collect::<Vec<_>>(),
            vec![playlist_dir.clone(), playlist_dir.clone()]
        );
    }

    #[test]
    fn it_should_report_a_failed_move_and_continue() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let videos_root = tempfile::tempdir().unwrap();
        let channel_dir = videos_root.path().join("channels/somechannel");
        let video_file_repository = Arc::new(
            FakeVideoFileRepository::with_file_exists(true)
                .with_failing_move(channel_dir.join("Beta/Beta.mp4")),
        );
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        let alpha = legacy_video("yt1", "Alpha", "Alpha/Alpha.mp4", None);
        let beta = legacy_video("yt2", "Beta", "Beta/Beta.mp4", None);
        let gamma = legacy_video("yt3", "Gamma", "Gamma/Gamma.mp4", None);
        seed_channel_video(&db, &alpha);
        seed_channel_video(&db, &beta);
        seed_channel_video(&db, &gamma);
        seed_metadata(
            &db,
            &alpha,
            published("2026-03-15T10:00:00Z"),
            &channel_dir.join("Alpha"),
        );
        seed_metadata(
            &db,
            &beta,
            published("2026-03-16T10:00:00Z"),
            &channel_dir.join("Beta"),
        );
        seed_metadata(
            &db,
            &gamma,
            published("2026-03-17T10:00:00Z"),
            &channel_dir.join("Gamma"),
        );
        let migrator = layout_migrator(
            &db,
            video_file_repository.clone(),
            Arc::new(FakeShowFileRepository::default()),
            videos_root.path(),
        );

        let result = migrator.migrate(false);

        assert_eq!(
            result.unwrap(),
            MigrationReport {
                moved: vec![
                    moved(
                        "channels/somechannel/Alpha/Alpha.mp4",
                        "channels/somechannel/Season 2026/S2026E031501 - Alpha.mp4"
                    ),
                    moved(
                        "channels/somechannel/Gamma/Gamma.mp4",
                        "channels/somechannel/Season 2026/S2026E031701 - Gamma.mp4"
                    ),
                ],
                skipped: vec![],
                failed: vec![(
                    "channels/somechannel/Beta/Beta.mp4".to_string(),
                    format!(
                        "permission denied moving {:?}",
                        channel_dir.join("Beta/Beta.mp4")
                    )
                )],
            }
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                alpha.relocate_to_episode(
                    episode(2026, 31501),
                    "Season 2026/S2026E031501 - Alpha.mp4",
                    None,
                    fixed_timestamp(),
                ),
                Video {
                    episode_number: Some(episode(2026, 31601)),
                    ..beta
                },
                gamma.relocate_to_episode(
                    episode(2026, 31701),
                    "Season 2026/S2026E031701 - Gamma.mp4",
                    None,
                    fixed_timestamp(),
                ),
            ]
        );
    }

    #[test]
    fn it_should_plan_the_same_day_of_different_years_independently_on_a_dry_run() {
        let db = TestDatabase::new();
        let videos_root = tempfile::tempdir().unwrap();
        let channel_dir = videos_root.path().join("channels/somechannel");
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        let older = legacy_video("yt1", "Older", "Older/Older.mp4", None);
        let newer = legacy_video("yt2", "Newer", "Newer/Newer.mp4", None);
        seed_channel_video(&db, &older);
        seed_channel_video(&db, &newer);
        seed_metadata(
            &db,
            &older,
            published("2025-03-15T10:00:00Z"),
            &channel_dir.join("Older"),
        );
        seed_metadata(
            &db,
            &newer,
            published("2026-03-15T10:00:00Z"),
            &channel_dir.join("Newer"),
        );
        let migrator = layout_migrator(
            &db,
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            Arc::new(FakeShowFileRepository::default()),
            videos_root.path(),
        );

        let result = migrator.migrate(true);

        assert_eq!(
            result.unwrap(),
            MigrationReport {
                moved: vec![
                    moved(
                        "channels/somechannel/Older/Older.mp4",
                        "channels/somechannel/Season 2025/S2025E031501 - Older.mp4"
                    ),
                    moved(
                        "channels/somechannel/Newer/Newer.mp4",
                        "channels/somechannel/Season 2026/S2026E031501 - Newer.mp4"
                    ),
                ],
                skipped: vec![],
                failed: vec![],
            }
        );
    }

    #[test]
    fn it_should_plan_past_a_later_number_already_recorded_that_day_on_a_dry_run() {
        let db = TestDatabase::new();
        let videos_root = tempfile::tempdir().unwrap();
        let channel_dir = videos_root.path().join("channels/somechannel");
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        let numbered = legacy_video("yt1", "Numbered", "Numbered/Numbered.mp4", None)
            .with_episode_number(episode(2026, 31501), fixed_timestamp());
        let in_place = Video::create(VideoId::new("yt2").unwrap(), "In Place", fixed_timestamp())
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                "Season 2026/S2026E031502 - In Place.mp4",
                None,
                None,
                fixed_timestamp(),
            )
            .with_episode_number(episode(2026, 31502), fixed_timestamp());
        let unnumbered = legacy_video("yt3", "Unnumbered", "Unnumbered/Unnumbered.mp4", None);
        seed_channel_video(&db, &numbered);
        seed_channel_video(&db, &in_place);
        seed_channel_video(&db, &unnumbered);
        seed_metadata(
            &db,
            &numbered,
            published("2026-03-15T08:00:00Z"),
            &channel_dir.join("Numbered"),
        );
        seed_metadata(
            &db,
            &unnumbered,
            published("2026-03-15T10:00:00Z"),
            &channel_dir.join("Unnumbered"),
        );
        let migrator = layout_migrator(
            &db,
            Arc::new(FakeVideoFileRepository::with_file_exists(true)),
            Arc::new(FakeShowFileRepository::default()),
            videos_root.path(),
        );

        let result = migrator.migrate(true);

        assert_eq!(
            result.unwrap(),
            MigrationReport {
                moved: vec![
                    moved(
                        "channels/somechannel/Numbered/Numbered.mp4",
                        "channels/somechannel/Season 2026/S2026E031501 - Numbered.mp4"
                    ),
                    moved(
                        "channels/somechannel/Unnumbered/Unnumbered.mp4",
                        "channels/somechannel/Season 2026/S2026E031503 - Unnumbered.mp4"
                    ),
                ],
                skipped: vec![],
                failed: vec![],
            }
        );
    }

    #[test]
    fn it_should_change_nothing_on_a_dry_run() {
        let db = TestDatabase::new();
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_file_repository = Arc::new(FakeVideoFileRepository::with_file_exists(true));
        let show_file_repository = Arc::new(FakeShowFileRepository::default());
        let videos_root = tempfile::tempdir().unwrap();
        let playlist_dir = videos_root.path().join("playlists/kids");
        let channel_dir = videos_root.path().join("channels/somechannel");
        SqlitePlaylistRepository::new(db.database())
            .insert(&my_playlist())
            .unwrap();
        SqliteChannelRepository::new(db.database())
            .insert(&my_channel())
            .unwrap();
        let alpha = legacy_playlist_video(
            "yt1",
            "Alpha",
            "Alpha/Alpha.mp4",
            Some("Alpha/Alpha.jpg"),
            1,
        );
        let beta = legacy_playlist_video("yt2", "Beta", "Beta [yt2]/Beta [yt2].mp4", None, 2);
        let gamma = legacy_video("yt3", "Gamma", "Gamma/Gamma.mp4", None);
        let delta = legacy_video("yt4", "Delta", "Delta/Delta.mp4", None);
        seed_playlist_video(&db, &alpha, 0);
        seed_playlist_video(&db, &beta, 1);
        seed_channel_video(&db, &gamma);
        seed_channel_video(&db, &delta);
        seed_metadata(
            &db,
            &alpha,
            published("2026-03-15T10:00:00Z"),
            &playlist_dir.join("Alpha"),
        );
        seed_metadata(
            &db,
            &beta,
            published("2014-06-01T10:00:00Z"),
            &playlist_dir.join("Beta [yt2]"),
        );
        seed_metadata(
            &db,
            &gamma,
            published("2026-03-15T09:00:00Z"),
            &channel_dir.join("Gamma"),
        );
        seed_metadata(
            &db,
            &delta,
            published("2026-03-15T08:00:00Z"),
            &channel_dir.join("Delta"),
        );
        let migrator = layout_migrator(
            &db,
            video_file_repository.clone(),
            show_file_repository.clone(),
            videos_root.path(),
        );

        let result = migrator.migrate(true);

        assert_eq!(
            result.unwrap(),
            MigrationReport {
                moved: vec![
                    moved(
                        "playlists/kids/Beta [yt2]/Beta [yt2].mp4",
                        "playlists/kids/Season 1/S01E002 - Beta.mp4"
                    ),
                    moved(
                        "playlists/kids/Alpha/Alpha.mp4",
                        "playlists/kids/Season 1/S01E001 - Alpha.mp4"
                    ),
                    moved(
                        "channels/somechannel/Delta/Delta.mp4",
                        "channels/somechannel/Season 2026/S2026E031501 - Delta.mp4"
                    ),
                    moved(
                        "channels/somechannel/Gamma/Gamma.mp4",
                        "channels/somechannel/Season 2026/S2026E031502 - Gamma.mp4"
                    ),
                ],
                skipped: vec![],
                failed: vec![],
            }
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![alpha, beta, gamma, delta]
        );
        assert_eq!(*video_file_repository.moved.lock().unwrap(), vec![]);
        assert_eq!(
            *video_file_repository.removed_if_empty.lock().unwrap(),
            Vec::<PathBuf>::new()
        );
        assert_eq!(*show_file_repository.show_nfos.lock().unwrap(), vec![]);
        assert_eq!(*show_file_repository.posters.lock().unwrap(), vec![]);
        assert_eq!(
            [
                playlist_dir.join("Alpha/movie.nfo"),
                playlist_dir.join("Beta [yt2]/movie.nfo"),
                channel_dir.join("Gamma/movie.nfo"),
                channel_dir.join("Delta/movie.nfo"),
                playlist_dir.join("Season 1"),
                channel_dir.join("Season 2026"),
            ]
            .map(|path| path.exists()),
            [true, true, true, true, false, false]
        );
    }

    /// Builds a migrator around the ports a test configures or asserts, over
    /// `db`'s real repositories and `videos_root`.
    fn layout_migrator(
        db: &TestDatabase,
        video_file_repository: Arc<FakeVideoFileRepository>,
        show_file_repository: Arc<FakeShowFileRepository>,
        videos_root: &Path,
    ) -> LayoutMigrator {
        let video_repository = Arc::new(SqliteVideoRepository::new(db.database()));
        let video_metadata_repository = Arc::new(SqliteVideoMetadataRepository::new(db.database()));
        let videos_path = videos_root.to_string_lossy().to_string();
        LayoutMigrator::new(
            video_repository.clone(),
            Arc::new(SqlitePlaylistRepository::new(db.database())),
            Arc::new(SqliteChannelRepository::new(db.database())),
            Arc::new(SqlitePlaylistVideoRepository::new(db.database())),
            Arc::new(SqliteChannelVideoRepository::new(db.database())),
            video_metadata_repository.clone(),
            Arc::new(VideoRelocator::new(
                video_repository,
                video_file_repository,
                video_metadata_repository,
                Arc::new(FixedClock(fixed_timestamp())),
            )),
            Arc::new(ShowFileWriter::new(
                show_file_repository,
                Arc::new(FakeChannelAvatarRepository::default()),
                videos_path.clone(),
            )),
            videos_path,
        )
    }

    fn my_playlist() -> Playlist {
        Playlist::create(
            PlaylistId::new("PL1").unwrap(),
            PlaylistName::new("Kids").unwrap(),
            PlaylistPath::new("playlists/kids").unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            false,
            ReconcileInterval::default_interval(),
            fixed_timestamp(),
        )
    }

    fn my_channel() -> Channel {
        Channel::create(
            ChannelHandle::new("@somechannel").unwrap(),
            "Some Channel",
            "UC123",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            ReconcileInterval::default_interval(),
            PlaylistPath::new("channels/somechannel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    /// A video downloaded in the per-video-folder layout.
    fn legacy_video(
        youtube_id: &str,
        title: &str,
        filename: &str,
        thumbnail_filename: Option<&str>,
    ) -> Video {
        Video::create(VideoId::new(youtube_id).unwrap(), title, fixed_timestamp())
            .start_download(fixed_timestamp())
            .mark_downloaded(
                Quality::High,
                filename,
                thumbnail_filename.map(str::to_string),
                None,
                fixed_timestamp(),
            )
    }

    /// A playlist video downloaded in the per-video-folder layout, numbered
    /// `episode` as the playlist reconcile (or migration `0008`) records it.
    fn legacy_playlist_video(
        youtube_id: &str,
        title: &str,
        filename: &str,
        thumbnail_filename: Option<&str>,
        episode: u32,
    ) -> Video {
        legacy_video(youtube_id, title, filename, thumbnail_filename)
            .with_episode_number(playlist_episode(episode), fixed_timestamp())
    }

    fn seed_playlist_video(db: &TestDatabase, video: &Video, position: i64) {
        SqliteVideoRepository::new(db.database())
            .save(video)
            .unwrap();
        SqlitePlaylistVideoRepository::new(db.database())
            .save(&PlaylistVideo::create(
                PlaylistId::new("PL1").unwrap(),
                video.id.clone(),
                position,
                fixed_timestamp(),
            ))
            .unwrap();
    }

    fn seed_channel_video(db: &TestDatabase, video: &Video) {
        SqliteVideoRepository::new(db.database())
            .save(video)
            .unwrap();
        SqliteChannelVideoRepository::new(db.database())
            .save(&ChannelVideo::create(
                ChannelHandle::new("@somechannel").unwrap(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
    }

    /// Records `video`'s metadata as the previous layout did, writing its
    /// `movie.nfo` into `video_dir`.
    fn seed_metadata(
        db: &TestDatabase,
        video: &Video,
        published_at: DateTime<Utc>,
        video_dir: &Path,
    ) {
        SqliteVideoMetadataRepository::new(db.database())
            .save(
                &video.id,
                &metadata_of(video, published_at),
                &ChannelEpisodeNumber::first_of_day(published_at).into(),
                &video_dir.join("movie.nfo"),
            )
            .unwrap();
    }

    fn metadata_of(video: &Video, published_at: DateTime<Utc>) -> VideoMetadata {
        VideoMetadata::new(
            video.title.clone(),
            "A description",
            "My Channel",
            "My Channel",
            published_at,
            None,
            Vec::new(),
            video.youtube_id.as_str(),
            None,
            video.title.clone(),
            fixed_timestamp(),
        )
    }

    fn moved(from: &str, to: &str) -> (String, String) {
        (from.to_string(), to.to_string())
    }

    fn playlist_episode(episode: u32) -> EpisodeNumber {
        PlaylistEpisodeNumber::new(episode).unwrap().into()
    }

    fn episode(season: i32, episode: u32) -> EpisodeNumber {
        EpisodeNumber::from_columns(season, episode).unwrap()
    }

    fn published(timestamp: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(timestamp).unwrap().to_utc()
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }
}
