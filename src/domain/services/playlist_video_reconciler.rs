use crate::domain::event::DomainEvent;
use crate::domain::playlist::Playlist;
use crate::domain::playlist::PlaylistId;
use crate::domain::playlist_video::PlaylistVideo;
use crate::domain::playlist_video::{VideoAddedToPlaylist, VideoRemovedFromPlaylist};
use crate::domain::services::{
    DesiredState, InternalVideoReconciler, InternalVideoReconcilerApi, MembershipDelta,
    ShowFileWriter, ShowFileWriterApi,
};
use crate::domain::show::ShowKey;
use crate::domain::task::Task;
use crate::domain::video::{Video, VideoId, VideoRecordId, VideoStatus};
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::sqlite_video_transcript_repository::VideoTranscriptRepository;
use crate::infrastructure::repositories::youtube_playlist_items_repository::{
    YoutubePlaylistItem, YoutubePlaylistItemsRepository,
};
use crate::infrastructure::shared::domain_events::event_publisher::EventPublisher;
use crate::infrastructure::shared::error_report;
use crate::infrastructure::shared::show_locks::{ShowLockGuard, ShowLocks};
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, info_span, warn};

/// Reconciles a playlist with YouTube: brings its stored membership in line
/// with YouTube's playlist items, then hands its folder to
/// `InternalVideoReconciler` to bring it in line with the database.
#[derive(Clone)]
pub struct PlaylistVideoReconciler {
    playlist_repository: Arc<dyn PlaylistRepository>,
    video_repository: Arc<dyn VideoRepository>,
    video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    youtube_playlist_items_repository: Arc<dyn YoutubePlaylistItemsRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    task_repository: Arc<dyn TaskRepository>,
    internal_video_reconciler: Arc<InternalVideoReconciler>,
    show_file_writer: Arc<ShowFileWriter>,
    clock: Arc<dyn Clock>,
    show_locks: Arc<ShowLocks>,
}

impl PlaylistVideoReconciler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_repository: Arc<dyn VideoRepository>,
        video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        youtube_playlist_items_repository: Arc<dyn YoutubePlaylistItemsRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        task_repository: Arc<dyn TaskRepository>,
        internal_video_reconciler: Arc<InternalVideoReconciler>,
        show_file_writer: Arc<ShowFileWriter>,
        clock: Arc<dyn Clock>,
        show_locks: Arc<ShowLocks>,
    ) -> Self {
        Self {
            playlist_repository,
            video_repository,
            video_transcript_repository,
            playlist_video_repository,
            youtube_playlist_items_repository,
            event_publisher,
            task_repository,
            internal_video_reconciler,
            show_file_writer,
            clock,
            show_locks,
        }
    }
}

pub trait PlaylistVideoReconcilerApi: Send + Sync {
    /// Runs one reconcile pass for a playlist and reschedules the next
    /// recurring pass — no-ops entirely if the playlist no longer exists.
    /// Called by the recurring `ReconcilePlaylistTask`. A failure to list
    /// the playlist's items is not fatal: membership is left untouched for
    /// this pass.
    fn reconcile(&self, id: PlaylistId) -> anyhow::Result<()>;

    /// Runs the first reconcile pass of a newly created playlist and
    /// schedules its recurring passes — no-ops entirely if the playlist no
    /// longer exists. Called by the one-shot `PlaylistCreated` reaction.
    /// Unlike `reconcile`, a failure to list the playlist's items fails the
    /// pass and schedules nothing, so the caller retries it instead of
    /// leaving the new playlist empty until a recurring pass.
    fn initial_reconcile(&self, id: PlaylistId) -> anyhow::Result<()>;

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
        let _guard = self.lock(&id);
        let Some(playlist) = self.find_playlist(&id)? else {
            return Ok(());
        };

        self.reconcile_playlist(&playlist, ListingFailure::SkipMembership)?;
        self.schedule_next_reconcile(&id)
    }

    fn initial_reconcile(&self, id: PlaylistId) -> anyhow::Result<()> {
        let _guard = self.lock(&id);
        let Some(playlist) = self.find_playlist(&id)? else {
            return Ok(());
        };

        self.reconcile_playlist(&playlist, ListingFailure::FailPass)?;
        self.schedule_next_reconcile(&id)
    }

    fn force_reconcile(&self, id: PlaylistId) -> anyhow::Result<()> {
        let _guard = self.lock(&id);
        let Some(playlist) = self.find_playlist(&id)? else {
            return Ok(());
        };

        self.reconcile_playlist(&playlist, ListingFailure::SkipMembership)
    }
}

