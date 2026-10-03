use crate::domain::event::DomainEvent;
use crate::domain::playlist::Playlist;
use crate::domain::playlist::PlaylistId;
use crate::domain::playlist_video::PlaylistVideo;
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
use crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubePlaylistItemsRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// What a membership sync changed that the filesystem pass of the same
/// reconcile must know about.
struct MembershipChanges {
    /// Videos added by the sync: their thumbnail fetch comes from their own
    /// video-added event, not from the missing-thumbnail recovery.
    added_ids: Vec<VideoRecordId>,
    /// The title each renamed video had before the sync: a download started
    /// before the rename is still writing into the folder named after it.
    previous_titles: HashMap<VideoRecordId, String>,
}

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
        let Some(playlist) = self.playlist_repository.find(&id)? else {
            debug!(playlist_id = %id, "playlist no longer exists, skipping reconcile");
            return Ok(());
        };

        self.run_reconcile_pass(&playlist)?;
        self.schedule_next_reconcile(&id)
    }

    fn force_reconcile(&self, id: PlaylistId) -> anyhow::Result<()> {
        let Some(playlist) = self.playlist_repository.find(&id)? else {
            debug!(playlist_id = %id, "playlist no longer exists, skipping reconcile");
            return Ok(());
        };

        self.run_reconcile_pass(&playlist)
    }
}

impl PlaylistVideoReconciler {
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

    /// Diffs membership against YouTube, then reconciles the filesystem
    /// against recorded downloads. Shared by `reconcile` and
    /// `force_reconcile`.
    fn run_reconcile_pass(&self, playlist: &Playlist) -> anyhow::Result<()> {
        info!(playlist_id = %playlist.id, kind = %playlist.kind, "reconciling playlist");

        let changes = self.sync_playlist_membership(playlist)?;

        self.reconcile_filesystem(playlist, &changes)
    }

    /// Diffs a YouTube-linked playlist's stored videos against YouTube's
    /// playlist-items API: adds newly-seen videos as `PENDING`, deletes
    /// stored videos no longer present on YouTube.
    fn sync_playlist_membership(&self, playlist: &Playlist) -> anyhow::Result<MembershipChanges> {
        let id = &playlist.id;
        let current_videos = self
            .youtube_playlist_items_repository
            .list_current_videos(id)?;
        let stored_videos = self.playlist_video_repository.list_for_playlist(id)?;

        let now = self.clock.now();
        let mut current_youtube_ids = Vec::with_capacity(current_videos.len());
        let mut added_ids = Vec::new();
        let mut previous_titles = HashMap::new();
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
                    added_ids.push(video.id);
                }
                Some(existing) => {
                    if let Some(video) = self.video_repository.find(&existing.video_id)?
                        && video.title != current.title
                    {
                        self.video_repository
                            .update_title(&video.id, &current.title, now)?;
                        previous_titles.insert(video.id, video.title);
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

        Ok(MembershipChanges {
            added_ids,
            previous_titles,
        })
    }

    /// Reconciles `playlist`'s output directory against its recorded
    /// downloads: heals a `Downloaded` video whose file is missing, or whose
    /// file is present but not mp4 (a stale non-mp4 container downloaded
    /// before yt-dlp was made to always remux to mp4 — see
    /// `args_for_quality`), by resetting it and scheduling a fresh download.
    /// Also resets any `Errored` video (one that permanently exhausted its
    /// download retries) the same way once it has been errored for the
    /// recovery cooldown (`Video::is_due_for_recovery`), with no limit on
    /// how many times a given video may be recovered this way. Also deletes a file that
    /// doesn't belong to any currently-`Downloaded` video (an orphan) — this
    /// is what clears out a stale non-mp4 file once its video has been
    /// redownloaded under a fresh filename.
    /// Ids of every video with a download task pending or running.
    fn video_ids_with_download_in_flight(&self) -> anyhow::Result<HashSet<String>> {
        Ok(self
            .task_repository
            .list_non_completed()?
            .iter()
            .filter_map(ScheduledTask::download_video_id)
            .collect())
    }

    fn reconcile_filesystem(
        &self,
        playlist: &Playlist,
        changes: &MembershipChanges,
    ) -> anyhow::Result<()> {
        let _downloads_in_flight = self.video_ids_with_download_in_flight()?;
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
        // The folder a download or thumbnail fetch still in flight is writing
        // into isn't recorded yet, so it is protected by its predicted name.
        let unrecorded_folders: Vec<String> = stored_videos
            .iter()
            .flat_map(|video| {
                video.unrecorded_folder_candidates(
                    changes.previous_titles.get(&video.id).map(String::as_str),
                )
            })
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
            .chain(unrecorded_folders.iter().map(String::as_str))
            .map(top_level_entry)
            .collect();
        let playlist_position_by_video: HashMap<&VideoRecordId, Option<i64>> =
            stored_playlist_videos
                .iter()
                .map(|pv| (&pv.video_id, pv.position))
                .collect();
        // Videos the missing-thumbnail recovery below must skip. A video
        // added by this same pass gets its fetch from its own video-added
        // event, so recovery only covers videos stored before the pass. A
        // video reset for redownload below gets its thumbnail with its own
        // fresh download (see design.md's Non-Goals).
        let mut skip_thumbnail_ids: HashSet<&VideoRecordId> = changes.added_ids.iter().collect();

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
                skip_thumbnail_ids.insert(&video.id);
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
            .filter(|v| v.is_due_for_recovery(self.clock.now()))
        {
            let now = self.clock.now();
            warn!(
                playlist_id = %playlist.id,
                video_id = %video.youtube_id,
                "permanently errored video found during reconcile, resetting for redownload"
            );
            let reset = video.clone().reset_for_redownload(now);
            self.video_repository.update(&reset)?;
            skip_thumbnail_ids.insert(&video.id);
            self.task_repository.schedule(
                &Task::DownloadVideo {
                    video_id: video.id.as_str().to_string(),
                    quality: playlist.quality.as_str().to_string(),
                    output_dir: output_dir.to_string_lossy().to_string(),
                },
                now,
            )?;
        }

        self.thumbnail_fetcher.schedule_missing(
            &stored_videos,
            &skip_thumbnail_ids,
            &output_dir,
        )?;

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
