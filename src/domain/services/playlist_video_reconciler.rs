use crate::domain::event::DomainEvent;
use crate::domain::playlist::Playlist;
use crate::domain::playlist::PlaylistId;
use crate::domain::playlist_video::PlaylistVideo;
use crate::domain::playlist_video::{VideoAddedToPlaylist, VideoRemovedFromPlaylist};
use crate::domain::services::{ThumbnailFetcher, ThumbnailFetcherApi};
use crate::domain::task::{ScheduledTask, Task};
use crate::domain::video::{
    Video, VideoStatus, resolve_output_dir, top_level_entry, video_dir_for_filename,
};
use crate::domain::video::{VideoId, VideoRecordId};
use crate::domain::video_metadata::{build_video_metadata, resolve_sorttitle};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_metadata_repository::YoutubeMetadataRepository;
use crate::infrastructure::repositories::youtube_playlist_items_repository::{
    YoutubePlaylistItem, YoutubePlaylistItemsRepository,
};
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Reconciles a playlist's stored videos against YouTube membership and its
/// output directory against recorded downloads.
#[derive(Clone)]
pub struct PlaylistVideoReconciler {
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

impl PlaylistVideoReconciler {
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
}

pub trait PlaylistVideoReconcilerApi: Send + Sync {
    /// Runs one reconcile pass for a playlist and reschedules the next
    /// recurring pass — no-ops entirely if the playlist no longer exists.
    /// Called by the recurring `ReconcilePlaylistTask` and by the one-shot
    /// `PlaylistCreated` reaction alike.
    fn reconcile(&self, id: PlaylistId) -> anyhow::Result<()>;

    /// Runs one reconcile pass for a playlist immediately, on demand,
    /// without touching the recurring reconcile schedule — whatever
    /// `ReconcilePlaylist` task is already pending for this playlist (from
    /// creation or the last recurring pass) is left exactly as it was. This
    /// means triggering it repeatedly never queues extra tasks. No-ops
    /// entirely if the playlist no longer exists.
    fn force_reconcile(&self, id: PlaylistId) -> anyhow::Result<()>;
}

impl PlaylistVideoReconcilerApi for PlaylistVideoReconciler {
    fn reconcile(&self, id: PlaylistId) -> anyhow::Result<()> {
        let Some(playlist) = self.find_playlist(&id)? else {
            return Ok(());
        };

        self.reconcile_playlist(&playlist)?;
        self.schedule_next_reconcile(&id)
    }

    fn force_reconcile(&self, id: PlaylistId) -> anyhow::Result<()> {
        let Some(playlist) = self.find_playlist(&id)? else {
            return Ok(());
        };

        self.reconcile_playlist(&playlist)
    }
}

impl PlaylistVideoReconciler {
    fn find_playlist(&self, id: &PlaylistId) -> anyhow::Result<Option<Playlist>> {
        let playlist = self.playlist_repository.find(id)?;
        if playlist.is_none() {
            debug!(playlist_id = %id, "playlist no longer exists, skipping reconcile");
        }
        Ok(playlist)
    }

    /// One reconcile pass: brings the stored membership in line with YouTube,
    /// then brings the output directory in line with the stored videos.
    fn reconcile_playlist(&self, playlist: &Playlist) -> anyhow::Result<()> {
        info!(playlist_id = %playlist.id, kind = %playlist.kind, "reconciling playlist");

        let membership = self.sync_membership_with_youtube(playlist)?;
        let local = self.take_local_snapshot(playlist, &membership)?;

        let redownloaded = self.redownload_broken_files(playlist, &local)?;
        self.generate_missing_metadata(&local)?;
        let recovered = self.recover_errored_videos(playlist, &local)?;

        // Added videos get their download and thumbnail from their own
        // video-added event, reset ones with their fresh download, so the
        // safety nets below skip them.
        let handled: HashSet<&VideoRecordId> = membership
            .added_ids
            .iter()
            .chain(redownloaded)
            .chain(recovered)
            .collect();
        self.reschedule_stranded_downloads(playlist, &local, &handled)?;
        self.thumbnail_fetcher
            .schedule_missing(&local.videos, &handled, &local.output_dir)?;

        self.delete_orphaned_files(playlist, &local)
    }