impl PlaylistVideoReconciler {
    /// Waits for a move of the playlist to finish, so the pass reads the
    /// playlist, and works in its folder, only once it has moved.
    fn lock(&self, id: &PlaylistId) -> ShowLockGuard<'_> {
        self.show_locks.lock(ShowKey::Playlist(id.clone()))
    }

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
    fn reconcile_playlist(
        &self,
        playlist: &Playlist,
        on_listing_failure: ListingFailure,
    ) -> anyhow::Result<()> {
        let _span = info_span!("reconcile_playlist", playlist_id = %playlist.id).entered();
        info!(playlist_id = %playlist.id, kind = %playlist.kind, "reconciling playlist");

        // Best effort: a show file that can't be written must not stop the
        // source from syncing; the next pass tries again.
        if let Err(e) = self.show_file_writer.write_playlist_show(playlist) {
            warn!(playlist_id = %playlist.id, error = %e, "failed to write show files");
        }
        let delta = self.sync_membership_with_youtube(playlist, on_listing_failure)?;
        let desired = self.desired_state(playlist)?;
        self.internal_video_reconciler.reconcile(&desired, &delta)
    }

    /// Brings the stored membership in line with YouTube's playlist items:
    /// adds newly seen videos, refreshes the title and position of known
    /// ones, and removes stored videos no longer in the playlist. A failed
    /// listing either leaves membership untouched for this pass, the
    /// filesystem steps still running from the stored rows, or fails the
    /// pass, per `on_listing_failure`.
    fn sync_membership_with_youtube(
        &self,
        playlist: &Playlist,
        on_listing_failure: ListingFailure,
    ) -> anyhow::Result<MembershipDelta> {
        let Some(current_videos) = self.list_current_videos(playlist, on_listing_failure)? else {
            return Ok(MembershipDelta::default());
        };
        let stored_videos = self.list_stored_videos(playlist)?;
        let now = self.clock.now();

        // Keyed by YouTube id; a video added below joins it, so a video
        // listed twice in the playlist is stored once.
        let mut known_videos: HashMap<String, (PlaylistVideo, Video)> = stored_videos
            .iter()
            .map(|(pv, video)| {
                (
                    video.youtube_id.as_str().to_string(),
                    (pv.clone(), video.clone()),
                )
            })
            .collect();
        let mut delta = MembershipDelta::default();
        let mut numbering = playlist.clone();
        for current in Self::in_position_order(&current_videos) {
            match known_videos.get(&current.video_id) {
                None => {
                    let youtube_id = VideoId::new(&current.video_id)?;
                    let (numbered, added) = self.add_video(numbering, youtube_id, current, now)?;
                    numbering = numbered;
                    delta.added_ids.push(added.1.id.clone());
                    known_videos.insert(current.video_id.clone(), added);
                }
                Some((existing, video)) => {
                    self.refresh_known_video(existing, video, current, now)?
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

    /// The playlist's current items. When they can't be listed, e.g. the
    /// playlist was deleted or made private on YouTube, or the YouTube Data
    /// API errors, either `None` (logged) or the error, per
    /// `on_listing_failure`.
    fn list_current_videos(
        &self,
        playlist: &Playlist,
        on_listing_failure: ListingFailure,
    ) -> anyhow::Result<Option<Vec<YoutubePlaylistItem>>> {
        match (
            self.youtube_playlist_items_repository
                .list_current_videos(&playlist.id),
            on_listing_failure,
        ) {
            (Ok(items), _) => Ok(Some(items)),
            (Err(e), ListingFailure::FailPass) => Err(e.context("failed to list playlist items")),
            (Err(e), ListingFailure::SkipMembership) => {
                warn!(
                    playlist_id = %playlist.id,
                    error = %error_report::cause_chain(&e),
                    "failed to list playlist items"
                );
                Ok(None)
            }
        }
    }

    /// Every stored video of the playlist with its membership, in playlist
    /// order, read with one query per table.
    fn list_stored_videos(
        &self,
        playlist: &Playlist,
    ) -> anyhow::Result<Vec<(PlaylistVideo, Video)>> {
        let playlist_videos = self
            .playlist_video_repository
            .list_for_playlist(&playlist.id)?;
        let video_ids: Vec<VideoRecordId> = playlist_videos
            .iter()
            .map(|pv| pv.video_id.clone())
            .collect();
        let mut videos: HashMap<VideoRecordId, Video> = self
            .video_repository
            .find_many(&video_ids)?
            .into_iter()
            .map(|video| (video.id.clone(), video))
            .collect();

        Ok(playlist_videos
            .into_iter()
            .filter_map(|pv| videos.remove(&pv.video_id).map(|video| (pv, video)))
            .collect())
    }

    /// Stores a newly seen video as `PENDING` at its playlist position, under
    /// the playlist's next episode number, and announces it, which is what
    /// triggers its download and thumbnail. Returns the playlist with that
    /// number recorded as its last.
    fn add_video(
        &self,
        playlist: Playlist,
        youtube_id: VideoId,
        current: &YoutubePlaylistItem,
        now: DateTime<Utc>,
    ) -> anyhow::Result<(Playlist, (PlaylistVideo, Video))> {
        let number = playlist.next_episode_number()?;
        let video = Video::create(youtube_id, current.title.clone(), now)
            .with_episode_number(number.into(), now);
        self.video_repository.save(&video)?;
        let playlist_video =
            PlaylistVideo::create(playlist.id.clone(), video.id.clone(), current.position, now);
        self.playlist_video_repository.save(&playlist_video)?;
        self.playlist_repository
            .record_last_episode(&playlist.id, number)?;
        let playlist = playlist.with_last_episode(number);
        info!(
            playlist_id = %playlist.id,
            video_id = %video.youtube_id,
            title = %current.title,
            episode = number.episode(),
            "added video to playlist"
        );
        self.event_publisher
            .publish(&DomainEvent::VideoAddedToPlaylist(VideoAddedToPlaylist {
                playlist_id: playlist.id.as_str().to_string(),
                video_id: video.id.as_str().to_string(),
            }))?;

        Ok((playlist, (playlist_video, video)))
    }

    /// Follows a known video's title and playlist position on YouTube.
    fn refresh_known_video(
        &self,
        existing: &PlaylistVideo,
        video: &Video,
        current: &YoutubePlaylistItem,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        if video.title != current.title {
            self.video_repository
                .update_title(&video.id, &current.title, now)?;
        }
        if existing.position != current.position {
            self.playlist_video_repository.save(&PlaylistVideo {
                position: current.position,
                created_at: now,
                ..existing.clone()
            })?;
        }

        Ok(())
    }

    /// Deletes every stored video whose YouTube id is not in
    /// `current_youtube_ids` and announces its removal, which is what
    /// cleans up its files.
    fn remove_videos_not_in(
        &self,
        playlist: &Playlist,
        stored_videos: &[(PlaylistVideo, Video)],
        current_youtube_ids: &HashSet<&str>,
    ) -> anyhow::Result<()> {
        stored_videos
            .iter()
            .map(|(_, video)| video)
            .filter(|video| !current_youtube_ids.contains(video.youtube_id.as_str()))
            .try_for_each(|video| {
                info!(
                    playlist_id = %playlist.id,
                    video_id = %video.youtube_id,
                    "removing video from playlist (no longer on YouTube)"
                );
                self.playlist_video_repository
                    .delete(&playlist.id, &video.youtube_id)?;
                self.video_repository.delete(&video.id)?;
                self.video_transcript_repository.delete(&video.id)?;
                self.event_publisher
                    .publish(&DomainEvent::VideoRemovedFromPlaylist(
                        VideoRemovedFromPlaylist {
                            playlist_id: playlist.id.as_str().to_string(),
                            video_id: video.id.as_str().to_string(),
                            title: video.title.clone(),
                            filename: video.filename.clone(),
                            thumbnail_filename: video.thumbnail_filename.clone(),
                            subtitle_filename: video.subtitle_filename.clone(),
                            was_downloaded: video.status == VideoStatus::Downloaded,
                        },
                    ))
            })
    }

    /// The listed items by playlist position, so new videos are numbered in
    /// playlist order.
    fn in_position_order(items: &[YoutubePlaylistItem]) -> Vec<&YoutubePlaylistItem> {
        let mut ordered: Vec<&YoutubePlaylistItem> = items.iter().collect();
        ordered.sort_by_key(|item| item.position);
        ordered
    }

    /// What the database says the playlist's folder should contain.
    fn desired_state(&self, playlist: &Playlist) -> anyhow::Result<DesiredState> {
        let playlist_videos = self
            .playlist_video_repository
            .list_for_playlist(&playlist.id)?;

        Ok(DesiredState {
            path: playlist.path.clone(),
            quality: playlist.quality,
            video_ids: playlist_videos
                .iter()
                .map(|pv| pv.video_id.clone())
                .collect(),
            sort_positions: playlist_videos
                .into_iter()
                .map(|pv| (pv.video_id, pv.position))
                .collect(),
        })
    }

    /// Re-reads the playlist so an interval changed, or a playlist deleted,
    /// while the pass ran is honoured. A playlist keeps at most one pending
    /// reconcile; the running pass is not pending, so it never blocks itself.
    fn schedule_next_reconcile(&self, id: &PlaylistId) -> anyhow::Result<()> {
        let Some(playlist) = self.playlist_repository.find(id)? else {
            info!(playlist_id = %id, "playlist deleted during reconcile, not scheduling the next one");
            return Ok(());
        };
        let task = Task::ReconcilePlaylist {
            playlist_id: id.as_str().to_string(),
        };
        if self.task_repository.find_pending(&task)?.is_some() {
            info!(playlist_id = %id, "a reconcile of the playlist is already pending, not scheduling another");
            return Ok(());
        }
        let next_run_at = self.clock.now() + playlist.reconcile_interval.duration();
        self.task_repository.schedule(&task, next_run_at)?;
        info!(playlist_id = %id, next_run_at = %next_run_at, "scheduled next reconcile of playlist");

        Ok(())
    }
}

/// What a pass does when listing the playlist's items on YouTube fails.
#[derive(Clone, Copy)]
enum ListingFailure {
    /// Leave membership untouched and run the rest of the pass.
    SkipMembership,
    /// Fail the pass.
    FailPass,
}
