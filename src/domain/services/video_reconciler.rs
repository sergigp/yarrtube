use crate::domain::event::DomainEvent;
use crate::domain::playlist::{Playlist, PlaylistKind};
use crate::domain::playlist_video::PlaylistVideo;
use crate::domain::services::thumbnail_fetcher::ThumbnailFetcher;
use crate::domain::shared::{PlaylistId, VideoId, VideoRecordId};
use crate::domain::task::Task;
use crate::domain::video::{
    Video, VideoStatus, resolve_output_dir, top_level_entry, video_dir_for_filename,
};
use crate::domain::video_metadata::{build_video_metadata, resolve_sorttitle};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_metadata_repository::YoutubeMetadataRepository;
use crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubePlaylistItemsRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Reconciles a playlist's stored videos against YouTube membership and its
/// output directory against recorded downloads.
#[derive(Clone)]
pub struct VideoReconciler {
    playlist_repository: Arc<dyn PlaylistRepository>,
    video_repository: Arc<dyn VideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    youtube_playlist_items_repository: Arc<dyn YoutubePlaylistItemsRepository>,
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

impl VideoReconciler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_repository: Arc<dyn VideoRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        youtube_playlist_items_repository: Arc<dyn YoutubePlaylistItemsRepository>,
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
            playlist_repository,
            video_repository,
            playlist_video_repository,
            youtube_playlist_items_repository,
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

