use super::video::Video;
use super::video_filename::VideoFilename;
use super::video_status::VideoStatus;
use crate::domain::event::DomainEvent;
use crate::domain::playlist::Quality;
use crate::domain::shared::{PlaylistId, VideoId};
use crate::domain::task::Task;
use crate::infrastructure::repositories::sqlite_event_repository::EventPublisher;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::system_clock::Clock;
use crate::infrastructure::repositories::video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubePlaylistItemsRepository;
use crate::infrastructure::repositories::youtube_video_downloader_repository::VideoDownloaderRepository;
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

        let output_dir = Path::new(&self.videos_path).join(playlist.name.as_str());
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
                .update(&started.mark_downloaded(self.clock.now()))?;
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
        let output_dir = Path::new(&self.videos_path).join(playlist.name.as_str());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::playlist::{Playlist, PlaylistName, Quality};
    use crate::domain::video::VideoStatus;
    use crate::infrastructure::repositories::sqlite_event_repository::FakeEventPublisher;
    use crate::infrastructure::repositories::sqlite_playlist_repository::{
        FakePlaylistRepository, PlaylistRepository,
    };
    use crate::infrastructure::repositories::sqlite_task_repository::FakeTaskRepository;
    use crate::infrastructure::repositories::sqlite_video_repository::{
        FakeVideoRepository, VideoRepository,
    };
    use crate::infrastructure::repositories::system_clock::FixedClock;
    use crate::infrastructure::repositories::video_file_repository::FakeVideoFileRepository;
    use crate::infrastructure::repositories::youtube_playlist_items_repository::FakeYoutubePlaylistItemsRepository;
    use crate::infrastructure::repositories::youtube_video_downloader_repository::FakeVideoDownloaderRepository;
    use chrono::{DateTime, Utc};

    fn fixed_timestamp() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn playlist_id() -> PlaylistId {
        PlaylistId::new("PL1").unwrap()
    }

    fn video_id() -> VideoId {
        VideoId::new("vid1").unwrap()
    }

    fn service_with(
        seed_playlist: bool,
        seed_video: bool,
        downloader: FakeVideoDownloaderRepository,
    ) -> (VideoService, Arc<FakeVideoRepository>) {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        if seed_playlist {
            playlist_repository
                .insert(&Playlist::create(
                    playlist_id(),
                    PlaylistName::new("My Playlist").unwrap(),
                    Quality::High,
                    fixed_timestamp(),
                ))
                .unwrap();
        }
        let video_repository = Arc::new(FakeVideoRepository::default());
        if seed_video {
            video_repository
                .save(&Video::create(
                    playlist_id(),
                    video_id(),
                    "My Video",
                    fixed_timestamp(),
                ))
                .unwrap();
        }

        let service = VideoService::new(
            playlist_repository,
            video_repository.clone(),
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FakeTaskRepository::default()),
            Arc::new(downloader),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );

        (service, video_repository)
    }

    fn service_for_file_deletion(
        seed_playlist: bool,
        video_file_repository: Arc<FakeVideoFileRepository>,
    ) -> VideoService {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        if seed_playlist {
            playlist_repository
                .insert(&Playlist::create(
                    playlist_id(),
                    PlaylistName::new("My Playlist").unwrap(),
                    Quality::High,
                    fixed_timestamp(),
                ))
                .unwrap();
        }

        VideoService::new(
            playlist_repository,
            Arc::new(FakeVideoRepository::default()),
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FakeTaskRepository::default()),
            Arc::new(FakeVideoDownloaderRepository::new(true)),
            video_file_repository,
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        )
    }

    #[test]
    fn it_should_mark_the_video_downloaded_on_a_successful_download() {
        let (service, video_repository) =
            service_with(true, true, FakeVideoDownloaderRepository::new(true));

        service
            .download_video(playlist_id(), video_id(), Quality::High, false)
            .unwrap();

        let video = video_repository
            .find(&playlist_id(), &video_id())
            .unwrap()
            .unwrap();
        assert_eq!(video.status, VideoStatus::Downloaded);
    }

    #[test]
    fn it_should_mark_the_video_errored_retrying_when_the_download_fails_with_retries_left() {
        let (service, video_repository) =
            service_with(true, true, FakeVideoDownloaderRepository::new(false));

        let result = service.download_video(playlist_id(), video_id(), Quality::High, false);

        assert!(result.is_err());
        let video = video_repository
            .find(&playlist_id(), &video_id())
            .unwrap()
            .unwrap();
        assert_eq!(video.status, VideoStatus::ErroredRetrying);
    }

    #[test]
    fn it_should_mark_the_video_errored_when_the_download_fails_on_the_last_attempt() {
        let (service, video_repository) =
            service_with(true, true, FakeVideoDownloaderRepository::new(false));

        let result = service.download_video(playlist_id(), video_id(), Quality::High, true);

        assert!(result.is_err());
        let video = video_repository
            .find(&playlist_id(), &video_id())
            .unwrap()
            .unwrap();
        assert_eq!(video.status, VideoStatus::Errored);
    }

    #[test]
    fn it_should_no_op_when_the_playlist_no_longer_exists() {
        let (service, video_repository) =
            service_with(false, true, FakeVideoDownloaderRepository::new(true));

        service
            .download_video(playlist_id(), video_id(), Quality::High, false)
            .unwrap();

        let video = video_repository
            .find(&playlist_id(), &video_id())
            .unwrap()
            .unwrap();
        assert_eq!(video.status, VideoStatus::Pending);
    }

    #[test]
    fn it_should_no_op_when_the_video_no_longer_exists() {
        let (service, _video_repository) =
            service_with(true, false, FakeVideoDownloaderRepository::new(true));

        let result = service.download_video(playlist_id(), video_id(), Quality::High, false);

        assert!(result.is_ok());
    }

    #[test]
    fn it_should_pass_the_sanitized_title_as_the_desired_filename_to_the_downloader() {
        let playlist_repository = Arc::new(FakePlaylistRepository::default());
        playlist_repository
            .insert(&Playlist::create(
                playlist_id(),
                PlaylistName::new("My Playlist").unwrap(),
                Quality::High,
                fixed_timestamp(),
            ))
            .unwrap();
        let video_repository = Arc::new(FakeVideoRepository::default());
        let messy_title = "My: Messy / Title?";
        video_repository
            .save(&Video::create(
                playlist_id(),
                video_id(),
                messy_title,
                fixed_timestamp(),
            ))
            .unwrap();
        let downloader = Arc::new(FakeVideoDownloaderRepository::new(true));

        let service = VideoService::new(
            playlist_repository,
            video_repository,
            Arc::new(FakeYoutubePlaylistItemsRepository::default()),
            Arc::new(FakeEventPublisher::default()),
            Arc::new(FakeTaskRepository::default()),
            downloader.clone(),
            Arc::new(FakeVideoFileRepository::default()),
            Arc::new(FixedClock(fixed_timestamp())),
            3600,
            "/videos",
        );

        service
            .download_video(playlist_id(), video_id(), Quality::High, false)
            .unwrap();

        let calls = downloader.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        let (_, desired_filename, id, _, _) = &calls[0];
        assert_eq!(
            desired_filename,
            VideoFilename::from_title(messy_title).as_str()
        );
        assert_ne!(desired_filename, messy_title);
        assert_eq!(id, video_id().as_str());
    }

    #[test]
    fn it_should_delete_the_video_file_when_present() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(true)));
        let service = service_for_file_deletion(true, video_file_repository.clone());

        service
            .delete_video_file(playlist_id(), video_id(), "My Video")
            .unwrap();

        let calls = video_file_repository.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        let (output_dir, filename_stem, id) = &calls[0];
        assert_eq!(output_dir, Path::new("/videos/My Playlist"));
        assert_eq!(
            filename_stem,
            VideoFilename::from_title("My Video").as_str()
        );
        assert_eq!(id, video_id().as_str());
    }

    #[test]
    fn it_should_no_op_when_the_playlist_no_longer_exists_for_file_deletion() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(false)));
        let service = service_for_file_deletion(false, video_file_repository.clone());

        service
            .delete_video_file(playlist_id(), video_id(), "My Video")
            .unwrap();

        assert!(video_file_repository.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn it_should_no_op_when_no_matching_file_is_found() {
        let video_file_repository = Arc::new(FakeVideoFileRepository::new(Ok(false)));
        let service = service_for_file_deletion(true, video_file_repository.clone());

        let result = service.delete_video_file(playlist_id(), video_id(), "My Video");

        assert!(result.is_ok());
    }
}