    /// Brings the stored membership in line with YouTube's playlist items:
    /// adds newly seen videos, refreshes the title and position of known
    /// ones, and removes stored videos no longer in the playlist.
    fn sync_membership_with_youtube(
        &self,
        playlist: &Playlist,
    ) -> anyhow::Result<MembershipChanges> {
        let current_videos = self
            .youtube_playlist_items_repository
            .list_current_videos(&playlist.id)?;
        let stored_videos = self
            .playlist_video_repository
            .list_for_playlist(&playlist.id)?;
        let now = self.clock.now();

        let mut changes = MembershipChanges::default();
        for current in &current_videos {
            let youtube_id = VideoId::new(&current.video_id)?;
            match self
                .playlist_video_repository
                .find_by_youtube_video(&playlist.id, &youtube_id)?
            {
                None => {
                    let added_id = self.add_video(playlist, youtube_id, current, now)?;
                    changes.added_ids.push(added_id);
                }
                Some(existing) => {
                    if let Some((video_id, previous_title)) =
                        self.refresh_known_video(existing, current, now)?
                    {
                        changes.previous_titles.insert(video_id, previous_title);
                    }
                }
            }
        }

        let current_youtube_ids: HashSet<&str> = current_videos
            .iter()
            .map(|current| current.video_id.as_str())
            .collect();
        self.remove_videos_not_in(playlist, &stored_videos, &current_youtube_ids)?;

        Ok(changes)
    }

    /// Stores a newly seen video as `PENDING` at its playlist position and
    /// announces it, which is what triggers its download and thumbnail.
    fn add_video(
        &self,
        playlist: &Playlist,
        youtube_id: VideoId,
        current: &YoutubePlaylistItem,
        now: DateTime<Utc>,
    ) -> anyhow::Result<VideoRecordId> {
        let video = Video::create(youtube_id, current.title.clone(), now);
        self.video_repository.save(&video)?;
        let playlist_video = PlaylistVideo::create_with_position(
            playlist.id.clone(),
            video.id.clone(),
            current.position,
            now,
        );
        self.playlist_video_repository.save(&playlist_video)?;
        info!(
            playlist_id = %playlist.id,
            video_id = %video.youtube_id,
            title = %current.title,
            "added video to playlist"
        );
        self.event_publisher
            .publish(&DomainEvent::VideoAddedToPlaylist(VideoAddedToPlaylist {
                playlist_id: playlist.id.as_str().to_string(),
                video_id: video.id.as_str().to_string(),
            }))?;

        Ok(video.id)
    }

    /// Follows a known video's title and playlist position on YouTube.
    /// Returns the video's previous title when it was renamed.
    fn refresh_known_video(
        &self,
        existing: PlaylistVideo,
        current: &YoutubePlaylistItem,
        now: DateTime<Utc>,
    ) -> anyhow::Result<Option<(VideoRecordId, String)>> {
        let mut renamed = None;
        if let Some(video) = self.video_repository.find(&existing.video_id)?
            && video.title != current.title
        {
            self.video_repository
                .update_title(&video.id, &current.title, now)?;
            renamed = Some((video.id, video.title));
        }
        if existing.position != Some(current.position) {
            self.playlist_video_repository.save(&PlaylistVideo {
                position: Some(current.position),
                created_at: now,
                ..existing
            })?;
        }

        Ok(renamed)
    }

