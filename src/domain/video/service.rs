use super::errors::{AddVideoToCustomPlaylistError, RemoveVideoFromPlaylistError};
use super::video::Video;
use super::video_filename::VideoFilename;
use super::video_status::VideoStatus;
use crate::domain::event::DomainEvent;
use crate::domain::playlist::{Playlist, PlaylistKind};
use crate::domain::shared::{PlaylistId, Quality, VideoId};
use crate::domain::task::Task;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubePlaylistItemsRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::VideoDownloaderRepository;
use crate::infrastructure::repositories::youtube_video_repository::YoutubeVideoRepository;
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::system_clock::Clock;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Orchestrates every operation on the video aggregate. Injected with only the
/// ports video operations actually use — not every port the application has.
#[derive(Clone)]
pub struct VideoService {
    playlist_repository: Arc<dyn PlaylistRepository>,
    video_repository: Arc<dyn VideoRepository>,
    youtube_playlist_items_repository: Arc<dyn YoutubePlaylistItemsRepository>,
    youtube_video_repository: Arc<dyn YoutubeVideoRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    task_repository: Arc<dyn TaskRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    clock: Arc<dyn Clock>,
    reconcile_interval_seconds: i64,
    videos_path: String,
}

impl VideoService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_repository: Arc<dyn VideoRepository>,
        youtube_playlist_items_repository: Arc<dyn YoutubePlaylistItemsRepository>,
        youtube_video_repository: Arc<dyn YoutubeVideoRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        task_repository: Arc<dyn TaskRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        clock: Arc<dyn Clock>,
        reconcile_interval_seconds: i64,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            playlist_repository,
            video_repository,
            youtube_playlist_items_repository,
            youtube_video_repository,
            event_publisher,
            task_repository,
            video_downloader_repository,
            video_file_repository,
            clock,
            reconcile_interval_seconds,
            videos_path: videos_path.into(),
        }
    }

    /// Runs one reconcile pass for a playlist: diffs membership against
    /// YouTube when it's `YoutubeLinked` (a no-op for `Custom`), always
    /// reconciles the filesystem against recorded downloads, then always
    /// reschedules the next pass — no-ops entirely if the playlist no longer
    /// exists. Called by the recurring `ReconcilePlaylistTask` and by the
    /// one-shot `PlaylistCreated` reaction alike.
    pub fn reconcile_playlist(&self, id: PlaylistId) -> anyhow::Result<()> {
        let Some(playlist) = self.playlist_repository.find(&id)? else {
            debug!(playlist_id = %id, "playlist no longer exists, skipping reconcile");
            return Ok(());
        };

        info!(playlist_id = %id, kind = %playlist.kind, "reconciling playlist");

        if playlist.kind == PlaylistKind::YoutubeLinked {
            self.sync_playlist_membership(&playlist)?;
        }

        self.reconcile_filesystem(&playlist)?;

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

    /// Diffs a YouTube-linked playlist's stored videos against YouTube's
    /// playlist-items API: adds newly-seen videos as `PENDING`, deletes
    /// stored videos no longer present on YouTube.
    fn sync_playlist_membership(&self, playlist: &Playlist) -> anyhow::Result<()> {
        let id = &playlist.id;
        let current_videos = self
            .youtube_playlist_items_repository
            .list_current_videos(id)?;
        let stored_videos = self.video_repository.list_for_playlist(id)?;

        let now = self.clock.now();
        let mut current_ids = Vec::with_capacity(current_videos.len());
        for video in &current_videos {
            let video_id = VideoId::new(&video.video_id)?;
            let existing = self.video_repository.find(id, &video_id)?;
            let is_new = existing.is_none();

            let to_save = match existing {
                None => {
                    info!(
                        playlist_id = %id,
                        video_id = %video_id,
                        title = %video.title,
                        "added video to playlist"
                    );
                    Video::create(id.clone(), video_id.clone(), video.title.clone(), now)
                }
                Some(stored) => Video {
                    title: video.title.clone(),
                    updated_at: now,
                    ..stored
                },
            };
            self.video_repository.save(&to_save)?;

            if is_new {
                self.event_publisher.publish(&DomainEvent::VideoAdded {
                    playlist_id: id.as_str().to_string(),
                    video_id: video_id.as_str().to_string(),
                })?;
            }

            current_ids.push(video_id);
        }

        let current_id_strs: HashSet<&str> = current_ids.iter().map(|v| v.as_str()).collect();
        for stored in &stored_videos {
            if !current_id_strs.contains(stored.video_id.as_str()) {
                info!(
                    playlist_id = %id,
                    video_id = %stored.video_id,
                    "removing video from playlist (no longer on YouTube)"
                );
                self.video_repository.delete(id, &stored.video_id)?;
                self.event_publisher.publish(&DomainEvent::VideoDeleted {
                    playlist_id: id.as_str().to_string(),
                    video_id: stored.video_id.as_str().to_string(),
                    title: stored.title.clone(),
                    filename: stored.filename.clone(),
                    was_downloaded: stored.status == VideoStatus::Downloaded,
                })?;
            }
        }

        Ok(())
    }

    /// Reconciles `playlist`'s output directory against its recorded
    /// downloads: heals a `Downloaded` video whose file is missing (reset it
    /// and schedule a fresh download), and deletes a file that doesn't
    /// belong to any currently-`Downloaded` video (an orphan).
    pub fn reconcile_filesystem(&self, playlist: &Playlist) -> anyhow::Result<()> {
        let output_dir = Path::new(&self.videos_path).join(playlist.path.as_str());
        let files = self.video_file_repository.list(&output_dir)?;
        let stored_videos = self.video_repository.list_for_playlist(&playlist.id)?;
        let downloaded: Vec<&Video> = stored_videos
            .iter()
            .filter(|v| v.status == VideoStatus::Downloaded)
            .collect();
        let downloaded_filenames: HashSet<&str> = downloaded
            .iter()
            .filter_map(|v| v.filename.as_deref())
            .collect();

        for video in &downloaded {
            let file_present = video
                .filename
                .as_deref()
                .is_some_and(|filename| files.iter().any(|f| f == filename));
            if file_present {
                continue;
            }

            let now = self.clock.now();
            warn!(
                playlist_id = %playlist.id,
                video_id = %video.video_id,
                "downloaded video's file is missing from disk, resetting for redownload"
            );
            let reset = (*video).clone().reset_for_redownload(now);
            self.video_repository.update(&reset)?;
            self.task_repository.schedule(
                &Task::DownloadVideo {
                    playlist_id: playlist.id.as_str().to_string(),
                    video_id: video.video_id.as_str().to_string(),
                    quality: playlist.quality.as_str().to_string(),
                },
                now,
            )?;
        }

        for file in &files {
            if downloaded_filenames.contains(file.as_str()) {
                continue;
            }
            if self.video_file_repository.delete(&output_dir, file)? {
                info!(playlist_id = %playlist.id, file, "deleted orphaned file during reconciliation");
            }
        }

        Ok(())
    }

    /// Downloads one video via `yt-dlp`, transitioning it through
    /// in-progress to downloaded/errored. No-ops (without touching status)
    /// if the playlist or the video no longer exist, since either means the
    /// download no longer needs to happen. Returns `Err` on a failed
    /// download so the task queue retries/dead-letters it.
    pub fn download_video(
        &self,
        playlist_id: PlaylistId,
        video_id: VideoId,
        quality: Quality,
        is_last_attempt: bool,
    ) -> anyhow::Result<()> {
        let Some(playlist) = self.playlist_repository.find(&playlist_id)? else {
            debug!(playlist_id = %playlist_id, "playlist no longer exists, skipping download");
            return Ok(());
        };
        let Some(video) = self.video_repository.find(&playlist_id, &video_id)? else {
            debug!(playlist_id = %playlist_id, video_id = %video_id, "video no longer exists, skipping download");
            return Ok(());
        };

        let filename = VideoFilename::from_title(&video.title);
        let started = video.start_download(self.clock.now());
        self.video_repository.update(&started)?;

        let output_dir = Path::new(&self.videos_path).join(playlist.path.as_str());
        info!(playlist_id = %playlist_id, video_id = %video_id, "downloading video");
        let outcome = self.video_downloader_repository.download(
            &video_id.to_url(),
            filename.as_str(),
            video_id.as_str(),
            quality,
            &output_dir,
        );

        match outcome {
            Ok(Some(downloaded_filename)) => {
                self.video_repository.update(&started.mark_downloaded(
                    quality,
                    downloaded_filename,
                    self.clock.now(),
                ))?;
                info!(playlist_id = %playlist_id, video_id = %video_id, "video downloaded");
                Ok(())
            }
            Ok(None) => {
                let updated = if is_last_attempt {
                    started.mark_errored(self.clock.now())
                } else {
                    started.mark_errored_retrying(self.clock.now())
                };
                self.video_repository.update(&updated)?;
                warn!(playlist_id = %playlist_id, video_id = %video_id, "yt-dlp reported a failed download");
                Err(anyhow::anyhow!(
                    "yt-dlp failed to download video {video_id}"
                ))
            }
            Err(e) => {
                let updated = if is_last_attempt {
                    started.mark_errored(self.clock.now())
                } else {
                    started.mark_errored_retrying(self.clock.now())
                };
                self.video_repository.update(&updated)?;
                error!(playlist_id = %playlist_id, video_id = %video_id, error = %e, "video download errored");
                Err(e)
            }
        }
    }

    /// Deletes a removed video's downloaded file from disk, scheduled by
    /// `subscribers::delete_video_file_on_video_deleted` whenever a
    /// downloaded video is removed from its playlist. No-ops (without
    /// erroring) if the video has no recorded filename, the playlist no
    /// longer exists, or no matching file is found, so the task is safe to
    /// retry.
    pub fn delete_video_file(
        &self,
        playlist_id: PlaylistId,
        video_id: VideoId,
        filename: Option<String>,
    ) -> anyhow::Result<()> {
        let Some(filename) = filename else {
            debug!(playlist_id = %playlist_id, video_id = %video_id, "video has no recorded filename, skipping file deletion");
            return Ok(());
        };
        let Some(playlist) = self.playlist_repository.find(&playlist_id)? else {
            debug!(playlist_id = %playlist_id, "playlist no longer exists, skipping file deletion");
            return Ok(());
        };

        let output_dir = Path::new(&self.videos_path).join(playlist.path.as_str());
        let deleted = self.video_file_repository.delete(&output_dir, &filename)?;

        if deleted {
            info!(playlist_id = %playlist_id, video_id = %video_id, "deleted video file");
        } else {
            debug!(playlist_id = %playlist_id, video_id = %video_id, "no matching video file found to delete");
        }

        Ok(())
    }

    /// Adds a video to a custom playlist, confirming it exists and is
    /// accessible on YouTube (and fetching its title from there) before
    /// persisting. No-ops if the video is already stored for this playlist.
    pub fn add_video_to_custom_playlist(
        &self,
        playlist_id: PlaylistId,
        video_id: VideoId,
    ) -> Result<(), AddVideoToCustomPlaylistError> {
        let playlist = self
            .playlist_repository
            .find(&playlist_id)
            .map_err(AddVideoToCustomPlaylistError::Repository)?
            .ok_or_else(|| AddVideoToCustomPlaylistError::PlaylistNotFound(playlist_id.clone()))?;
        if playlist.kind != PlaylistKind::Custom {
            return Err(AddVideoToCustomPlaylistError::NotCustomPlaylist(
                playlist_id,
            ));
        }

        if self
            .video_repository
            .find(&playlist_id, &video_id)
            .map_err(AddVideoToCustomPlaylistError::Repository)?
            .is_some()
        {
            return Ok(());
        }

        let youtube_video = self
            .youtube_video_repository
            .find(&video_id)
            .map_err(AddVideoToCustomPlaylistError::Lookup)?
            .ok_or_else(|| AddVideoToCustomPlaylistError::YoutubeVideoNotFound(video_id.clone()))?;

        let now = self.clock.now();
        let video = Video::create(
            playlist_id.clone(),
            video_id.clone(),
            youtube_video.title,
            now,
        );
        self.video_repository
            .save(&video)
            .map_err(AddVideoToCustomPlaylistError::Repository)?;
        self.event_publisher
            .publish(&DomainEvent::VideoAdded {
                playlist_id: playlist_id.as_str().to_string(),
                video_id: video_id.as_str().to_string(),
            })
            .map_err(AddVideoToCustomPlaylistError::Repository)?;
        info!(playlist_id = %playlist_id, video_id = %video_id, "added video to custom playlist");
        Ok(())
    }

    /// Removes a video from a custom playlist, hard-deleting its stored
    /// record and publishing `VideoDeleted` for the existing cleanup path.
    pub fn remove_video_from_playlist(
        &self,
        playlist_id: PlaylistId,
        video_id: VideoId,
    ) -> Result<(), RemoveVideoFromPlaylistError> {
        let playlist = self
            .playlist_repository
            .find(&playlist_id)
            .map_err(RemoveVideoFromPlaylistError::Repository)?
            .ok_or_else(|| RemoveVideoFromPlaylistError::PlaylistNotFound(playlist_id.clone()))?;
        if playlist.kind != PlaylistKind::Custom {
            return Err(RemoveVideoFromPlaylistError::NotCustomPlaylist(playlist_id));
        }

        let video = self
            .video_repository
            .find(&playlist_id, &video_id)
            .map_err(RemoveVideoFromPlaylistError::Repository)?
            .ok_or_else(|| RemoveVideoFromPlaylistError::VideoNotFound(video_id.clone()))?;

        self.video_repository
            .delete(&playlist_id, &video_id)
            .map_err(RemoveVideoFromPlaylistError::Repository)?;
        self.event_publisher
            .publish(&DomainEvent::VideoDeleted {
                playlist_id: playlist_id.as_str().to_string(),
                video_id: video_id.as_str().to_string(),
                title: video.title.clone(),
                filename: video.filename.clone(),
                was_downloaded: video.status == VideoStatus::Downloaded,
            })
            .map_err(RemoveVideoFromPlaylistError::Repository)?;
        info!(playlist_id = %playlist_id, video_id = %video_id, "removed video from custom playlist");
        Ok(())
    }
}
