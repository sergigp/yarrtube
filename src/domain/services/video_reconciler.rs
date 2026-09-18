use crate::domain::event::DomainEvent;
use crate::domain::playlist::{Playlist, PlaylistKind};
use crate::domain::playlist_video::PlaylistVideo;
use crate::domain::shared::{PlaylistId, VideoId};
use crate::domain::task::Task;
use crate::domain::video::{Video, VideoStatus, top_level_entry};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubePlaylistItemsRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::HashSet;
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
    event_publisher: Arc<dyn EventPublisher>,
    task_repository: Arc<dyn TaskRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
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
        event_publisher: Arc<dyn EventPublisher>,
        task_repository: Arc<dyn TaskRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        clock: Arc<dyn Clock>,
        reconcile_interval_seconds: i64,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            playlist_repository,
            video_repository,
            playlist_video_repository,
            youtube_playlist_items_repository,
            event_publisher,
            task_repository,
            video_file_repository,
            clock,
            reconcile_interval_seconds,
            videos_path: videos_path.into(),
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
        let output_dir = Path::new(&self.videos_path).join(playlist.path.as_str());
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
        let protected_top_level: HashSet<&str> = downloaded
            .iter()
            .flat_map(|v| [v.filename.as_deref(), v.thumbnail_filename.as_deref()])
            .flatten()
            .map(top_level_entry)
            .collect();

        for video in &downloaded {
            let healthy = video.filename.as_deref().is_some_and(|filename| {
                self.video_file_repository
                    .file_exists(&output_dir, filename)
                    && Path::new(filename)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
            });
            if healthy {
                continue;
            }

            let now = self.clock.now();
            warn!(
                playlist_id = %playlist.id,
                video_id = %video.youtube_id,
                filename = video.filename.as_deref().unwrap_or(""),
                "downloaded video's file is missing or not mp4, resetting for redownload"
            );
            let reset = (*video).clone().reset_for_redownload(now);
            self.video_repository.update(&reset)?;
            self.task_repository.schedule(
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: playlist.quality.as_str().to_string(),
                    output_dir: output_dir.to_string_lossy().to_string(),
                },
                now,
            )?;
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
            self.task_repository.schedule(
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: playlist.quality.as_str().to_string(),
                    output_dir: output_dir.to_string_lossy().to_string(),
                },
                now,
            )?;
        }

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
    use crate::infrastructure::repositories::sqlite_video_repository::FakeVideoRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
    use crate::infrastructure::shared::domain_events::event_publisher::FakeEventPublisher;
    use crate::infrastructure::shared::system_clock::FixedClock;
    use chrono::{DateTime, Utc};

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    struct Harness {
        reconciler: VideoReconciler,
        video_repository: Arc<FakeVideoRepository>,
        task_repository: Arc<FakeTaskRepository>,
        video_file_repository: Arc<FakeVideoFileRepository>,
    }

    fn harness(
        playlist: &Playlist,
        video: &Video,
        video_file_repository: FakeVideoFileRepository,
    ) -> Harness {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository.insert(playlist).unwrap();

        let video_repository = Arc::new(FakeVideoRepository::default());
        video_repository.save(video).unwrap();

        let playlist_video_repository = Arc::new(FakePlaylistVideoRepository::default());
        playlist_video_repository
            .save(&PlaylistVideo::create(
                playlist.id.clone(),
                video.id.clone(),
                fixed_timestamp(),
            ))
            .unwrap();

        let task_repository = Arc::new(FakeTaskRepository::default());
        let video_file_repository = Arc::new(video_file_repository);

        let reconciler = VideoReconciler::new(
            playlist_repository,
            video_repository.clone(),
            playlist_video_repository,
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            task_repository.clone(),
            video_file_repository.clone(),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );

        Harness {
            reconciler,
            video_repository,
            task_repository,
            video_file_repository,
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
}