    /// Deletes every stored video whose YouTube id is not in
    /// `current_youtube_ids` and announces its removal, which is what
    /// cleans up its files.
    fn remove_videos_not_in(
        &self,
        playlist: &Playlist,
        stored_videos: &[PlaylistVideo],
        current_youtube_ids: &HashSet<&str>,
    ) -> anyhow::Result<()> {
        for stored in stored_videos {
            let Some(video) = self.video_repository.find(&stored.video_id)? else {
                continue;
            };
            if current_youtube_ids.contains(video.youtube_id.as_str()) {
                continue;
            }

            info!(
                playlist_id = %playlist.id,
                video_id = %video.youtube_id,
                "removing video from playlist (no longer on YouTube)"
            );
            self.playlist_video_repository
                .delete(&playlist.id, &video.youtube_id)?;
            self.video_repository.delete(&video.id)?;
            self.event_publisher
                .publish(&DomainEvent::VideoRemovedFromPlaylist(
                    VideoRemovedFromPlaylist {
                        playlist_id: playlist.id.as_str().to_string(),
                        video_id: video.id.as_str().to_string(),
                        title: video.title.clone(),
                        filename: video.filename.clone(),
                        thumbnail_filename: video.thumbnail_filename.clone(),
                        was_downloaded: video.status == VideoStatus::Downloaded,
                    },
                ))?;
        }

        Ok(())
    }

    /// Snapshots the playlist's stored videos and output directory once, so
    /// every filesystem step works from the same picture taken before any of
    /// them changes something.
    fn take_local_snapshot(
        &self,
        playlist: &Playlist,
        membership: &MembershipChanges,
    ) -> anyhow::Result<LocalSnapshot> {
        let downloads_in_flight = self.video_ids_with_download_in_flight()?;
        let output_dir = resolve_output_dir(&self.videos_path, playlist.path.as_str());
        let files = self.video_file_repository.list(&output_dir)?;
        let playlist_videos = self
            .playlist_video_repository
            .list_for_playlist(&playlist.id)?;
        let videos = self.find_videos(&playlist_videos)?;
        let positions = playlist_videos
            .iter()
            .filter_map(|pv| Some((pv.video_id.clone(), pv.position?)))
            .collect();
        let broken_download_ids = videos
            .iter()
            .filter(|v| {
                v.status == VideoStatus::Downloaded && !self.has_healthy_file(v, &output_dir)
            })
            .map(|v| v.id.clone())
            .collect();
        let protected_entries = protected_entries(&videos, &membership.previous_titles);

        Ok(LocalSnapshot {
            output_dir,
            videos,
            files,
            downloads_in_flight,
            broken_download_ids,
            protected_entries,
            positions,
        })
    }

    fn video_ids_with_download_in_flight(&self) -> anyhow::Result<HashSet<String>> {
        Ok(self
            .task_repository
            .list_non_completed()?
            .iter()
            .filter_map(ScheduledTask::download_video_id)
            .collect())
    }

    fn find_videos(&self, playlist_videos: &[PlaylistVideo]) -> anyhow::Result<Vec<Video>> {
        playlist_videos
            .iter()
            .filter_map(|pv| self.video_repository.find(&pv.video_id).transpose())
            .collect()
    }

    /// A video's file is healthy when it exists and is an mp4.
    fn has_healthy_file(&self, video: &Video, output_dir: &Path) -> bool {
        video.filename.as_deref().is_some_and(|filename| {
            self.video_file_repository.file_exists(output_dir, filename)
                && Path::new(filename)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
        })
    }

