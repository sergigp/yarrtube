use crate::domain::event::DomainEvent;
use crate::domain::playlist::Playlist;
use crate::domain::playlist::PlaylistId;
use crate::domain::playlist_video::PlaylistVideo;
use crate::domain::playlist_video::{VideoAddedToPlaylist, VideoRemovedFromPlaylist};
use crate::domain::services::{
    DesiredState, InternalVideoReconciler, InternalVideoReconcilerApi, MembershipDelta,
};
use crate::domain::task::Task;
use crate::domain::video::{Video, VideoId, VideoRecordId, VideoStatus};
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::youtube_playlist_items_repository::{
    YoutubePlaylistItem, YoutubePlaylistItemsRepository,
};
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::error_report;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{debug, info, info_span, warn};

/// Reconciles a playlist with YouTube: brings its stored membership in line
/// with YouTube's playlist items, then hands its folder to
/// `InternalVideoReconciler` to bring it in line with the database.
#[derive(Clone)]
pub struct PlaylistVideoReconciler {
    playlist_repository: Arc<dyn PlaylistRepository>,
    video_repository: Arc<dyn VideoRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    youtube_playlist_items_repository: Arc<dyn YoutubePlaylistItemsRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    task_repository: Arc<dyn TaskRepository>,
    internal_video_reconciler: Arc<InternalVideoReconciler>,
    clock: Arc<dyn Clock>,
    reconcile_interval_seconds: i64,
}

impl PlaylistVideoReconciler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_repository: Arc<dyn VideoRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        youtube_playlist_items_repository: Arc<dyn YoutubePlaylistItemsRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        task_repository: Arc<dyn TaskRepository>,
        internal_video_reconciler: Arc<InternalVideoReconciler>,
        clock: Arc<dyn Clock>,
        reconcile_interval_seconds: i64,
    ) -> Self {
        Self {
            playlist_repository,
            video_repository,
            playlist_video_repository,
            youtube_playlist_items_repository,
            event_publisher,
            task_repository,
            internal_video_reconciler,
            clock,
            reconcile_interval_seconds,
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

    /// One reconcile pass: brings the database in line with YouTube (the
    /// playlist's membership), then the playlist's folder in line with the
    /// database. Everything the pass logs, `InternalVideoReconciler`'s steps
    /// included, carries the playlist id through the pass's span.
    fn reconcile_playlist(&self, playlist: &Playlist) -> anyhow::Result<()> {
        let _span = info_span!("reconcile_playlist", playlist_id = %playlist.id).entered();
        info!(playlist_id = %playlist.id, kind = %playlist.kind, "reconciling playlist");

        let delta = self.sync_membership_with_youtube(playlist)?;
        let desired = self.desired_state(playlist)?;
        self.internal_video_reconciler.reconcile(&desired, &delta)
    }

    /// Brings the stored membership in line with YouTube's playlist items:
    /// adds newly seen videos, refreshes the title and position of known
    /// ones, and removes stored videos no longer in the playlist. A failed
    /// listing is not fatal: membership is left untouched for this pass and
    /// the filesystem steps still run from the stored rows.
    fn sync_membership_with_youtube(&self, playlist: &Playlist) -> anyhow::Result<MembershipDelta> {
        let Some(current_videos) = self.list_current_videos(playlist) else {
            return Ok(MembershipDelta::default());
        };
        let stored_videos = self
            .playlist_video_repository
            .list_for_playlist(&playlist.id)?;
        let now = self.clock.now();

        let mut delta = MembershipDelta::default();
        for current in &current_videos {
            let youtube_id = VideoId::new(&current.video_id)?;
            match self
                .playlist_video_repository
                .find_by_youtube_video(&playlist.id, &youtube_id)?
            {
                None => {
                    let added_id = self.add_video(playlist, youtube_id, current, now)?;
                    delta.added_ids.push(added_id);
                }
                Some(existing) => {
                    if let Some((video_id, previous_title)) =
                        self.refresh_known_video(existing, current, now)?
                    {
                        delta.previous_titles.insert(video_id, previous_title);
                    }
                }
            }
        }

        let current_youtube_ids: HashSet<&str> = current_videos
            .iter()
            .map(|current| current.video_id.as_str())
            .collect();
        self.remove_videos_not_in(playlist, &stored_videos, &current_youtube_ids)?;

        Ok(delta)
    }

    /// The playlist's current items, or `None` (logged) when they can't be
    /// listed, e.g. the playlist was deleted or made private on YouTube, or
    /// the YouTube Data API errors.
    fn list_current_videos(&self, playlist: &Playlist) -> Option<Vec<YoutubePlaylistItem>> {
        self.youtube_playlist_items_repository
            .list_current_videos(&playlist.id)
            .inspect_err(|e| {
                warn!(
                    playlist_id = %playlist.id,
                    error = %error_report::cause_chain(e),
                    "failed to list playlist items"
                );
            })
            .ok()
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
        let playlist_video =
            PlaylistVideo::create(playlist.id.clone(), video.id.clone(), current.position, now);
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
        if existing.position != current.position {
            self.playlist_video_repository.save(&PlaylistVideo {
                position: current.position,
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

    /// What the database says the playlist's folder should contain.
    fn desired_state(&self, playlist: &Playlist) -> anyhow::Result<DesiredState> {
        let playlist_videos = self
            .playlist_video_repository
            .list_for_playlist(&playlist.id)?;

        Ok(DesiredState {
            path: playlist.path.clone(),
            quality: playlist.quality,
            videos: self.find_videos(&playlist_videos)?,
            sort_positions: playlist_videos
                .iter()
                .map(|pv| (pv.video_id.clone(), pv.position))
                .collect(),
        })
    }

    fn find_videos(&self, playlist_videos: &[PlaylistVideo]) -> anyhow::Result<Vec<Video>> {
        playlist_videos
            .iter()
            .filter_map(|pv| self.video_repository.find(&pv.video_id).transpose())
            .collect()
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
