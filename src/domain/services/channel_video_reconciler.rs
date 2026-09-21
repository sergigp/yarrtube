use crate::domain::channel::{Channel, ChannelHandle};
use crate::domain::channel_video::ChannelVideo;
use crate::domain::event::DomainEvent;
use crate::domain::services::thumbnail_fetcher::ThumbnailFetcher;
use crate::domain::shared::{VideoId, VideoRecordId};
use crate::domain::task::Task;
use crate::domain::video::{Video, VideoStatus, top_level_entry, video_dir_for_filename};
use crate::domain::video_metadata::{build_video_metadata, resolve_sorttitle};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_channel_videos_repository::ChannelVideosRepository;
use crate::infrastructure::repositories::youtube_metadata_repository::YoutubeMetadataRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

/// Reconciles a channel's stored videos against its current `video_limit`
/// most recent uploads on YouTube, and its output directory against
/// recorded downloads — the channel equivalent of `VideoReconciler`.
#[derive(Clone)]
pub struct ChannelVideoReconciler {
    channel_repository: Arc<dyn ChannelRepository>,
    video_repository: Arc<dyn VideoRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    channel_videos_repository: Arc<dyn ChannelVideosRepository>,
    youtube_metadata_repository: Arc<dyn YoutubeMetadataRepository>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    task_repository: Arc<dyn TaskRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    thumbnail_fetcher: Arc<ThumbnailFetcher>,
    clock: Arc<dyn Clock>,
    reconcile_interval_seconds: i64,
    videos_path: String,
}

