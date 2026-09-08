use super::video::Video;
use crate::domain::event::DomainEvent;
use crate::domain::shared::{PlaylistId, VideoId};
use crate::domain::task::Task;
use crate::infrastructure::repositories::sqlite_event_repository::EventPublisher;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::system_clock::Clock;
use crate::infrastructure::repositories::youtube_playlist_items_repository::YoutubePlaylistItemsRepository;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{debug, info};

/// Orchestrates every operation on the video aggregate. Injected with only the
/// ports video operations actually use — not every port the application has.
#[derive(Clone)]
pub struct VideoService {
    playlist_repository: Arc<dyn PlaylistRepository>,
    video_repository: Arc<dyn VideoRepository>,
    youtube_playlist_items_repository: Arc<dyn YoutubePlaylistItemsRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    task_repository: Arc<dyn TaskRepository>,
    clock: Arc<dyn Clock>,
    sync_interval_seconds: i64,
}

impl VideoService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_repository: Arc<dyn VideoRepository>,
        youtube_playlist_items_repository: Arc<dyn YoutubePlaylistItemsRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        task_repository: Arc<dyn TaskRepository>,
        clock: Arc<dyn Clock>,
        sync_interval_seconds: i64,
    ) -> Self {
        Self {
            playlist_repository,
            video_repository,
            youtube_playlist_items_repository,
            event_publisher,
            task_repository,
            clock,
            sync_interval_seconds,
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
        let stored_ids: HashSet<&str> = stored_videos.iter().map(|v| v.video_id.as_str()).collect();

        let now = self.clock.now();
        let mut current_ids = Vec::with_capacity(current_videos.len());
        for video in &current_videos {
            let video_id = VideoId::new(&video.video_id)?;
            let is_new = !stored_ids.contains(video_id.as_str());

            self.video_repository.upsert(&Video::create(
                id.clone(),
                video_id.clone(),
                video.title.clone(),
                now,
            ))?;

            if is_new {
                info!(
                    playlist_id = %id,
                    video_id = %video_id,
                    title = %video.title,
                    "added video to playlist"
                );
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
            }
        }
        self.video_repository.delete_not_in(&id, &current_ids)?;

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
}
