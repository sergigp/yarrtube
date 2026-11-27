use crate::domain::event::DomainEvent;
use crate::domain::playlist::Playlist;
use crate::domain::playlist::PlaylistId;
use crate::domain::playlist_video::PlaylistVideo;
use crate::domain::playlist_video::{VideoAddedToPlaylist, VideoRemovedFromPlaylist};
use crate::domain::services::{
    DesiredState, FolderReconciliation, InternalVideoReconciler, InternalVideoReconcilerApi,
    ShowFileWriter, ShowFileWriterApi, VideoUpdater, VideoUpdaterApi,
};
use crate::domain::shared::{MembershipChanges, elapsed_seconds};
use crate::domain::show::ShowKey;
use crate::domain::task::Task;
use crate::domain::video::{
    DiscardedVideo, RestoreDiscardedVideosError, Video, VideoId, YoutubeVideoId,
};
use crate::infrastructure::repositories::sqlite_discarded_video_repository::DiscardedVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_task_repository::TaskRepository;
use crate::infrastructure::repositories::sqlite_video_categories_repository::VideoCategoriesRepository;
use crate::infrastructure::repositories::sqlite_video_category_verdict_repository::VideoCategoryVerdictRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::repositories::sqlite_video_summary_repository::VideoSummaryRepository;
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
    video_updater: VideoUpdater,
    video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
    video_summary_repository: Arc<dyn VideoSummaryRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    youtube_playlist_items_repository: Arc<dyn YoutubePlaylistItemsRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    task_repository: Arc<dyn TaskRepository>,
    internal_video_reconciler: Arc<InternalVideoReconciler>,
    show_file_writer: Arc<ShowFileWriter>,
    clock: Arc<dyn Clock>,
    show_locks: Arc<ShowLocks>,
    video_categories_repository: Arc<dyn VideoCategoriesRepository>,
    video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
    discarded_video_repository: Arc<dyn DiscardedVideoRepository>,
}

