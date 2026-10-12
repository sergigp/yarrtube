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
        if !dry_run {
            self.show_file_writer.write_playlist_show(playlist)?;
        }
        let video_ids = self
            .playlist_video_repository
            .list_for_playlist(&playlist.id)?
            .into_iter()
            .map(|playlist_video| playlist_video.video_id)
            .collect::<Vec<_>>();
        self.migrate_videos(&playlist.path, &video_ids, legacy, dry_run)
    }

    fn migrate_channel(
        &self,
        channel: &Channel,
        legacy: &[Video],
        dry_run: bool,
    ) -> anyhow::Result<MigrationReport> {
        if !dry_run {
            self.show_file_writer.write_channel_show(channel)?;
        }
        let video_ids = self
            .channel_video_repository
            .list_for_channel(&channel.id)?
            .into_iter()
            .map(|channel_video| channel_video.video_id)
            .collect::<Vec<_>>();
        self.migrate_videos(&channel.path, &video_ids, legacy, dry_run)
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
    /// time, earliest first.
    fn by_publish_time(
        &self,
        legacy: &[Video],
        video_ids: &[VideoRecordId],
    ) -> anyhow::Result<Vec<(Video, Option<DateTime<Utc>>)>> {
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
        videos.sort_by_key(|(_, published_at)| *published_at);
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

    /// Adds what relocating `video` of the source at `path` did, keyed by
    /// paths relative to the videos root.
    fn record(
        mut self,
        path: &PlaylistPath,
        video: &Video,
        relocation: anyhow::Result<Relocation>,
    ) -> Self {
        let in_source = |relative: &str| format!("{}/{relative}", path.as_str());
        let recorded = in_source(video.filename.as_deref().unwrap_or_default());
        match relocation {
            Ok(Relocation::Moved { from, to, .. }) => {
                self.moved.push((in_source(&from), in_source(&to)))
            }
            Ok(Relocation::Skipped(reason)) => self.skipped.push((recorded, reason)),
            Ok(Relocation::AlreadyInPlace) => {}
            Err(e) => self.failed.push((recorded, e.to_string())),
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::{Channel, ChannelHandle, VideoLimit};
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::playlist::{Playlist, PlaylistId, PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::shared::Quality;
    use crate::domain::video::{EpisodeNumber, Video, VideoId};
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
        let alpha = legacy_video("yt1", "Alpha", "Alpha/Alpha.mp4", Some("Alpha/Alpha.jpg"));
        let beta = legacy_video("yt2", "Beta", "Beta [yt2]/Beta [yt2].mp4", None);
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
                        "playlists/kids/Season 2026/S2026E031501 - Alpha.mp4"
                    ),
                    moved(
                        "playlists/kids/Beta [yt2]/Beta [yt2].mp4",
                        "playlists/kids/Season 2026/S2026E031601 - Beta.mp4"
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
                    episode(2026, 31501),
                    "Season 2026/S2026E031501 - Alpha.mp4",
                    Some("Season 2026/S2026E031501 - Alpha.jpg".to_string()),
                    fixed_timestamp(),
                ),
                beta.clone().relocate_to_episode(
                    episode(2026, 31601),
                    "Season 2026/S2026E031601 - Beta.mp4",
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
                    playlist_dir.join("Season 2026/S2026E031501 - Alpha.mp4")
                ),
                (
                    playlist_dir.join("Alpha/Alpha.jpg"),
                    playlist_dir.join("Season 2026/S2026E031501 - Alpha.jpg")
                ),
                (
                    playlist_dir.join("Beta [yt2]/Beta [yt2].mp4"),
                    playlist_dir.join("Season 2026/S2026E031601 - Beta.mp4")
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
            std::fs::read_to_string(playlist_dir.join("Season 2026/S2026E031501 - Alpha.nfo"))
                .unwrap(),
            render_episode_nfo(
                &metadata_of(&alpha, published("2026-03-15T10:00:00Z"))
                    .with_thumb(Some("S2026E031501 - Alpha.jpg".to_string())),
                &episode(2026, 31501)
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
        let playlist_dir = videos_root.path().join("playlists/kids");
        SqlitePlaylistRepository::new(db.database())
            .insert(&my_playlist())
            .unwrap();
        let latest = legacy_video("yt1", "Latest", "Latest/Latest.mp4", None);
        let middle = legacy_video("yt2", "Middle", "Middle/Middle.mp4", None);
        let earliest = legacy_video("yt3", "Earliest", "Earliest/Earliest.mp4", None);
        seed_playlist_video(&db, &latest, 0);
        seed_playlist_video(&db, &middle, 1);
        seed_playlist_video(&db, &earliest, 2);
        seed_metadata(
            &db,
            &latest,
            published("2026-03-15T10:00:02Z"),
            &playlist_dir.join("Latest"),
        );
        seed_metadata(
            &db,
            &middle,
            published("2026-03-15T10:00:01Z"),
            &playlist_dir.join("Middle"),
        );
        seed_metadata(
            &db,
            &earliest,
            published("2026-03-15T10:00:00Z"),
            &playlist_dir.join("Earliest"),
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
                        "playlists/kids/Earliest/Earliest.mp4",
                        "playlists/kids/Season 2026/S2026E031501 - Earliest.mp4"
                    ),
                    moved(
                        "playlists/kids/Middle/Middle.mp4",
                        "playlists/kids/Season 2026/S2026E031502 - Middle.mp4"
                    ),
                    moved(
                        "playlists/kids/Latest/Latest.mp4",
                        "playlists/kids/Season 2026/S2026E031503 - Latest.mp4"
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
        let present = legacy_video("yt1", "Present", "Present/Present.mp4", None);
        let unpublished = legacy_video("yt2", "Unpublished", "Unpublished/Unpublished.mp4", None);
        let missing = legacy_video("yt3", "Missing", "Missing/Missing.mp4", None);
        seed_playlist_video(&db, &present, 0);
        seed_playlist_video(&db, &unpublished, 1);
        seed_playlist_video(&db, &missing, 2);
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
                    "playlists/kids/Season 2026/S2026E031501 - Present.mp4"
                )],
                skipped: vec![
                    (
                        "playlists/kids/Unpublished/Unpublished.mp4".to_string(),
                        SkipReason::NoPublishTimestamp
                    ),
                    (
                        "playlists/kids/Missing/Missing.mp4".to_string(),
                        SkipReason::FileMissing
                    ),
                ],
                failed: vec![],
            }
        );
        assert_eq!(
            video_repository.list().unwrap(),
            vec![
                present.clone().relocate_to_episode(
                    episode(2026, 31501),
                    "Season 2026/S2026E031501 - Present.mp4",
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
                playlist_dir.join("Season 2026/S2026E031501 - Present.mp4")
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
        let alpha = legacy_video("yt1", "Alpha", "Alpha/Alpha.mp4", None);
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
                episode(2026, 31501),
                "Season 2026/S2026E031501 - Alpha.mp4",
                None,
                fixed_timestamp(),
            )]
        );
        assert_eq!(
            *video_file_repository.moved.lock().unwrap(),
            vec![(
                playlist_dir.join("Alpha/Alpha.mp4"),
                playlist_dir.join("Season 2026/S2026E031501 - Alpha.mp4")
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
        let playlist_dir = videos_root.path().join("playlists/kids");
        let video_file_repository = Arc::new(
            FakeVideoFileRepository::with_file_exists(true)
                .with_failing_move(playlist_dir.join("Beta/Beta.mp4")),
        );
        SqlitePlaylistRepository::new(db.database())
            .insert(&my_playlist())
            .unwrap();
        let alpha = legacy_video("yt1", "Alpha", "Alpha/Alpha.mp4", None);
        let beta = legacy_video("yt2", "Beta", "Beta/Beta.mp4", None);
        let gamma = legacy_video("yt3", "Gamma", "Gamma/Gamma.mp4", None);
        seed_playlist_video(&db, &alpha, 0);
        seed_playlist_video(&db, &beta, 1);
        seed_playlist_video(&db, &gamma, 2);
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
            &playlist_dir.join("Beta"),
        );
        seed_metadata(
            &db,
            &gamma,
            published("2026-03-17T10:00:00Z"),
            &playlist_dir.join("Gamma"),
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
                        "playlists/kids/Alpha/Alpha.mp4",
                        "playlists/kids/Season 2026/S2026E031501 - Alpha.mp4"
                    ),
                    moved(
                        "playlists/kids/Gamma/Gamma.mp4",
                        "playlists/kids/Season 2026/S2026E031701 - Gamma.mp4"
                    ),
                ],
                skipped: vec![],
                failed: vec![(
                    "playlists/kids/Beta/Beta.mp4".to_string(),
                    format!(
                        "permission denied moving {:?}",
                        playlist_dir.join("Beta/Beta.mp4")
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
        let alpha = legacy_video("yt1", "Alpha", "Alpha/Alpha.mp4", Some("Alpha/Alpha.jpg"));
        let beta = legacy_video("yt2", "Beta", "Beta [yt2]/Beta [yt2].mp4", None);
        let gamma = legacy_video("yt3", "Gamma", "Gamma/Gamma.mp4", None);
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
            published("2026-03-15T11:00:00Z"),
            &playlist_dir.join("Beta [yt2]"),
        );
        seed_metadata(
            &db,
            &gamma,
            published("2026-03-15T09:00:00Z"),
            &channel_dir.join("Gamma"),
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
                        "playlists/kids/Alpha/Alpha.mp4",
                        "playlists/kids/Season 2026/S2026E031501 - Alpha.mp4"
                    ),
                    moved(
                        "playlists/kids/Beta [yt2]/Beta [yt2].mp4",
                        "playlists/kids/Season 2026/S2026E031502 - Beta.mp4"
                    ),
                    moved(
                        "channels/somechannel/Gamma/Gamma.mp4",
                        "channels/somechannel/Season 2026/S2026E031501 - Gamma.mp4"
                    ),
                ],
                skipped: vec![],
                failed: vec![],
            }
        );
        assert_eq!(video_repository.list().unwrap(), vec![alpha, beta, gamma]);
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
                playlist_dir.join("Season 2026"),
            ]
            .map(|path| path.exists()),
            [true, true, true, false]
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
                &EpisodeNumber::first_of_day(published_at),
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

    fn episode(season: i32, episode: u32) -> EpisodeNumber {
        EpisodeNumber::new(season, episode).unwrap()
    }

    fn published(timestamp: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(timestamp).unwrap().to_utc()
    }

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }
}