    /// Resets every `Downloaded` video whose file is missing or not mp4 (a
    /// stale container downloaded before yt-dlp always remuxed to mp4, see
    /// `args_for_quality`) and schedules a fresh download for it; orphan
    /// cleanup removes the stale file once it is redownloaded under a fresh
    /// filename. Returns the ids it reset.
    fn redownload_broken_files<'a>(
        &self,
        playlist: &Playlist,
        local: &'a LocalSnapshot,
    ) -> anyhow::Result<Vec<&'a VideoRecordId>> {
        local
            .broken_downloads()
            .map(|video| {
                warn!(
                    playlist_id = %playlist.id,
                    video_id = %video.youtube_id,
                    filename = video.filename.as_deref().unwrap_or(""),
                    "downloaded video's file is missing or not mp4, resetting for redownload"
                );
                self.reset_for_redownload(playlist, &local.output_dir, video)?;
                Ok(&video.id)
            })
            .collect()
    }

    /// Repairs the metadata of every healthy `Downloaded` video that has
    /// none.
    fn generate_missing_metadata(&self, local: &LocalSnapshot) -> anyhow::Result<()> {
        for video in local.healthy_downloads() {
            if self.video_metadata_repository.find(&video.id)?.is_none() {
                self.generate_metadata(
                    video,
                    &local.output_dir,
                    local.positions.get(&video.id).copied(),
                );
            }
        }

        Ok(())
    }

    /// Regenerates `video`'s metadata (fetch `YoutubeMetadata`, resolve
    /// `sorttitle`, build `VideoMetadata`, save) the same way
    /// `VideoDownloader::download` does at download time. Any failure is
    /// logged and swallowed — a `Downloaded` video's status and file are
    /// never touched by this, and a repeated failure simply tries again on
    /// the next reconcile pass. `playlist_position` is `None` when no
    /// position is recorded, resolving `sorttitle` via publish date instead.
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
        let video_metadata = build_video_metadata(
            &video.youtube_id,
            &metadata,
            sorttitle,
            thumbnail_filename,
            self.clock.now(),
        );
        let video_dir = video_dir_for_filename(output_dir, filename);

        if let Err(e) = self
            .video_metadata_repository
            .save(&video.id, &video_metadata, &video_dir)
        {
            warn!(video_id = %video.id, error = %e, "failed to save video metadata during reconcile");
        }
    }

    /// Gives every `Errored` video (one that exhausted its download retries)
    /// another chance once its recovery cooldown is over
    /// (`Video::is_due_for_recovery`), with no limit on how many times a
    /// video may be recovered. Returns the ids it reset.
    fn recover_errored_videos<'a>(
        &self,
        playlist: &Playlist,
        local: &'a LocalSnapshot,
    ) -> anyhow::Result<Vec<&'a VideoRecordId>> {
        let now = self.clock.now();
        local
            .videos
            .iter()
            .filter(|v| v.is_due_for_recovery(now))
            .map(|video| {
                warn!(
                    playlist_id = %playlist.id,
                    video_id = %video.youtube_id,
                    "permanently errored video found during reconcile, resetting for redownload"
                );
                self.reset_for_redownload(playlist, &local.output_dir, video)?;
                Ok(&video.id)
            })
            .collect()
    }

    /// Safety net for a video still waiting to be downloaded that has no
    /// download task pending or running (e.g. lost on a crash): schedules
    /// one again. Skips the `handled` videos, already taken care of.
    fn reschedule_stranded_downloads(
        &self,
        playlist: &Playlist,
        local: &LocalSnapshot,
        handled: &HashSet<&VideoRecordId>,
    ) -> anyhow::Result<()> {
        local
            .videos
            .iter()
            .filter(|v| {
                !v.is_download_settled()
                    && !local.downloads_in_flight.contains(v.id.as_str())
                    && !handled.contains(&v.id)
            })
            .try_for_each(|video| {
                warn!(
                    playlist_id = %playlist.id,
                    video_id = %video.youtube_id,
                    status = video.status.as_str(),
                    "stranded video found during reconcile, rescheduling its download"
                );
                self.schedule_download(playlist, &local.output_dir, video, self.clock.now())
            })
    }

    /// Deletes every top-level entry of the output directory that doesn't
    /// belong to a stored video.
    fn delete_orphaned_files(
        &self,
        playlist: &Playlist,
        local: &LocalSnapshot,
    ) -> anyhow::Result<()> {
        for file in &local.files {
            if local.protected_entries.contains(file) {
                continue;
            }
            if self.video_file_repository.delete(&local.output_dir, file)? {
                info!(playlist_id = %playlist.id, file, "deleted orphaned file during reconciliation");
            }
        }

        Ok(())
    }

    fn reset_for_redownload(
        &self,
        playlist: &Playlist,
        output_dir: &Path,
        video: &Video,
    ) -> anyhow::Result<()> {
        let now = self.clock.now();
        self.video_repository
            .update(&video.clone().reset_for_redownload(now))?;
        self.schedule_download(playlist, output_dir, video, now)
    }

    fn schedule_download(
        &self,
        playlist: &Playlist,
        output_dir: &Path,
        video: &Video,
        run_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        self.task_repository.schedule(
            &Task::DownloadVideo {
                video_id: video.id.as_str().to_string(),
                quality: playlist.quality.as_str().to_string(),
                output_dir: output_dir.to_string_lossy().to_string(),
            },
            run_at,
        )
    }

    fn schedule_next_reconcile(&self, id: &PlaylistId) -> anyhow::Result<()> {
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
}