    /// Regenerates `video`'s metadata (fetch `YoutubeMetadata`, resolve
    /// `sorttitle`, build `VideoMetadata`, save) the same way
    /// `VideoDownloader::download` does at download time. Any failure is
    /// logged and swallowed — a `Downloaded` video's status and file are
    /// never touched by this, and a repeated failure simply tries again on
    /// the next reconcile pass. `playlist_position` is `None` for a
    /// custom-playlist video, resolving `sorttitle` via publish date.
    fn generate_metadata(&self, video: &Video, output_dir: &Path, playlist_position: Option<i64>) {
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
        let sorttitle =
            resolve_sorttitle(&metadata.title, metadata.published_at, playlist_position);
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

    /// Runs one reconcile pass for a playlist and reschedules the next
    /// recurring pass — no-ops entirely if the playlist no longer exists.
    /// Called by the recurring `ReconcilePlaylistTask` and by the one-shot
    /// `PlaylistCreated` reaction alike.
    pub fn reconcile(&self, id: PlaylistId) -> anyhow::Result<()> {
        let Some(playlist) = self.playlist_repository.find(&id)? else {
            debug!(playlist_id = %id, "playlist no longer exists, skipping reconcile");
            return Ok(());
        };

        self.run_reconcile_pass(&playlist)?;

        let now = self.clock.now();
        let next_run_at = now + chrono::Duration::seconds(self.reconcile_interval_seconds);
        self.task_repository.schedule(
            &Task::ReconcilePlaylist {
                playlist_id: id.as_str().to_string(),
            },
            next_run_at,
        )?;
        info!(playlist_id = %id, next_run_at = %next_run_at, "scheduled next reconcile of playlist");

        Ok(())
    }

    /// Runs one reconcile pass for a playlist immediately, on demand,
    /// without touching the recurring reconcile schedule — whatever
    /// `ReconcilePlaylist` task is already pending for this playlist (from
    /// creation or the last recurring pass) is left exactly as it was. This
    /// means triggering it repeatedly never queues extra tasks. No-ops
    /// entirely if the playlist no longer exists.
    pub fn force_reconcile(&self, id: PlaylistId) -> anyhow::Result<()> {
        let Some(playlist) = self.playlist_repository.find(&id)? else {
            debug!(playlist_id = %id, "playlist no longer exists, skipping reconcile");
            return Ok(());
        };

        self.run_reconcile_pass(&playlist)
    }

    /// Diffs membership against YouTube when `playlist` is `YoutubeLinked`
    /// (a no-op for `Custom`), then always reconciles the filesystem against
    /// recorded downloads. Shared by `reconcile` and `force_reconcile`.
    fn run_reconcile_pass(&self, playlist: &Playlist) -> anyhow::Result<()> {
        info!(playlist_id = %playlist.id, kind = %playlist.kind, "reconciling playlist");

        if playlist.kind == PlaylistKind::YoutubeLinked {
            self.sync_playlist_membership(playlist)?;
        }

        self.reconcile_filesystem(playlist)
    }

    /// Diffs a YouTube-linked playlist's stored videos against YouTube's
    /// playlist-items API: adds newly-seen videos as `PENDING`, deletes
    /// stored videos no longer present on YouTube.
    fn sync_playlist_membership(&self, playlist: &Playlist) -> anyhow::Result<()> {
        let id = &playlist.id;
        let output_dir = resolve_output_dir(&self.videos_path, playlist.path.as_str());
        let current_videos = self
            .youtube_playlist_items_repository
            .list_current_videos(id)?;
        let stored_videos = self.playlist_video_repository.list_for_playlist(id)?;

        let now = self.clock.now();
        let mut current_youtube_ids = Vec::with_capacity(current_videos.len());
        for current in &current_videos {
            let youtube_id = VideoId::new(&current.video_id)?;
            let existing = self
                .playlist_video_repository
                .find_by_youtube_video(id, &youtube_id)?;

            match existing {
                None => {
                    let video = Video::create(youtube_id.clone(), current.title.clone(), now);
                    self.video_repository.save(&video)?;
                    let playlist_video = PlaylistVideo::create_with_position(
                        id.clone(),
                        video.id.clone(),
                        current.position,
                        now,
                    );
                    self.playlist_video_repository.save(&playlist_video)?;
                    self.thumbnail_fetcher.fetch(&video, &output_dir);
                    info!(
                        playlist_id = %id,
                        video_id = %youtube_id,
                        title = %current.title,
                        "added video to playlist"
                    );
                    self.event_publisher
                        .publish(&DomainEvent::VideoAddedToPlaylist {
                            playlist_id: id.as_str().to_string(),
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
                    if existing.position != Some(current.position) {
                        self.playlist_video_repository.save(&PlaylistVideo {
                            position: Some(current.position),
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
                playlist_id = %id,
                video_id = %video.youtube_id,
                "removing video from playlist (no longer on YouTube)"
            );
            self.playlist_video_repository
                .delete(id, &video.youtube_id)?;
            self.video_repository.delete(&video.id)?;
            self.event_publisher
                .publish(&DomainEvent::VideoRemovedFromPlaylist {
                    playlist_id: id.as_str().to_string(),
                    video_id: video.id.as_str().to_string(),
                    title: video.title.clone(),
                    filename: video.filename.clone(),
                    thumbnail_filename: video.thumbnail_filename.clone(),
                    was_downloaded: video.status == VideoStatus::Downloaded,
                })?;
        }

        Ok(())
    }

    /// Reconciles `playlist`'s output directory against its recorded
    /// downloads: heals a `Downloaded` video whose file is missing, or whose
    /// file is present but not mp4 (a stale non-mp4 container downloaded
    /// before yt-dlp was made to always remux to mp4 — see
    /// `args_for_quality`), by resetting it and scheduling a fresh download.
    /// Also resets any `Errored` video (one that permanently exhausted its
    /// download retries) the same way, with no limit on how many times a
    /// given video may be recovered this way — see design.md's "Reconcile
    /// also recovers Errored videos" decision. Also deletes a file that
    /// doesn't belong to any currently-`Downloaded` video (an orphan) — this
    /// is what clears out a stale non-mp4 file once its video has been
    /// redownloaded under a fresh filename.
    fn reconcile_filesystem(&self, playlist: &Playlist) -> anyhow::Result<()> {
        let output_dir = resolve_output_dir(&self.videos_path, playlist.path.as_str());
        let files = self.video_file_repository.list(&output_dir)?;
        let stored_playlist_videos = self
            .playlist_video_repository
            .list_for_playlist(&playlist.id)?;
        let stored_videos: Vec<Video> = stored_playlist_videos
            .iter()
            .filter_map(|pv| self.video_repository.find(&pv.video_id).transpose())
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
        let playlist_position_by_video: HashMap<&VideoRecordId, Option<i64>> =
            stored_playlist_videos
                .iter()
                .map(|pv| (&pv.video_id, pv.position))
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
                    playlist_id = %playlist.id,
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
                        quality: playlist.quality.as_str().to_string(),
                        output_dir: output_dir.to_string_lossy().to_string(),
                    },
                    now,
                )?;
                continue;
            }

            if self.video_metadata_repository.find(&video.id)?.is_some() {
                continue;
            }
            let playlist_position = playlist_position_by_video.get(&video.id).copied().flatten();
            self.generate_metadata(video, &output_dir, playlist_position);
        }

        for video in stored_videos
            .iter()
            .filter(|v| v.status == VideoStatus::Errored)
        {
            let now = self.clock.now();
            warn!(
                playlist_id = %playlist.id,
                video_id = %video.youtube_id,
                "permanently errored video found during reconcile, resetting for redownload"
            );
            let reset = video.clone().reset_for_redownload(now);
            self.video_repository.update(&reset)?;
            reset_video_ids.insert(&video.id);
            self.task_repository.schedule(
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: playlist.quality.as_str().to_string(),
                    output_dir: output_dir.to_string_lossy().to_string(),
                },
                now,
            )?;
        }

        self.thumbnail_fetcher
            .fetch_missing(&stored_videos, &reset_video_ids, &output_dir);

        for file in &files {
            if protected_top_level.contains(file.as_str()) {
                continue;
            }
            if self.video_file_repository.delete(&output_dir, file)? {
                info!(playlist_id = %playlist.id, file, "deleted orphaned file during reconciliation");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::playlist::{PlaylistKind, PlaylistName, PlaylistPath};
    use crate::domain::playlist_video::PlaylistVideo;
    use crate::domain::shared::{Quality, VideoId};
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::sqlite_playlist_repository::FakePlaylistRepository;
    use crate::infrastructure::repositories::sqlite_playlist_video_repository::FakePlaylistVideoRepository;
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_metadata_repository::FakeVideoMetadataRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::youtube_metadata_repository::FakeYoutubeMetadataRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use crate::infrastructure::shared::ytdlp::FetchedThumbnail;
    use chrono::{DateTime, Utc};

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    struct Harness {
        reconciler: VideoReconciler,
        video_repository: Arc<FakeVideoRepository>,
        task_repository: Arc<FakeTaskRepository>,
        video_file_repository: Arc<FakeVideoFileRepository>,
        video_metadata_repository: Arc<FakeVideoMetadataRepository>,
        thumbnail_downloader: Arc<FakeVideoDownloaderRepository>,
    }

    fn harness(
        playlist: &Playlist,
        video: &Video,
        video_file_repository: FakeVideoFileRepository,
    ) -> Harness {
        harness_with_metadata(
            playlist,
            video,
            None,
            video_file_repository,
            FakeYoutubeMetadataRepository::default(),
            FakeVideoMetadataRepository::default(),
        )
    }

    /// Like `harness`, but also lets a test seed the video's playlist
    /// position and inject specific `YoutubeMetadataRepository`/
    /// `VideoMetadataRepository` fakes, for exercising the metadata-repair
    /// loop.
    fn harness_with_metadata(
        playlist: &Playlist,
        video: &Video,
        playlist_position: Option<i64>,
        video_file_repository: FakeVideoFileRepository,
        youtube_metadata_repository: FakeYoutubeMetadataRepository,
        video_metadata_repository: FakeVideoMetadataRepository,
    ) -> Harness {
        harness_with_thumbnail_downloader(
            playlist,
            video,
            playlist_position,
            video_file_repository,
            youtube_metadata_repository,
            video_metadata_repository,
            FakeVideoDownloaderRepository::default(),
        )
    }

    /// Like `harness_with_metadata`, but also lets a test inject a specific
    /// `FakeVideoDownloaderRepository` to exercise the thumbnail-fetch
    /// call sites (video creation, missing-thumbnail recovery).
    #[allow(clippy::too_many_arguments)]
    fn harness_with_thumbnail_downloader(
        playlist: &Playlist,
        video: &Video,
        playlist_position: Option<i64>,
        video_file_repository: FakeVideoFileRepository,
        youtube_metadata_repository: FakeYoutubeMetadataRepository,
        video_metadata_repository: FakeVideoMetadataRepository,
        thumbnail_downloader: FakeVideoDownloaderRepository,
    ) -> Harness {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(playlist).unwrap();

        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository.save(video).unwrap();

        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let playlist_video = match playlist_position {
            Some(position) => PlaylistVideo::create_with_position(
                playlist.id.clone(),
                video.id.clone(),
                position,
                fixed_timestamp(),
            ),
            None => PlaylistVideo::create(playlist.id.clone(), video.id.clone(), fixed_timestamp()),
        };
        playlist_video_repository.save(&playlist_video).unwrap();

        let task_repository = Arc::new(FakeTaskRepository::default());
        let video_file_repository = Arc::new(video_file_repository);
        let video_metadata_repository = Arc::new(video_metadata_repository);
        let thumbnail_downloader = Arc::new(thumbnail_downloader);
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            thumbnail_downloader.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));

        let reconciler = VideoReconciler::new(
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository,
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(youtube_metadata_repository),
            video_metadata_repository.clone(),
            Arc::new(FakeEventPublisher::default()),
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

    fn custom_playlist() -> Playlist {
        Playlist::create(
            PlaylistId::new("PL1").unwrap(),
            PlaylistName::new("My Playlist").unwrap(),
            PlaylistPath::new("my-playlist").unwrap(),
            Quality::High,
            PlaylistKind::Custom,
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
        let playlist = custom_playlist();
        let video = downloaded_video("My Video/My Video.mp4", None);
        let harness = harness(
            &playlist,
            &video,
            FakeVideoFileRepository::with_file_exists(true),
        );

        harness
            .reconciler
            .force_reconcile(playlist.id.clone())
            .unwrap();

        let found = harness.video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::Downloaded);
        assert!(harness.task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_judge_a_legacy_flat_video_healthy_via_file_exists() {
        let playlist = custom_playlist();
        let video = downloaded_video("My Video.mp4", None);
        let harness = harness(
            &playlist,
            &video,
            FakeVideoFileRepository::with_file_exists(true),
        );

        harness
            .reconciler
            .force_reconcile(playlist.id.clone())
            .unwrap();

        let found = harness.video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::Downloaded);
        assert!(harness.task_repository.scheduled.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_not_sweep_a_healthy_new_style_videos_folder_as_orphaned() {
        let playlist = custom_playlist();
        let video = downloaded_video("My Video/My Video.mp4", None);
        let video_file_repository = FakeVideoFileRepository {
            file_exists_result: std::sync::Mutex::new(Some(true)),
            list_result: std::sync::Mutex::new(Some(Ok(vec!["My Video".to_string()]))),
            ..Default::default()
        };
        let harness = harness(&playlist, &video, video_file_repository);

        harness
            .reconciler
            .force_reconcile(playlist.id.clone())
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
        let playlist = custom_playlist();
        let video = downloaded_video("My Video/My Video.mp4", None);
        let video_file_repository = FakeVideoFileRepository {
            file_exists_result: std::sync::Mutex::new(Some(true)),
            list_result: std::sync::Mutex::new(Some(Ok(vec![
                "My Video".to_string(),
                "Orphan Video".to_string(),
            ]))),
            ..Default::default()
        };
        let harness = harness(&playlist, &video, video_file_repository);

        harness
            .reconciler
            .force_reconcile(playlist.id.clone())
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
        let playlist = custom_playlist();
        let video = downloaded_video("My Video/My Video.mp4", None);
        let harness = harness_with_metadata(
            &playlist,
            &video,
            Some(3),
            FakeVideoFileRepository::with_file_exists(true),
            fake_youtube_metadata("My Video"),
            FakeVideoMetadataRepository::default(),
        );

        harness
            .reconciler
            .force_reconcile(playlist.id.clone())
            .unwrap();

        let found = harness.video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::Downloaded);
        assert!(harness.task_repository.scheduled.lock().unwrap().is_empty());
        let saved = harness
            .video_metadata_repository
            .find(&video.id)
            .unwrap()
            .unwrap();
        assert_eq!(saved.sorttitle, "0003 My Video");
    }

    #[test]
    fn it_should_leave_already_recorded_metadata_untouched_during_reconcile() {
        let playlist = custom_playlist();
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
                Path::new("/videos/my-playlist/My Video"),
            )
            .unwrap();
        let harness = harness_with_metadata(
            &playlist,
            &video,
            Some(3),
            FakeVideoFileRepository::with_file_exists(true),
            // A fake that would produce different metadata if it were
            // (wrongly) called again — proving the row above is untouched.
            fake_youtube_metadata("Fresh Title"),
            video_metadata_repository,
        );

        harness
            .reconciler
            .force_reconcile(playlist.id.clone())
            .unwrap();

        let saved = harness
            .video_metadata_repository
            .find(&video.id)
            .unwrap()
            .unwrap();
        assert_eq!(saved.title, "Stale Title");
    }

    fn youtube_linked_playlist() -> Playlist {
        Playlist::create(
            PlaylistId::new("PL1").unwrap(),
            PlaylistName::new("My Playlist").unwrap(),
            PlaylistPath::new("my-playlist").unwrap(),
            Quality::High,
            PlaylistKind::YoutubeLinked,
            fixed_timestamp(),
        )
    }

    /// Wires a `VideoReconciler` directly (rather than via `harness`, which
    /// seeds a `Custom` playlist with an already-stored video) so these
    /// tests can exercise `sync_playlist_membership`'s newly-added-video
    /// path against an empty `VideoRepository`.
    #[allow(clippy::type_complexity)]
    fn membership_harness(
        playlist: &Playlist,
        current_videos: Vec<crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubePlaylistItem>,
        thumbnail_downloader: FakeVideoDownloaderRepository,
    ) -> (
        VideoReconciler,
        Arc<FakeVideoRepository>,
        Arc<FakeEventPublisher>,
        Arc<FakeVideoDownloaderRepository>,
    ) {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(playlist).unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        let youtube_playlist_items_repository = Arc::new(FakeYoutubePlaylistItemsRepository {
            videos: std::sync::Mutex::new(current_videos),
        });
        let event_publisher = Arc::new(FakeEventPublisher::default());
        let thumbnail_downloader = Arc::new(thumbnail_downloader);
        let thumbnail_fetcher = Arc::new(ThumbnailFetcher::new(
            video_repository.clone(),
            thumbnail_downloader.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
        ));

        let reconciler = VideoReconciler::new(
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository,
            youtube_playlist_items_repository,
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
        use crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubePlaylistItem;

        let playlist = youtube_linked_playlist();
        let downloader = FakeVideoDownloaderRepository::default().with_thumbnail_result(Some(
            FetchedThumbnail {
                folder: "My Video".to_string(),
                filename: "My Video.jpg".to_string(),
            },
        ));
        let (reconciler, video_repository, event_publisher, thumbnail_downloader) =
            membership_harness(
                &playlist,
                vec![YoutubePlaylistItem {
                    video_id: "yt1".to_string(),
                    title: "My Video".to_string(),
                    position: 0,
                }],
                downloader,
            );

        reconciler.force_reconcile(playlist.id.clone()).unwrap();

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
            [DomainEvent::VideoAddedToPlaylist { .. }]
        ));
    }

    #[test]
    fn it_should_still_persist_and_publish_when_the_thumbnail_fetch_fails() {
        use crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubePlaylistItem;

        let playlist = youtube_linked_playlist();
        let downloader = FakeVideoDownloaderRepository::default().with_thumbnail_error();
        let (reconciler, video_repository, event_publisher, _thumbnail_downloader) =
            membership_harness(
                &playlist,
                vec![YoutubePlaylistItem {
                    video_id: "yt1".to_string(),
                    title: "My Video".to_string(),
                    position: 0,
                }],
                downloader,
            );

        reconciler.force_reconcile(playlist.id.clone()).unwrap();

        let stored = video_repository.videos.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].thumbnail_filename, None);
        let published = event_publisher.published.lock().unwrap();
        assert!(matches!(
            published.as_slice(),
            [DomainEvent::VideoAddedToPlaylist { .. }]
        ));
    }

    #[test]
    fn it_should_protect_a_pending_videos_prefetched_thumbnail_folder_from_the_orphan_sweep() {
        let playlist = custom_playlist();
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp())
            .with_thumbnail("My Video/My Video.jpg", fixed_timestamp());
        let video_file_repository = FakeVideoFileRepository {
            file_exists_result: std::sync::Mutex::new(Some(true)),
            list_result: std::sync::Mutex::new(Some(Ok(vec!["My Video".to_string()]))),
            ..Default::default()
        };
        let harness = harness(&playlist, &video, video_file_repository);

        harness
            .reconciler
            .force_reconcile(playlist.id.clone())
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
        let playlist = custom_playlist();
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp());
        let downloader = FakeVideoDownloaderRepository::default().with_thumbnail_result(Some(
            FetchedThumbnail {
                folder: "My Video".to_string(),
                filename: "My Video.jpg".to_string(),
            },
        ));
        let harness = harness_with_thumbnail_downloader(
            &playlist,
            &video,
            None,
            FakeVideoFileRepository::default(),
            FakeYoutubeMetadataRepository::default(),
            FakeVideoMetadataRepository::default(),
            downloader,
        );

        harness
            .reconciler
            .force_reconcile(playlist.id.clone())
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
        let playlist = custom_playlist();
        let video = downloaded_video("My Video/My Video.mp4", Some("My Video/My Video.jpg"));
        let harness = harness_with_thumbnail_downloader(
            &playlist,
            &video,
            None,
            FakeVideoFileRepository::with_file_exists(true),
            FakeYoutubeMetadataRepository::default(),
            FakeVideoMetadataRepository::default(),
            FakeVideoDownloaderRepository::default(),
        );

        harness
            .reconciler
            .force_reconcile(playlist.id.clone())
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
        let playlist = custom_playlist();
        let video = downloaded_video("My Video/My Video.mp4", None);
        let harness = harness_with_thumbnail_downloader(
            &playlist,
            &video,
            None,
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
            .force_reconcile(playlist.id.clone())
            .unwrap();

        assert_eq!(harness.thumbnail_downloader.thumbnail_calls_count(), 0);
        let found = harness.video_repository.find(&video.id).unwrap().unwrap();
        assert_eq!(found.status, VideoStatus::Pending);
        assert_eq!(found.filename, None);
    }

    #[test]
    fn it_should_not_fetch_a_thumbnail_for_a_video_whose_real_download_is_in_progress() {
        // A video currently being downloaded by a concurrent `DownloadVideo`
        // task has no `filename` yet, so `existing_folder` would resolve to
        // `None` and a concurrent recovery-pass fetch would collide with the
        // in-progress download's own folder, spawning a stray sibling folder
        // that then gets permanently protected from the orphan sweep.
        let playlist = custom_playlist();
        let video = Video::create(VideoId::new("yt1").unwrap(), "My Video", fixed_timestamp())
            .start_download(fixed_timestamp());
        let harness = harness_with_thumbnail_downloader(
            &playlist,
            &video,
            None,
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
            .force_reconcile(playlist.id.clone())
            .unwrap();

        assert_eq!(harness.thumbnail_downloader.thumbnail_calls_count(), 0);
    }
}
