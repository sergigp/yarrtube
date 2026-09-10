use super::video::Video;
use super::video_filename::VideoFilename;
use super::video_status::VideoStatus;
use crate::domain::event::DomainEvent;
use crate::domain::shared::{PlaylistId, Quality, VideoId};
use crate::domain::task::Task;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubePlaylistItemsRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::VideoDownloaderRepository;
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
    event_publisher: Arc<dyn EventPublisher>,
    task_repository: Arc<dyn TaskRepository>,
    video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    clock: Arc<dyn Clock>,
    sync_interval_seconds: i64,
    videos_path: String,
}

impl VideoService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_repository: Arc<dyn VideoRepository>,
        youtube_playlist_items_repository: Arc<dyn YoutubePlaylistItemsRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        task_repository: Arc<dyn TaskRepository>,
        video_downloader_repository: Arc<dyn VideoDownloaderRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        clock: Arc<dyn Clock>,
        sync_interval_seconds: i64,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            playlist_repository,
            video_repository,
            youtube_playlist_items_repository,
            event_publisher,
            task_repository,
            video_downloader_repository,
            video_file_repository,
            clock,
            sync_interval_seconds,
            videos_path: videos_path.into(),
        }
    }

    pub fn sync_playlist_videos(&self, id: PlaylistId) -> anyhow::Result<()> {
        if self.playlist_repository.find(&id)?.is_none() {
            debug!(playlist_id = %id, "playlist no longer exists, skipping sync");
            return Ok(());
        }

        info!(playlist_id = %id, "syncing playlist");
        let current_videos = self
            .youtube_playlist_items_repository
            .list_current_videos(&id)?;
        let stored_videos = self.video_repository.list_for_playlist(&id)?;

        let now = self.clock.now();
        let mut current_ids = Vec::with_capacity(current_videos.len());
        for video in &current_videos {
            let video_id = VideoId::new(&video.video_id)?;
            let existing = self.video_repository.find(&id, &video_id)?;
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
                self.video_repository.delete(&id, &stored.video_id)?;
                self.event_publisher.publish(&DomainEvent::VideoDeleted {
                    playlist_id: id.as_str().to_string(),
                    video_id: stored.video_id.as_str().to_string(),
                    title: stored.title.clone(),
                    was_downloaded: stored.status == VideoStatus::Downloaded,
                })?;
            }
        }

        let next_run_at = now + chrono::Duration::seconds(self.sync_interval_seconds);
        self.task_repository.schedule(
            &Task::SyncPlaylist {
                playlist_id: id.as_str().to_string(),
            },
            next_run_at,
        )?;
        info!(playlist_id = %id, next_run_at = %next_run_at, "scheduled next sync of playlist");

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

        let (succeeded, error) = match outcome {
            Ok(succeeded) => (succeeded, None),
            Err(e) => (false, Some(e)),
        };

        if succeeded {
            self.video_repository
                .update(&started.mark_downloaded(quality, self.clock.now()))?;
            info!(playlist_id = %playlist_id, video_id = %video_id, "video downloaded");
            return Ok(());
        }

        let updated = if is_last_attempt {
            started.mark_errored(self.clock.now())
        } else {
            started.mark_errored_retrying(self.clock.now())
        };
        self.video_repository.update(&updated)?;

        match error {
            Some(e) => {
                error!(playlist_id = %playlist_id, video_id = %video_id, error = %e, "video download errored");
                Err(e)
            }
            None => {
                warn!(playlist_id = %playlist_id, video_id = %video_id, "yt-dlp reported a failed download");
                Err(anyhow::anyhow!(
                    "yt-dlp failed to download video {video_id}"
                ))
            }
        }
    }

    /// Deletes a removed video's downloaded file from disk, scheduled by
    /// `subscribers::delete_video_file_on_video_deleted` whenever a
    /// downloaded video is removed from its playlist. No-ops (without
    /// erroring) if the playlist no longer exists or no matching file is
    /// found, so the task is safe to retry.
    pub fn delete_video_file(
        &self,
        playlist_id: PlaylistId,
        video_id: VideoId,
        title: &str,
    ) -> anyhow::Result<()> {
        let Some(playlist) = self.playlist_repository.find(&playlist_id)? else {
            debug!(playlist_id = %playlist_id, "playlist no longer exists, skipping file deletion");
            return Ok(());
        };

        let filename = VideoFilename::from_title(title);
        let output_dir = Path::new(&self.videos_path).join(playlist.path.as_str());
        let deleted =
            self.video_file_repository
                .delete(&output_dir, filename.as_str(), video_id.as_str())?;

        if deleted {
            info!(playlist_id = %playlist_id, video_id = %video_id, "deleted video file");
        } else {
            debug!(playlist_id = %playlist_id, video_id = %video_id, "no matching video file found to delete");
        }

        Ok(())
    }
}