impl ChannelVideoReconciler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        channel_repository: Arc<dyn ChannelRepository>,
        video_repository: Arc<dyn VideoRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        channel_videos_repository: Arc<dyn ChannelVideosRepository>,
        youtube_metadata_repository: Arc<dyn YoutubeMetadataRepository>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        task_repository: Arc<dyn TaskRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        thumbnail_fetcher: Arc<ThumbnailFetcher>,
        clock: Arc<dyn Clock>,
        reconcile_interval_seconds: i64,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            channel_repository,
            video_repository,
            channel_video_repository,
            channel_videos_repository,
            youtube_metadata_repository,
            video_metadata_repository,
            event_publisher,
            task_repository,
            video_file_repository,
            thumbnail_fetcher,
            clock,
            reconcile_interval_seconds,
            videos_path: videos_path.into(),
        }
    }

    fn output_dir(&self, path: &str) -> std::path::PathBuf {
        Path::new(&self.videos_path).join(path)
    }

    /// Regenerates `video`'s metadata — the channel equivalent of
    /// `VideoReconciler::generate_metadata`. A channel-tracked video's
    /// `sorttitle` always resolves via publish date: `ChannelVideo.position`
    /// is a recency rank, never passed in as a playlist position.
    fn generate_metadata(&self, video: &Video, output_dir: &Path) {
        let Some(filename) = video.filename.as_deref() else {
            return;
        };
        let metadata = match self.youtube_metadata_repository.find(&video.youtube_id) {
            Ok(Some(metadata)) => metadata,
            Ok(None) => {
                warn!(video_id = %video.id, "no YouTube metadata found for video, skipping metadata repair");
                return;
            }
            Err(e) => {
                warn!(video_id = %video.id, error = %e, "failed to fetch YouTube metadata during reconcile, skipping metadata repair");
                return;
            }
        };

        let thumbnail_filename = video
            .thumbnail_filename
            .as_deref()
            .and_then(|f| Path::new(f).file_name())
            .and_then(|f| f.to_str())
            .map(str::to_string);
        let sorttitle = resolve_sorttitle(&metadata.title, metadata.published_at, None);
        let video_metadata =
            build_video_metadata(&video.youtube_id, &metadata, sorttitle, thumbnail_filename);
        let video_dir = video_dir_for_filename(output_dir, filename);

        if let Err(e) = self
            .video_metadata_repository
            .save(&video.id, &video_metadata, &video_dir)
        {
            warn!(video_id = %video.id, error = %e, "failed to save video metadata during reconcile");
        }
    }

    /// Runs one reconcile pass for a channel and reschedules the next
    /// recurring pass — no-ops entirely if the channel no longer exists.
    pub fn reconcile(&self, id: ChannelHandle) -> anyhow::Result<()> {
        let Some(channel) = self.channel_repository.find(&id)? else {
            info!(channel_id = %id, "channel no longer exists, skipping reconcile");
            return Ok(());
        };

        self.run_reconcile_pass(&channel)?;

        let now = self.clock.now();
        let next_run_at = now + chrono::Duration::seconds(self.reconcile_interval_seconds);
        self.task_repository.schedule(
            &Task::ReconcileChannel {
                channel_id: id.as_str().to_string(),
            },
            next_run_at,
        )?;
        info!(channel_id = %id, next_run_at = %next_run_at, "scheduled next reconcile of channel");

        Ok(())
    }

    /// Runs one reconcile pass for a channel immediately, on demand, without
    /// touching the recurring reconcile schedule. No-ops entirely if the
    /// channel no longer exists.
    pub fn force_reconcile(&self, id: ChannelHandle) -> anyhow::Result<()> {
        let Some(channel) = self.channel_repository.find(&id)? else {
            info!(channel_id = %id, "channel no longer exists, skipping reconcile");
            return Ok(());
        };

        self.run_reconcile_pass(&channel)
    }

    /// Diffs membership against YouTube, then reconciles the filesystem
    /// against recorded downloads. Shared by `reconcile` and
    /// `force_reconcile`. A `yt-dlp` failure during the membership diff
    /// propagates before filesystem reconciliation runs, leaving every
    /// stored row (and every file on disk) untouched.
    fn run_reconcile_pass(&self, channel: &Channel) -> anyhow::Result<()> {
        info!(channel_id = %channel.id, "reconciling channel");

        self.sync_channel_membership(channel)?;
        self.reconcile_filesystem(channel)
    }

    /// Diffs the channel's current `video_limit` most recent uploads against
    /// its stored `ChannelVideo` rows: adds newly-seen videos as `PENDING`
    /// at their recency position, evicts stored videos no longer among the
    /// current top-N (whether removed on YouTube or aged past the limit).
    fn sync_channel_membership(&self, channel: &Channel) -> anyhow::Result<()> {
        let id = &channel.id;
        let output_dir = self.output_dir(channel.path.as_str());
        let current_videos = self
            .channel_videos_repository
            .list_current_videos(id, channel.video_limit.value())?;
        let stored_videos = self.channel_video_repository.list_for_channel(id)?;

        let now = self.clock.now();
        let mut current_youtube_ids = Vec::with_capacity(current_videos.len());
        for current in &current_videos {
            let youtube_id = VideoId::new(&current.youtube_id)?;
            let existing = self
                .channel_video_repository
                .find_by_youtube_video(id, &youtube_id)?;

            match existing {
                None => {
                    let video = Video::create(youtube_id.clone(), current.title.clone(), now);
                    self.video_repository.save(&video)?;
                    let channel_video =
                        ChannelVideo::create(id.clone(), video.id.clone(), current.position, now);
                    self.channel_video_repository.save(&channel_video)?;
                    self.thumbnail_fetcher.fetch(&video, &output_dir);
                    info!(
                        channel_id = %id,
                        video_id = %youtube_id,
                        title = %current.title,
                        "added video to channel"
                    );
                    self.event_publisher
                        .publish(&DomainEvent::VideoAddedToChannel {
                            channel_id: id.as_str().to_string(),
                            video_id: video.id.as_str().to_string(),
                        })?;
                }
                Some(existing) => {
                    if let Some(video) = self.video_repository.find(&existing.video_id)? {
                        self.video_repository.update(&Video {
                            title: current.title.clone(),
                            updated_at: now,
                            ..video
                        })?;
                    }
                    if existing.position != current.position {
                        self.channel_video_repository.save(&ChannelVideo {
                            position: current.position,
                            created_at: now,
                            ..existing
                        })?;
                    }
                }
            }

            current_youtube_ids.push(youtube_id);
        }

        let current_id_strs: HashSet<&str> =
            current_youtube_ids.iter().map(|v| v.as_str()).collect();
        for stored in &stored_videos {
            let Some(video) = self.video_repository.find(&stored.video_id)? else {
                continue;
            };
            if current_id_strs.contains(video.youtube_id.as_str()) {
                continue;
            }

            info!(
                channel_id = %id,
                video_id = %video.youtube_id,
                "evicting video from channel (no longer among its most recent uploads)"
            );
            self.channel_video_repository
                .delete(id, &video.youtube_id)?;
            self.video_repository.delete(&video.id)?;
            self.event_publisher
                .publish(&DomainEvent::VideoRemovedFromChannel {
                    channel_id: id.as_str().to_string(),
                    video_id: video.id.as_str().to_string(),
                    title: video.title.clone(),
                    filename: video.filename.clone(),
                    thumbnail_filename: video.thumbnail_filename.clone(),
                    was_downloaded: video.status == VideoStatus::Downloaded,
                })?;
        }

        Ok(())
    }

    /// Reconciles `channel`'s output directory against its recorded
    /// downloads: heals a `Downloaded` video whose file is missing, or whose
    /// file is present but not mp4, by resetting it and scheduling a fresh
    /// download. Also resets any `Errored` video (one that permanently
    /// exhausted its download retries) the same way. Also deletes a file
    /// that doesn't belong to any currently-`Downloaded` video (an orphan).
    /// Mirrors `VideoReconciler::reconcile_filesystem`.
    fn reconcile_filesystem(&self, channel: &Channel) -> anyhow::Result<()> {
        let output_dir = self.output_dir(channel.path.as_str());
        let files = self.video_file_repository.list(&output_dir)?;
        let stored_channel_videos = self
            .channel_video_repository
            .list_for_channel(&channel.id)?;
        let stored_videos: Vec<Video> = stored_channel_videos
            .iter()
            .filter_map(|cv| self.video_repository.find(&cv.video_id).transpose())
            .collect::<anyhow::Result<Vec<Video>>>()?;
        let downloaded: Vec<&Video> = stored_videos
            .iter()
            .filter(|v| v.status == VideoStatus::Downloaded)
            .collect();
        // Every stored video's thumbnail folder is protected regardless of
        // status: a `Pending`/`InProgress` video may already have a
        // pre-fetched thumbnail on disk, ahead of its own download — see
        // the `video-thumbnails` capability.
        let protected_top_level: HashSet<&str> = downloaded
            .iter()
            .filter_map(|v| v.filename.as_deref())
            .chain(
                stored_videos
                    .iter()
                    .filter_map(|v| v.thumbnail_filename.as_deref()),
            )
            .map(top_level_entry)
            .collect();
        // Videos reset for redownload below: their in-memory `stored_videos`
        // snapshot goes stale the instant the reset is persisted, and their
        // thumbnail is expected to arrive with their own fresh download (see
        // design.md's Non-Goals) — so the recovery loop must skip them
        // rather than fetch a thumbnail for, and persist over, a video
        // object that no longer matches what's in the database.
        let mut reset_video_ids: HashSet<&VideoRecordId> = HashSet::new();

        for video in &downloaded {
            let healthy = video.filename.as_deref().is_some_and(|filename| {
                self.video_file_repository
                    .file_exists(&output_dir, filename)
                    && Path::new(filename)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
            });
            if !healthy {
                let now = self.clock.now();
                warn!(
                    channel_id = %channel.id,
                    video_id = %video.youtube_id,
                    filename = video.filename.as_deref().unwrap_or(""),
                    "downloaded video's file is missing or not mp4, resetting for redownload"
                );
                let reset = (*video).clone().reset_for_redownload(now);
                self.video_repository.update(&reset)?;
                reset_video_ids.insert(&video.id);
                self.task_repository.schedule(
                    &Task::DownloadVideo {
                        video_id: video.id.as_str().to_string(),
                        quality: channel.quality.as_str().to_string(),
                        output_dir: output_dir.to_string_lossy().to_string(),
                    },
                    now,
                )?;
                continue;
            }

            if self.video_metadata_repository.find(&video.id)?.is_some() {
                continue;
            }
            self.generate_metadata(video, &output_dir);
        }

        for video in stored_videos
            .iter()
            .filter(|v| v.status == VideoStatus::Errored)
        {
            let now = self.clock.now();
            warn!(
                channel_id = %channel.id,
                video_id = %video.youtube_id,
                "permanently errored video found during reconcile, resetting for redownload"
            );
            let reset = video.clone().reset_for_redownload(now);
            self.video_repository.update(&reset)?;
            reset_video_ids.insert(&video.id);
            self.task_repository.schedule(
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: channel.quality.as_str().to_string(),
                    output_dir: output_dir.to_string_lossy().to_string(),
                },
                now,
            )?;
        }

        for video in stored_videos
            .iter()
            .filter(|v| v.thumbnail_filename.is_none() && !reset_video_ids.contains(&v.id))
        {
            self.thumbnail_fetcher.fetch(video, &output_dir);
        }

        for file in &files {
            if protected_top_level.contains(file.as_str()) {
                continue;
            }
            if self.video_file_repository.delete(&output_dir, file)? {
                info!(channel_id = %channel.id, file, "deleted orphaned file during reconciliation");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::channel::VideoLimit;
    use crate::domain::channel_video::ChannelVideo;
    use crate::domain::playlist::PlaylistPath;
    use crate::domain::shared::{Quality, VideoId};
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_channel_repository::FakeChannelRepository;
    use crate::infrastructure::repositories::sqlite_channel_video_repository::FakeChannelVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::FakeVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::youtube_channel_videos_repository::{
        ChannelVideoListing, FakeChannelVideosRepository,
    };
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use crate::infrastructure::shared::ytdlp::FetchedThumbnail;
    use chrono::{DateTime, Utc};

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    struct Harness {
        reconciler: ChannelVideoReconciler,
        video_repository: Arc<FakeVideoRepository>,
        task_repository: Arc<FakeTaskRepository>,
        video_file_repository: Arc<FakeVideoFileRepository>,
        video_metadata_repository: Arc<FakeVideoMetadataRepository>,
        thumbnail_downloader: Arc<FakeVideoDownloaderRepository>,
    }

    fn harness(
        channel: &Channel,
        video: &Video,
        video_file_repository: FakeVideoFileRepository,
    ) -> Harness {
        harness_with_metadata(
            channel,
            video,
            video_file_repository,
            FakeYoutubeMetadataRepository::default(),
            FakeVideoMetadataRepository::default(),
        )
    }

    /// Like `harness`, but also lets a test inject specific
    /// `YoutubeMetadataRepository`/`VideoMetadataRepository` fakes, for
    /// exercising the metadata-repair loop.
    fn harness_with_metadata(
        channel: &Channel,
        video: &Video,
        video_file_repository: FakeVideoFileRepository,
        youtube_metadata_repository: FakeYoutubeMetadataRepository,
        video_metadata_repository: FakeVideoMetadataRepository,
    ) -> Harness {
        harness_with_thumbnail_downloader(
            channel,
            video,
            video_file_repository,
            youtube_metadata_repository,
            video_metadata_repository,
            FakeVideoDownloaderRepository::default(),
        )
    }

    /// Like `harness_with_metadata`, but also lets a test inject a specific
    /// `FakeVideoDownloaderRepository` to exercise the thumbnail-fetch call
    /// sites (video creation, missing-thumbnail recovery).
    #[allow(clippy::too_many_arguments)]
    fn harness_with_thumbnail_downloader(
        channel: &Channel,
        video: &Video,
        video_file_repository: FakeVideoFileRepository,
        youtube_metadata_repository: FakeYoutubeMetadataRepository,
        video_metadata_repository: FakeVideoMetadataRepository,
        thumbnail_downloader: FakeVideoDownloaderRepository,
    ) -> Harness {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        channel_repository.insert(channel).unwrap();

        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository.save(video).unwrap();

        let channel_video_repository = Arc::new(FakeChannelVideoRepository::default());
        channel_video_repository
            .save(&ChannelVideo::create(
                channel.id.clone(),
                video.id.clone(),
                0,
                fixed_timestamp(),
            ))
            .unwrap();
        channel_video_repository.register_youtube_id(&video.id, &video.youtube_id);

        let task_repository = Arc::new(FakeTaskRepository::default());
        let video_file_repository = Arc::new(video_file_repository);
        let video_metadata_repository = Arc::new(video_metadata_repository);
        let channel_videos_repository = Arc::new(FakeChannelVideosRepository::with_videos(vec![
            ChannelVideoListing {
                youtube_id: video.youtube_id.as_str().to_string(),
                title: video.title.clone(),
                position: 0,
            },
        ]));
        let thumbnail_downloader = Arc::new(thumbnail_downloader);
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            thumbnail_downloader.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));
        let event_publisher = Arc::new(FakeEventPublisher::default());

        let reconciler = ChannelVideoReconciler::new(
            channel_repository,
            video_repository.clone(),
            channel_video_repository,
            channel_videos_repository,
            Arc::new(youtube_metadata_repository),
            video_metadata_repository.clone(),
            event_publisher.clone(),
            task_repository.clone(),
            video_file_repository.clone(),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );

        Harness {
            reconciler,
            video_repository,
            task_repository,
            video_file_repository,
            video_metadata_repository,
            thumbnail_downloader,
        }
    }

    fn channel() -> Channel {
        Channel::create(
            ChannelHandle::new("@mychannel").unwrap(),
            "My Channel",
            "UC1",
            Quality::High,
            VideoLimit::new(10).unwrap(),
            PlaylistPath::new("my-channel").unwrap(),
            None,
            fixed_timestamp(),
        )
    }

    fn downloaded_video(filename: &str, thumbnail_filename: Option<&str>) -> Video {
        Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp()).mark_downloaded(
            Quality::High,
            filename,
            thumbnail_filename.map(str::to_string),
            None,
            fixed_timestamp(),
        )
    }

    #[test]
    fn it_should_judge_a_new_style_video_healthy_via_file_exists_without_a_top_level_listing() {
        let channel = channel();
        let video = downloaded_video("My Video/My Video.mp4", None);
        let harness = harness(
            &channel,
            &video,
            FakeVideoFileRepository::with_file_exists(true),
        );

        harness
            .reconciler
            .force_reconcile(channel.id.clone())
            .unwrap();

        let found = harness.video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::Downloaded);
        assert!(harness.task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_judge_a_legacy_flat_video_healthy_via_file_exists() {
        let channel = channel();
        let video = downloaded_video("My Video.mp4", None);
        let harness = harness(
            &channel,
            &video,
            FakeVideoFileRepository::with_file_exists(true),
        );

        harness
            .reconciler
            .force_reconcile(channel.id.clone())
            .unwrap();

        let found = harness.video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::Downloaded);
        assert!(harness.task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_not_sweep_a_healthy_new_style_videos_folder_as_orphaned() {
        let channel = channel();
        let video = downloaded_video("My Video/My Video.mp4", None);
        let video_file_repository = FakeVideoFileRepository {
            file_exists_result: std::sync::Mutex::new(Some(true)),
            list_result: std::sync::Mutex::new(Some(Ok(vec!["My Video".to_string()]))),
            ..Default::default()
        };
        let harness = harness(&channel, &video, video_file_repository);

        harness
            .reconciler
            .force_reconcile(channel.id.clone())
            .unwrap();

        assert!(
            harness
                .video_file_repository
                .deleted_calls
                .lock()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn it_should_sweep_a_genuinely_orphaned_folder_during_reconciliation() {
        let channel = channel();
        let video = downloaded_video("My Video/My Video.mp4", None);
        let video_file_repository = FakeVideoFileRepository {
            file_exists_result: std::sync::Mutex::new(Some(true)),
            list_result: std::sync::Mutex::new(Some(Ok(vec![
                "My Video".to_string(),
                "Orphan Video".to_string(),
            ]))),
            ..Default::default()
        };
        let harness = harness(&channel, &video, video_file_repository);

        harness
            .reconciler
            .force_reconcile(channel.id.clone())
            .unwrap();

        let deleted_calls = harness.video_file_repository.deleted_calls.lock().unwrap();
        let deleted_names: Vec<&str> = deleted_calls
            .iter()
            .map(|(_, name)| name.as_str())
            .collect();
        assert_eq!(deleted_names, vec!["Orphan Video"]);
    }

    fn fake_youtube_metadata(title: &str) -> FakeYoutubeMetadataRepository {
        FakeYoutubeMetadataRepository {
            metadata: Some(
                crate::infrastructure::repositories::youtube_metadata_repository::YoutubeMetadata {
                    title: title.to_string(),
                    description: "A description".to_string(),
                    channel_title: "My Channel".to_string(),
                    published_at: fixed_timestamp(),
                    tags: Vec::new(),
                    category_id: None,
                },
            ),
        }
    }

    #[test]
    fn it_should_regenerate_metadata_for_a_healthy_downloaded_video_with_no_recorded_metadata() {
        let channel = channel();
        let video = downloaded_video("My Video/My Video.mp4", None);
        let harness = harness_with_metadata(
            &channel,
            &video,
            FakeVideoFileRepository::with_file_exists(true),
            fake_youtube_metadata("My Video"),
            FakeVideoMetadataRepository::default(),
        );

        harness
            .reconciler
            .force_reconcile(channel.id.clone())
            .unwrap();

        let found = harness.video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::Downloaded);
        assert!(harness.task_repository.scheduled.lock().unwrap().is_empty());
        let saved = harness
            .video_metadata_repository
            .find(&video.id)
            .unwrap()
            .unwrap();
        // A channel-tracked video's recency position is never used for
        // sorttitle, so it always resolves via the publish-date branch.
        assert_eq!(saved.sorttitle, "20231114 My Video");
    }

    #[test]
    fn it_should_leave_already_recorded_metadata_untouched_during_reconcile() {
        let channel = channel();
        let video = downloaded_video("My Video/My Video.mp4", None);
        let video_metadata_repository = FakeVideoMetadataRepository::default();
        let existing = crate::domain::video_metadata::VideoMetadata::new(
            "Stale Title",
            "Stale plot",
            "Stale Channel",
            "Stale Channel",
            "2020-01-01",
            2020,
            None,
            Vec::new(),
            "yt1",
            None,
            "0000 Stale Title",
        );
        video_metadata_repository
            .save(
                &video.id,
                &existing,
                Path::new("/videos/my-channel/My Video"),
            )
            .unwrap();
        let harness = harness_with_metadata(
            &channel,
            &video,
            FakeVideoFileRepository::with_file_exists(true),
            // A fake that would produce different metadata if it were
            // (wrongly) called again — proving the row above is untouched.
            fake_youtube_metadata("Fresh Title"),
            video_metadata_repository,
        );

        harness
            .reconciler
            .force_reconcile(channel.id.clone())
            .unwrap();

        let saved = harness
            .video_metadata_repository
            .find(&video.id)
            .unwrap()
            .unwrap();
        assert_eq!(saved.title, "Stale Title");
    }

    /// Wires a `ChannelVideoReconciler` directly (rather than via `harness`,
    /// which seeds an already-stored video) so this test can exercise
    /// `sync_channel_membership`'s newly-added-video path against an empty
    /// `VideoRepository`.
    fn membership_harness(
        channel: &Channel,
        current_videos: Vec<ChannelVideoListing>,
        thumbnail_downloader: FakeVideoDownloaderRepository,
    ) -> (
        ChannelVideoReconciler,
        Arc<FakeVideoRepository>,
        Arc<FakeEventPublisher>,
        Arc<FakeVideoDownloaderRepository>,
    ) {
        let channel_repository = Arc::new(FakeChannelRepository::default());
        channel_repository.insert(channel).unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        let channel_video_repository = Arc::new(FakeChannelVideoRepository::default());
        let channel_videos_repository =
            Arc::new(FakeChannelVideosRepository::with_videos(current_videos));
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let thumbnail_downloader = Arc::new(thumbnail_downloader);
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            thumbnail_downloader.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));

        let reconciler = ChannelVideoReconciler::new(
            channel_repository,
            video_repository.clone(),
            channel_video_repository,
            channel_videos_repository,
            Arc::new(FakeYoutubeMetadataRepository::default()),
            Arc::new(FakeVideoMetadataRepository::default()),
            event_publisher.clone(),
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeVideoFileRepository::default()),
            thumbnail_fetcher,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );

        (
            reconciler,
            video_repository,
            event_publisher,
            thumbnail_downloader,
        )
    }

    #[test]
    fn it_should_fetch_a_thumbnail_for_a_newly_added_video_before_publishing_its_event() {
        let channel = channel();
        let downloader = FakeVideoDownloaderRepository::default().with_thumbnail_result(Some(
            FetchedThumbnail {
                folder: "My Video".to_string(),
                filename: "My Video.jpg".to_string(),
            },
        ));
        let (reconciler, video_repository, event_publisher, thumbnail_downloader) =
            membership_harness(
                &channel,
                vec![ChannelVideoListing {
                    youtube_id: "yt1".to_string(),
                    title: "My Video".to_string(),
                    position: 0,
                }],
                downloader,
            );

        reconciler.force_reconcile(channel.id.clone()).unwrap();

        assert_eq!(thumbnail_downloader.thumbnail_calls_count(), 1);
        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(
            stored[0].thumbnail_filename,
            Some("My Video/My Video.jpg".to_string())
        );
        let published = event_publisher.published.lock().unwrap();
        assert!(matches!(
            published.as_slice(),
            [DomainEvent::VideoAddedToChannel { .. }]
        ));
    }

    #[test]
    fn it_should_still_persist_and_publish_when_the_thumbnail_fetch_fails() {
        let channel = channel();
        let downloader = FakeVideoDownloaderRepository::default().with_thumbnail_error();
        let (reconciler, video_repository, event_publisher, _thumbnail_downloader) =
            membership_harness(
                &channel,
                vec![ChannelVideoListing {
                    youtube_id: "yt1".to_string(),
                    title: "My Video".to_string(),
                    position: 0,
                }],
                downloader,
            );

        reconciler.force_reconcile(channel.id.clone()).unwrap();

        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].thumbnail_filename, None);
        let published = event_publisher.published.lock().unwrap();
        assert!(matches!(
            published.as_slice(),
            [DomainEvent::VideoAddedToChannel { .. }]
        ));
    }

    #[test]
    fn it_should_protect_a_pending_videos_prefetched_thumbnail_folder_from_the_orphan_sweep() {
        let channel = channel();
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp())
            .with_thumbnail("My Video/My Video.jpg", fixed_timestamp());
        let video_file_repository = FakeVideoFileRepository {
            file_exists_result: std::sync::Mutex::new(Some(true)),
            list_result: std::sync::Mutex::new(Some(Ok(vec!["My Video".to_string()]))),
            ..Default::default()
        };
        let harness = harness(&channel, &video, video_file_repository);

        harness
            .reconciler
            .force_reconcile(channel.id.clone())
            .unwrap();

        assert!(
            harness
                .video_file_repository
                .deleted_calls
                .lock()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn it_should_fetch_a_missing_thumbnail_during_reconcile() {
        let channel = channel();
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp());
        let downloader = FakeVideoDownloaderRepository::default().with_thumbnail_result(Some(
            FetchedThumbnail {
                folder: "My Video".to_string(),
                filename: "My Video.jpg".to_string(),
            },
        ));
        let harness = harness_with_thumbnail_downloader(
            &channel,
            &video,
            FakeVideoFileRepository::default(),
            FakeYoutubeMetadataRepository::default(),
            FakeVideoMetadataRepository::default(),
            downloader,
        );

        harness
            .reconciler
            .force_reconcile(channel.id.clone())
            .unwrap();

        assert_eq!(harness.thumbnail_downloader.thumbnail_calls_count(), 1);
        let found = harness.video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(
            found.thumbnail_filename,
            Some("My Video/My Video.jpg".to_string())
        );
    }

    #[test]
    fn it_should_not_refetch_a_thumbnail_the_video_already_has() {
        let channel = channel();
        let video = downloaded_video("My Video/My Video.mp4", Some("My Video/My Video.jpg"));
        let harness = harness_with_thumbnail_downloader(
            &channel,
            &video,
            FakeVideoFileRepository::with_file_exists(true),
            FakeYoutubeMetadataRepository::default(),
            FakeVideoMetadataRepository::default(),
            FakeVideoDownloaderRepository::default(),
        );

        harness
            .reconciler
            .force_reconcile(channel.id.clone())
            .unwrap();

        assert_eq!(harness.thumbnail_downloader.thumbnail_calls_count(), 0);
    }

    #[test]
    fn it_should_not_fetch_a_thumbnail_for_a_video_reset_for_redownload_in_the_same_pass() {
        // Missing file (default `file_exists` is false) with no recorded
        // thumbnail: `reconcile_filesystem` resets this video for
        // redownload. The missing-thumbnail recovery loop must not then
        // fetch a thumbnail for it (its own redownload will bring one) or,
        // worse, persist a stale copy of the video over the reset — see
        // design.md's Non-Goals.
        let channel = channel();
        let video = downloaded_video("My Video/My Video.mp4", None);
        let harness = harness_with_thumbnail_downloader(
            &channel,
            &video,
            FakeVideoFileRepository::default(),
            FakeYoutubeMetadataRepository::default(),
            FakeVideoMetadataRepository::default(),
            FakeVideoDownloaderRepository::default().with_thumbnail_result(Some(
                FetchedThumbnail {
                    folder: "My Video".to_string(),
                    filename: "My Video.jpg".to_string(),
                },
            )),
        );

        harness
            .reconciler
            .force_reconcile(channel.id.clone())
            .unwrap();

        assert_eq!(harness.thumbnail_downloader.thumbnail_calls_count(), 0);
        let found = harness.video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::Pending);
        assert_eq!(found.filename, None);
    }
}