/// What a membership sync changed that the filesystem pass of the same
/// reconcile must know about.
#[derive(Default)]
struct MembershipChanges {
    /// Videos added by the sync: their thumbnail fetch comes from their own
    /// video-added event, not from the missing-thumbnail recovery.
    added_ids: Vec<VideoRecordId>,
    /// The title each renamed video had before the sync: a download started
    /// before the rename is still writing into the folder named after it.
    previous_titles: HashMap<VideoRecordId, String>,
}

/// A playlist's stored videos and output directory as one pass found them,
/// before any filesystem step changes them.
struct LocalSnapshot {
    output_dir: PathBuf,
    videos: Vec<Video>,
    files: Vec<String>,
    /// Ids of every video with a download task pending or running.
    downloads_in_flight: HashSet<String>,
    /// `Downloaded` videos whose file is missing or not mp4.
    broken_download_ids: HashSet<VideoRecordId>,
    /// Top-level entries of `output_dir` that orphan cleanup must keep.
    protected_entries: HashSet<String>,
    /// Each video's recorded playlist position, used to resolve its
    /// metadata `sorttitle`.
    positions: HashMap<VideoRecordId, i64>,
}

impl LocalSnapshot {
    fn broken_downloads(&self) -> impl Iterator<Item = &Video> {
        self.videos
            .iter()
            .filter(|v| self.broken_download_ids.contains(&v.id))
    }

    fn healthy_downloads(&self) -> impl Iterator<Item = &Video> {
        self.videos.iter().filter(|v| {
            v.status == VideoStatus::Downloaded && !self.broken_download_ids.contains(&v.id)
        })
    }
}

/// Top-level entries of the output directory orphan cleanup must keep:
/// every `Downloaded` video's file, every stored video's thumbnail (a
/// `Pending`/`InProgress` video may already have one pre-fetched, see the
/// `video-thumbnails` capability), and the folder an in-flight download or
/// thumbnail fetch is writing into — not recorded yet, so protected by its
/// predicted name.
fn protected_entries(
    videos: &[Video],
    previous_titles: &HashMap<VideoRecordId, String>,
) -> HashSet<String> {
    let downloaded_files = videos
        .iter()
        .filter(|v| v.status == VideoStatus::Downloaded)
        .filter_map(|v| v.filename.clone());
    let thumbnails = videos.iter().filter_map(|v| v.thumbnail_filename.clone());
    let unrecorded_folders = videos.iter().flat_map(|video| {
        video.unrecorded_folder_candidates(previous_titles.get(&video.id).map(String::as_str))
    });

    downloaded_files
        .chain(thumbnails)
        .chain(unrecorded_folders)
        .map(|entry| top_level_entry(&entry).to_string())
        .collect()
}