impl PlaylistVideoReconciler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_repository: Arc<dyn VideoRepository>,
        video_transcript_repository: Arc<dyn VideoTranscriptRepository>,
        video_summary_repository: Arc<dyn VideoSummaryRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        youtube_playlist_items_repository: Arc<dyn YoutubePlaylistItemsRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        task_repository: Arc<dyn TaskRepository>,
        internal_video_reconciler: Arc<InternalVideoReconciler>,
        show_file_writer: Arc<ShowFileWriter>,
        clock: Arc<dyn Clock>,
        show_locks: Arc<ShowLocks>,
        video_categories_repository: Arc<dyn VideoCategoriesRepository>,
        video_category_verdict_repository: Arc<dyn VideoCategoryVerdictRepository>,
        discarded_video_repository: Arc<dyn DiscardedVideoRepository>,
    ) -> Self {
        Self {
            playlist_repository,
            video_updater: VideoUpdater::new(video_repository.clone()),
            video_repository,
            video_transcript_repository,
            video_summary_repository,
            playlist_video_repository,
            youtube_playlist_items_repository,
            event_publisher,
            task_repository,
            internal_video_reconciler,
            show_file_writer,
            clock,
            show_locks,
            video_categories_repository,
            video_category_verdict_repository,
            discarded_video_repository,
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

    /// As `ChannelVideoReconcilerApi::restore_discarded`, for a playlist.
    fn restore_discarded(&self, id: PlaylistId) -> Result<(), RestoreDiscardedVideosError>;
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

    fn restore_discarded(&self, id: PlaylistId) -> Result<(), RestoreDiscardedVideosError> {
        let _guard = self.lock(&id);
        let Some(playlist) = self.find_playlist(&id)? else {
            return Err(RestoreDiscardedVideosError::PlaylistNotFound(id));
        };

        self.discarded_video_repository
            .delete_all(&ShowKey::Playlist(id))?;
        self.restore_stored_discarded(&self.list_stored_videos(&playlist)?)?;
        Ok(self.reconcile_playlist(&playlist, ListingFailure::SkipMembership)?)
    }
}

impl PlaylistVideoReconciler {
    /// Puts every stored Discarded video back to Pending.
    fn restore_stored_discarded(&self, stored: &[(PlaylistVideo, Video)]) -> anyhow::Result<()> {
        let now = self.clock.now();
        stored
            .iter()
            .map(|(_, video)| video)
            .filter(|video| video.download.is_discarded())
            .try_for_each(|video| {
                self.video_updater
                    .update(&video.id, |video| video.restore(now))
                    .map(|_| ())
            })
    }

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
        let started = self.clock.now();
        info!(playlist_id = %playlist.id, kind = %playlist.kind, "reconciling playlist");

        // Best effort: a show file that can't be written must not stop the
        // source from syncing; the next pass tries again.
        if let Err(e) = self.show_file_writer.write_playlist_show(playlist) {
            warn!(playlist_id = %playlist.id, error = %e, "failed to write show files");
        }
        let membership = self.sync_membership_with_youtube(playlist, on_listing_failure)?;
        let desired = self.desired_state(playlist)?;
        let folder = self.internal_video_reconciler.reconcile(&desired)?;
        self.record_sync(playlist, &membership)?;
        Self::log_pass(
            playlist,
            &membership,
            &folder,
            elapsed_seconds(started, self.clock.now()),
        );
        Ok(())
    }

    /// Records the pass as the playlist's last sync, unless its listing
    /// failed: a pass that couldn't reach YouTube didn't sync the playlist.
    fn record_sync(
        &self,
        playlist: &Playlist,
        membership: &MembershipChanges,
    ) -> anyhow::Result<()> {
        if membership.listed.is_none() {
            return Ok(());
        }
        self.playlist_repository
            .record_last_synced_at(&playlist.clone().with_last_synced_at(self.clock.now()))
    }

    /// The pass's summary line.
    fn log_pass(
        playlist: &Playlist,
        membership: &MembershipChanges,
        folder: &FolderReconciliation,
        duration_seconds: f64,
    ) {
        info!(
            playlist_id = %playlist.id,
            kind = %playlist.kind,
            listed = membership.listed,
            added = membership.added,
            removed = membership.removed,
            syncs_requested = folder.syncs_requested,
            orphans_deleted = folder.orphans_deleted,
            listing_seconds = membership.listing_seconds,
            duration_seconds,
            "reconciled playlist"
        );
    }

    /// Brings the stored membership in line with YouTube's playlist items:
    /// adds newly seen videos (never a Discarded one removed earlier, which
    /// the playlist remembers), refreshes the title and position of known
    /// ones, and removes stored videos no longer in the playlist. A failed
    /// listing either leaves membership untouched for this pass, the
    /// filesystem steps still running from the stored rows, or fails the
    /// pass, per `on_listing_failure`.
    fn sync_membership_with_youtube(
        &self,
        playlist: &Playlist,
        on_listing_failure: ListingFailure,
    ) -> anyhow::Result<MembershipChanges> {
        let listing_started = self.clock.now();
        let listing = self.list_current_videos(playlist, on_listing_failure)?;
        let listing_seconds = elapsed_seconds(listing_started, self.clock.now());
        let Some(current_videos) = listing else {
            return Ok(MembershipChanges {
                listing_seconds,
                ..MembershipChanges::default()
            });
        };
        let stored_videos = self.list_stored_videos(playlist)?;
        let remembered = self
            .discarded_video_repository
            .list(&ShowKey::Playlist(playlist.id.clone()))?
            .into_iter()
            .map(|discarded| discarded.youtube_id)
            .collect::<HashSet<_>>();
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
        let mut numbering = playlist.clone();
        let mut added = 0;
        for current in Self::in_position_order(&current_videos) {
            match known_videos.get(&current.youtube_id) {
                None => {
                    let youtube_id = YoutubeVideoId::new(&current.youtube_id)?;
                    if remembered.contains(&youtube_id) {
                        continue;
                    }
                    let (numbered, video) = self.add_video(numbering, youtube_id, current, now)?;
                    numbering = numbered;
                    known_videos.insert(current.youtube_id.clone(), video);
                    added += 1;
                }
                Some((existing, video)) => {
                    self.refresh_known_video(existing, video, current, now)?
                }
            }
        }

        let current_youtube_ids: HashSet<&str> = current_videos
            .iter()
            .map(|current| current.youtube_id.as_str())
            .collect();
        let last_listed_position = current_videos
            .iter()
            .map(|current| current.position)
            .max()
            .unwrap_or(-1);
        let removed = self.keep_or_remove_videos_not_in(
            playlist,
            &stored_videos,
            &current_youtube_ids,
            last_listed_position,
        )?;

        Ok(MembershipChanges {
            listed: Some(current_videos.len()),
            added,
            removed,
            listing_seconds,
        })
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
        let video_ids: Vec<VideoId> = playlist_videos
            .iter()
            .map(|pv| pv.video_id.clone())
            .collect();
        let mut videos: HashMap<VideoId, Video> = self
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
    /// the playlist's next episode number, a favorite when another copy of it
    /// is, and announces it, which is what
    /// schedules its video sync. Returns the playlist with that
    /// number recorded as its last.
    fn add_video(
        &self,
        playlist: Playlist,
        youtube_id: YoutubeVideoId,
        current: &YoutubePlaylistItem,
        now: DateTime<Utc>,
    ) -> anyhow::Result<(Playlist, (PlaylistVideo, Video))> {
        let number = playlist.next_episode_number()?;
        let inherited = self.inherited_favorite(&youtube_id)?;
        let video = Video::create(youtube_id, current.title.clone(), now)
            .with_episode_number(number.into(), now)
            .with_favorited_at(inherited);
        self.video_repository.insert(&video)?;
        let playlist_video =
            PlaylistVideo::create(playlist.id.clone(), video.id.clone(), current.position, now);
        self.playlist_video_repository.save(&playlist_video)?;
        self.playlist_repository
            .record_last_episode(&playlist.id, number)?;
        let playlist = playlist.with_last_episode(number);
        info!(
            playlist_id = %playlist.id,
            youtube_id = %video.youtube_id,
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

    /// Earliest favorite time among the stored copies of `youtube_id`: a
    /// favorite belongs to the YouTube video.
    fn inherited_favorite(
        &self,
        youtube_id: &YoutubeVideoId,
    ) -> anyhow::Result<Option<DateTime<Utc>>> {
        Ok(self
            .video_repository
            .find_by_youtube_id(youtube_id)?
            .into_iter()
            .filter_map(|copy| copy.favorited_at)
            .min())
    }

    /// Follows a known video's title and playlist position on YouTube; a
    /// kept video back in the playlist is no longer kept.
    fn refresh_known_video(
        &self,
        existing: &PlaylistVideo,
        video: &Video,
        current: &YoutubePlaylistItem,
        now: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        if video.title != current.title {
            self.video_updater.update(&video.id, |video| {
                Some(video.with_title(current.title.clone(), now))
            })?;
        }
        if existing.position != current.position || existing.kept {
            self.playlist_video_repository
                .save(&existing.clone().listed_at(current.position, now))?;
        }

        Ok(())
    }

    /// Every stored video whose YouTube id is not in `current_youtube_ids`:
    /// a favorite is kept, placed after `last_listed_position` in its
    /// previous order, keeping its episode number; any other is removed (see
    /// `remove_video`). Returns how many were removed.
    fn keep_or_remove_videos_not_in(
        &self,
        playlist: &Playlist,
        stored_videos: &[(PlaylistVideo, Video)],
        current_youtube_ids: &HashSet<&str>,
        last_listed_position: i64,
    ) -> anyhow::Result<usize> {
        let (favorites, others): (Vec<_>, Vec<_>) = stored_videos
            .iter()
            .filter(|(_, video)| !current_youtube_ids.contains(video.youtube_id.as_str()))
            .map(|(pv, video)| {
                Ok((
                    pv,
                    video,
                    // An Unavailable favorite isn't kept: no view shows it to unfavorite.
                    !video.is_skipped() && self.is_favorite_now(video)?,
                ))
            })
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .partition(|(_, _, favorite)| *favorite);
        let now = self.clock.now();
        (last_listed_position + 1..)
            .zip(favorites)
            .try_for_each(|(position, (pv, _, _))| {
                self.playlist_video_repository
                    .save(&pv.clone().keep(position, now))
            })?;
        others.into_iter().try_fold(0, |removed, (_, video, _)| {
            self.remove_video(playlist, video)?;
            Ok(removed + 1)
        })
    }

    /// Deletes a stored video no longer in the YouTube playlist and announces
    /// its removal, which is what cleans up its files. A Discarded one is
    /// remembered first.
    fn remove_video(&self, playlist: &Playlist, video: &Video) -> anyhow::Result<()> {
        info!(
            playlist_id = %playlist.id,
            youtube_id = %video.youtube_id,
            "removing video from playlist (no longer on YouTube)"
        );
        self.remember_if_discarded(playlist, video)?;
        self.playlist_video_repository.delete(&video.id)?;
        self.video_repository.delete(&video.id)?;
        self.video_transcript_repository.delete(&video.id)?;
        self.video_summary_repository.delete(&video.id)?;
        self.video_categories_repository.delete(&video.id)?;
        self.video_category_verdict_repository
            .delete_for_video(&video.id)?;
        self.event_publisher
            .publish(&DomainEvent::VideoRemovedFromPlaylist(
                VideoRemovedFromPlaylist {
                    playlist_id: playlist.id.as_str().to_string(),
                    video_id: video.id.as_str().to_string(),
                    title: video.title.clone(),
                    filename: video.download.filename().map(str::to_string),
                    thumbnail_filename: video.thumbnail.filename.clone(),
                    subtitle_filename: video.subtitles.filename.clone(),
                    was_downloaded: video.download.is_downloaded(),
                },
            ))
    }

    /// Whether `video` is a favorite, re-read: it may have been favorited
    /// since the pass read it.
    fn is_favorite_now(&self, video: &Video) -> anyhow::Result<bool> {
        Ok(video.is_favorite()
            || self
                .video_repository
                .find(&video.id)?
                .is_some_and(|stored| stored.is_favorite()))
    }

    /// Remembers a Discarded video being removed, so the playlist never adds
    /// it back.
    fn remember_if_discarded(&self, playlist: &Playlist, video: &Video) -> anyhow::Result<()> {
        // Re-read: the video may have been discarded since the pass read it.
        let discarded = video.download.is_discarded()
            || self
                .video_repository
                .find(&video.id)?
                .is_some_and(|stored| stored.download.is_discarded());
        if !discarded {
            return Ok(());
        }
        self.discarded_video_repository.record(&DiscardedVideo::new(
            ShowKey::Playlist(playlist.id.clone()),
            video.youtube_id.clone(),
            self.clock.now(),
        ))
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
        Ok(DesiredState {
            path: playlist.path.clone(),
            video_ids: self
                .playlist_video_repository
                .list_for_playlist(&playlist.id)?
                .into_iter()
                .map(|pv| pv.video_id)
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
