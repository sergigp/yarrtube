use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::domain::video::{
    HomeLimits, HomeVideos, ListVideosError, RecentVideo, Video, VideoId, VideoSource, VideoStatus,
    VideoView,
};
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::sync::Arc;

/// Reads videos, listed by playlist or by channel.
#[derive(Clone)]
pub struct VideoSearcher {
    playlist_repository: Arc<dyn PlaylistRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    clock: Arc<dyn Clock>,
}

impl VideoSearcher {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            playlist_repository,
            playlist_video_repository,
            channel_repository,
            channel_video_repository,
            video_repository,
            video_metadata_repository,
            clock,
        }
    }
}

pub trait VideoSearcherApi: Send + Sync {
    /// Lists every video recorded for a playlist, confirming the playlist
    /// exists first, ordered by playlist position, each with its generated
    /// metadata when it has any.
    fn list(&self, playlist_id: &PlaylistId) -> Result<Vec<VideoView>, ListVideosError>;

    /// Lists every video recorded for a channel, confirming the channel
    /// exists first, ordered by recency (most recent first), each with its
    /// generated metadata when it has any.
    fn list_for_channel(
        &self,
        channel_id: &ChannelHandle,
    ) -> Result<Vec<VideoView>, ListVideosError>;

    /// Lists downloaded videos across every tracked playlist and channel,
    /// newest sync first, truncated to `limit`. A video tracked by more than
    /// one source appears once per source.
    fn list_recent(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError>;

    /// Lists videos in progress across every tracked playlist and channel
    /// (see `Video::is_in_progress`), last played first, truncated to
    /// `limit`. A video tracked by more than one source appears once.
    fn list_continue_watching(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError>;

    /// Lists quick watches across every tracked playlist and channel (see
    /// `Video::is_quick_watch`), newest sync first, truncated to `limit`. A
    /// video tracked by more than one source appears once.
    fn list_quick_watches(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError>;

    /// Lists the three home sections from one collection across sources:
    /// continue watching, then quick watches without the videos shown above,
    /// then latest videos (once per source, newest sync first) without the
    /// videos shown in either. Only shown videos are left out further down.
    fn list_home(&self, limits: HomeLimits) -> Result<HomeVideos, ListVideosError>;
}

impl VideoSearcherApi for VideoSearcher {
    fn list(&self, playlist_id: &PlaylistId) -> Result<Vec<VideoView>, ListVideosError> {
        self.playlist_repository
            .find(playlist_id)
            .map_err(ListVideosError::Repository)?
            .ok_or_else(|| ListVideosError::PlaylistNotFound(playlist_id.clone()))?;

        let playlist_videos = self
            .playlist_video_repository
            .list_for_playlist(playlist_id)
            .map_err(ListVideosError::Repository)?;
        playlist_videos
            .iter()
            .filter_map(|pv| self.video_repository.find(&pv.video_id).transpose())
            .map(|video| video.and_then(|video| self.view(video)))
            .collect::<anyhow::Result<Vec<VideoView>>>()
            .map_err(ListVideosError::Repository)
    }

    fn list_for_channel(
        &self,
        channel_id: &ChannelHandle,
    ) -> Result<Vec<VideoView>, ListVideosError> {
        self.channel_repository
            .find(channel_id)
            .map_err(ListVideosError::Repository)?
            .ok_or_else(|| ListVideosError::ChannelNotFound(channel_id.clone()))?;

        let channel_videos = self
            .channel_video_repository
            .list_for_channel(channel_id)
            .map_err(ListVideosError::Repository)?;
        channel_videos
            .iter()
            .filter_map(|cv| self.video_repository.find(&cv.video_id).transpose())
            .map(|video| video.and_then(|video| self.view(video)))
            .collect::<anyhow::Result<Vec<VideoView>>>()
            .map_err(ListVideosError::Repository)
    }

    fn list_recent(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError> {
        let mut recent = self.recent_from_playlists()?;
        recent.extend(self.recent_from_channels()?);

        recent.sort_by_key(|r| std::cmp::Reverse(r.video.created_at));
        recent.truncate(limit);
        Ok(recent)
    }

    fn list_continue_watching(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError> {
        Ok(Self::continue_watching_in(
            &self.downloaded_across_sources()?,
            self.clock.now(),
            limit,
        ))
    }

    fn list_quick_watches(&self, limit: usize) -> Result<Vec<RecentVideo>, ListVideosError> {
        Ok(Self::quick_watches_in(
            &self.downloaded_across_sources()?,
            limit,
        ))
    }

    fn list_home(&self, limits: HomeLimits) -> Result<HomeVideos, ListVideosError> {
        let videos = self.downloaded_across_sources()?;
        let continue_watching =
            Self::continue_watching_in(&videos, self.clock.now(), limits.continue_watching);
        let not_continue_watching = Self::not_shown_in(&videos, &continue_watching);
        let quick_watches = Self::quick_watches_in(&not_continue_watching, limits.quick_watches);
        let latest = Self::latest_in(
            Self::not_shown_in(&not_continue_watching, &quick_watches),
            limits.latest,
        );
        Ok(HomeVideos {
            continue_watching,
            quick_watches,
            latest,
        })
    }
}

impl VideoSearcher {
    fn view(&self, video: Video) -> anyhow::Result<VideoView> {
        let metadata = self.video_metadata_repository.find(&video.id)?;
        Ok(VideoView { video, metadata })
    }

    /// Every downloaded video of every tracked channel, then of every tracked
    /// playlist, once per source.
    fn downloaded_across_sources(&self) -> Result<Vec<RecentVideo>, ListVideosError> {
        let mut videos = self.recent_from_channels()?;
        videos.extend(self.recent_from_playlists()?);
        Ok(videos)
    }

    /// Downloaded videos across sources that `keep` accepts, highest `newest`
    /// first, once per YouTube video, truncated to `limit`.
    /// In progress videos of `videos` (see `Video::is_in_progress`), last
    /// played first, once per YouTube video, up to `limit`.
    fn continue_watching_in(
        videos: &[RecentVideo],
        now: DateTime<Utc>,
        limit: usize,
    ) -> Vec<RecentVideo> {
        Self::pick_once(
            videos,
            |video| video.is_in_progress(now),
            |video| video.last_played_at,
            limit,
        )
    }

    /// Quick watches of `videos` (see `Video::is_quick_watch`), newest sync
    /// first, once per YouTube video, up to `limit`.
    fn quick_watches_in(videos: &[RecentVideo], limit: usize) -> Vec<RecentVideo> {
        Self::pick_once(
            videos,
            Video::is_quick_watch,
            |video| video.created_at,
            limit,
        )
    }

    /// `videos` newest sync first, once per source, up to `limit`.
    fn latest_in(mut videos: Vec<RecentVideo>, limit: usize) -> Vec<RecentVideo> {
        videos.sort_by_key(|recent| std::cmp::Reverse(recent.video.created_at));
        videos.truncate(limit);
        videos
    }

    /// The videos of `videos` that `keep` accepts, once per YouTube video,
    /// highest `newest` first, truncated to `limit`.
    fn pick_once<K: Ord>(
        videos: &[RecentVideo],
        keep: impl Fn(&Video) -> bool,
        newest: impl Fn(&Video) -> K,
        limit: usize,
    ) -> Vec<RecentVideo> {
        let mut picked = Self::once_per_youtube_video(
            videos
                .iter()
                .filter(|recent| keep(&recent.video))
                .cloned()
                .collect(),
        );

        picked.sort_by_key(|recent| std::cmp::Reverse(newest(&recent.video)));
        picked.truncate(limit);
        picked
    }

    /// `videos` without any copy of the YouTube videos in `shown`.
    fn not_shown_in(videos: &[RecentVideo], shown: &[RecentVideo]) -> Vec<RecentVideo> {
        let shown: HashSet<&VideoId> = shown
            .iter()
            .map(|recent| &recent.video.youtube_id)
            .collect();
        videos
            .iter()
            .filter(|recent| !shown.contains(&recent.video.youtube_id))
            .cloned()
            .collect()
    }

    /// Keeps the first copy of each YouTube video. Applied before sorting,
    /// while channels still come before playlists, so that is the channel
    /// copy when there is one (its card has an avatar), whatever each copy's
    /// own timestamps. Watch state is shared by every copy, so any copy is
    /// correct.
    fn once_per_youtube_video(videos: Vec<RecentVideo>) -> Vec<RecentVideo> {
        videos
            .into_iter()
            .fold(
                (HashSet::new(), Vec::new()),
                |(mut seen, mut kept), recent| {
                    if seen.insert(recent.video.youtube_id.clone()) {
                        kept.push(recent);
                    }
                    (seen, kept)
                },
            )
            .1
    }

    fn recent_from_playlists(&self) -> Result<Vec<RecentVideo>, ListVideosError> {
        let playlists = self
            .playlist_repository
            .list()
            .map_err(ListVideosError::Repository)?;

        playlists
            .iter()
            .map(|playlist| -> anyhow::Result<Vec<RecentVideo>> {
                let playlist_videos = self
                    .playlist_video_repository
                    .list_for_playlist(&playlist.id)?;
                self.video_repository
                    .find_many(
                        &playlist_videos
                            .iter()
                            .map(|pv| pv.video_id.clone())
                            .collect::<Vec<_>>(),
                    )
                    .map(|videos| {
                        videos
                            .into_iter()
                            .filter(|video| video.status == VideoStatus::Downloaded)
                            .map(|video| RecentVideo {
                                video,
                                source: VideoSource::Playlist {
                                    id: playlist.id.clone(),
                                    name: playlist.name.clone(),
                                    path: playlist.path.clone(),
                                },
                            })
                            .collect()
                    })
            })
            .collect::<anyhow::Result<Vec<Vec<RecentVideo>>>>()
            .map(|nested| nested.into_iter().flatten().collect())
            .map_err(ListVideosError::Repository)
    }

    fn recent_from_channels(&self) -> Result<Vec<RecentVideo>, ListVideosError> {
        let channels = self
            .channel_repository
            .list()
            .map_err(ListVideosError::Repository)?;

        channels
            .iter()
            .map(|channel| -> anyhow::Result<Vec<RecentVideo>> {
                let channel_videos = self
                    .channel_video_repository
                    .list_for_channel(&channel.id)?;
                self.video_repository
                    .find_many(
                        &channel_videos
                            .iter()
                            .map(|cv| cv.video_id.clone())
                            .collect::<Vec<_>>(),
                    )
                    .map(|videos| {
                        videos
                            .into_iter()
                            .filter(|video| video.status == VideoStatus::Downloaded)
                            .map(|video| RecentVideo {
                                video,
                                source: VideoSource::Channel {
                                    handle: channel.id.clone(),
                                    name: channel.name.clone(),
                                    path: channel.path.clone(),
                                    avatar_filename: channel.avatar_filename.clone(),
                                },
                            })
                            .collect()
                    })
            })
            .collect::<anyhow::Result<Vec<Vec<RecentVideo>>>>()
            .map(|nested| nested.into_iter().flatten().collect())
            .map_err(ListVideosError::Repository)
    }
}
