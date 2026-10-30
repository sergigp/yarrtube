use crate::domain::category::{Category, CategoryId};
use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::domain::services::{CategoryVideoCollector, CategoryVideoCollectorApi};
use crate::domain::video::{
    CategoryVideoView, HomeCategorySection, HomeLimits, HomeVideoView, HomeVideos, ListVideosError,
    SourcedVideo, Video, VideoId, VideoSource, VideoStatus, VideoView,
};
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_metadata_repository::VideoMetadataRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use crate::infrastructure::shared::system_clock::Clock;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Reads videos, listed by playlist, by channel, by category or for home.
#[derive(Clone)]
pub struct VideoSearcher {
    playlist_repository: Arc<dyn PlaylistRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
    video_metadata_repository: Arc<dyn VideoMetadataRepository>,
    clock: Arc<dyn Clock>,
    category_repository: Arc<dyn CategoryRepository>,
    category_video_collector: CategoryVideoCollector,
}

impl VideoSearcher {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        video_metadata_repository: Arc<dyn VideoMetadataRepository>,
        clock: Arc<dyn Clock>,
        category_repository: Arc<dyn CategoryRepository>,
        category_video_collector: CategoryVideoCollector,
    ) -> Self {
        Self {
            playlist_repository,
            playlist_video_repository,
            channel_repository,
            channel_video_repository,
            video_repository,
            video_metadata_repository,
            clock,
            category_repository,
            category_video_collector,
        }
    }
}

pub trait VideoSearcherApi: Send + Sync {
    /// Lists every video recorded for a playlist, confirming the playlist
    /// exists first, ordered by playlist position, each with its generated
    /// metadata when it has any.
    fn list_for_playlist(
        &self,
        playlist_id: &PlaylistId,
    ) -> Result<Vec<VideoView>, ListVideosError>;

    /// Lists every video recorded for a channel, confirming the channel
    /// exists first, ordered by recency (most recent first), each with its
    /// generated metadata when it has any.
    fn list_for_channel(
        &self,
        channel_id: &ChannelHandle,
    ) -> Result<Vec<VideoView>, ListVideosError>;

    /// Lists the three home sections from one collection across sources:
    /// continue watching, then quick watches without the videos shown above,
    /// then latest videos (once per source, newest sync first) without the
    /// videos shown in either. Only shown videos are left out further down.
    fn list_home(&self, limits: HomeLimits) -> Result<HomeVideos, ListVideosError>;

    /// Downloaded videos of the category's members, once per YouTube video
    /// (channel copy preferred), newest sync first.
    fn list_for_category(&self, id: &CategoryId)
    -> Result<Vec<CategoryVideoView>, ListVideosError>;
}

impl VideoSearcherApi for VideoSearcher {
    fn list_for_playlist(
        &self,
        playlist_id: &PlaylistId,
    ) -> Result<Vec<VideoView>, ListVideosError> {
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
            .filter(|video| !is_excluded(video))
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
            .filter(|video| !is_excluded(video))
            .map(|video| video.and_then(|video| self.view(video)))
            .collect::<anyhow::Result<Vec<VideoView>>>()
            .map_err(ListVideosError::Repository)
    }

    fn list_home(&self, limits: HomeLimits) -> Result<HomeVideos, ListVideosError> {
        let videos = self.downloaded_across_sources()?;
        let continue_watching =
            Self::continue_watching_in(&videos, self.clock.now(), limits.continue_watching);
        let not_continue_watching = Self::not_shown_in(&videos, &continue_watching);
        let categories = self.list_categories()?;
        let by_video = self
            .category_ids_by_video(&categories)
            .map_err(ListVideosError::Repository)?;
        let category_sections =
            Self::category_sections_in(&not_continue_watching, &categories, &by_video, limits);
        let not_in_categories = Self::not_shown_in(
            &not_continue_watching,
            &category_sections
                .iter()
                .flat_map(|(_, videos)| videos.clone())
                .collect::<Vec<_>>(),
        );
        let quick_watches = Self::quick_watches_in(&not_in_categories, limits.quick_watches);
        let latest = Self::latest_in(
            Self::not_shown_in(&not_in_categories, &quick_watches),
            limits.latest,
        );
        Ok(HomeVideos {
            continue_watching: Self::with_categories(Self::views(continue_watching), &by_video),
            categories: Self::category_section_views(category_sections, &by_video),
            quick_watches: Self::with_categories(Self::views(quick_watches), &by_video),
            latest: Self::with_categories(Self::views(latest), &by_video),
        })
    }

    fn list_for_category(
        &self,
        id: &CategoryId,
    ) -> Result<Vec<CategoryVideoView>, ListVideosError> {
        let category = self.find_category(id)?;
        let mut videos = Self::once_per_youtube_video(
            self.category_video_collector
                .downloaded_videos(&category)
                .map_err(ListVideosError::Repository)?,
        );
        videos.sort_by_key(|sourced| std::cmp::Reverse(sourced.video.created_at));
        let by_video = self
            .category_ids_by_video(&self.list_categories()?)
            .map_err(ListVideosError::Repository)?;
        videos
            .into_iter()
            .map(|sourced| self.category_view(sourced, &by_video))
            .collect::<anyhow::Result<Vec<CategoryVideoView>>>()
            .map_err(ListVideosError::Repository)
    }
}

/// Whether a looked-up video is excluded, so the video-list read paths can
/// omit it (see the `video-listing` capability). An `Err` is never excluded,
/// so a lookup failure still propagates rather than being silently dropped.
fn is_excluded(video: &anyhow::Result<Video>) -> bool {
    matches!(video, Ok(video) if video.status == VideoStatus::Excluded)
}

impl VideoSearcher {
    fn find_category(&self, id: &CategoryId) -> Result<Category, ListVideosError> {
        self.category_repository
            .find(id)
            .map_err(ListVideosError::Repository)?
            .ok_or_else(|| ListVideosError::CategoryNotFound(id.clone()))
    }

    fn view(&self, video: Video) -> anyhow::Result<VideoView> {
        let metadata = self.video_metadata_repository.find(&video.id)?;
        Ok(VideoView { video, metadata })
    }

    /// `sourced` as a category video: with its metadata and the categories
    /// having its YouTube video in `by_video` (see `category_ids_by_video`).
    fn category_view(
        &self,
        sourced: SourcedVideo,
        by_video: &HashMap<VideoId, Vec<CategoryId>>,
    ) -> anyhow::Result<CategoryVideoView> {
        let VideoView { video, metadata } = self.view(sourced.video)?;
        let category_ids = by_video.get(&video.youtube_id).cloned().unwrap_or_default();
        Ok(CategoryVideoView {
            video,
            metadata,
            source: sourced.source,
            category_ids,
        })
    }

    fn views(videos: Vec<SourcedVideo>) -> Vec<HomeVideoView> {
        videos.into_iter().map(HomeVideoView::from).collect()
    }

    /// `views`, each with the categories having its YouTube video in
    /// `by_video` (see `category_ids_by_video`).
    fn with_categories(
        views: Vec<HomeVideoView>,
        by_video: &HashMap<VideoId, Vec<CategoryId>>,
    ) -> Vec<HomeVideoView> {
        views
            .into_iter()
            .map(|view| {
                let ids = by_video.get(&view.youtube_id).cloned().unwrap_or_default();
                view.with_category_ids(ids)
            })
            .collect()
    }

    fn list_categories(&self) -> Result<Vec<Category>, ListVideosError> {
        self.category_repository
            .list()
            .map_err(ListVideosError::Repository)
    }

    /// Category sections of `videos`, ranked by pending count (most first,
    /// ties by name ignoring case), each without videos shown in earlier
    /// sections, empty ones skipped, up to `limits.category_sections`.
    fn category_sections_in(
        videos: &[SourcedVideo],
        categories: &[Category],
        by_video: &HashMap<VideoId, Vec<CategoryId>>,
        limits: HomeLimits,
    ) -> Vec<(Category, Vec<SourcedVideo>)> {
        let mut ranked: Vec<(Category, Vec<SourcedVideo>)> = categories
            .iter()
            .map(|category| {
                (
                    category.clone(),
                    Self::pending_in(videos, &category.id, by_video),
                )
            })
            .collect();
        ranked.sort_by_key(|(_, pending)| std::cmp::Reverse(pending.len()));
        ranked
            .into_iter()
            .map(|(category, mut pending)| {
                pending.truncate(limits.category_videos);
                (category, pending)
            })
            .filter(|(_, pending)| !pending.is_empty())
            .collect()
    }

    /// Downloaded, unwatched videos of `videos` in `category` per `by_video`,
    /// once per YouTube video (channel copy first), newest sync first.
    fn pending_in(
        videos: &[SourcedVideo],
        category: &CategoryId,
        by_video: &HashMap<VideoId, Vec<CategoryId>>,
    ) -> Vec<SourcedVideo> {
        Self::pick_once(
            videos,
            |video| {
                video.is_downloaded_and_unwatched()
                    && by_video
                        .get(&video.youtube_id)
                        .is_some_and(|ids| ids.contains(category))
            },
            |video| video.created_at,
            usize::MAX,
        )
    }

    /// `sections` as home category sections, their videos with the
    /// categories having them in `by_video`.
    fn category_section_views(
        sections: Vec<(Category, Vec<SourcedVideo>)>,
        by_video: &HashMap<VideoId, Vec<CategoryId>>,
    ) -> Vec<HomeCategorySection> {
        sections
            .into_iter()
            .map(|(category, videos)| HomeCategorySection {
                id: category.id,
                name: category.name,
                color: category.color,
                videos: Self::with_categories(Self::views(videos), by_video),
            })
            .collect()
    }

    /// YouTube video → ids of `categories` having it downloaded, in
    /// category-name order.
    fn category_ids_by_video(
        &self,
        categories: &[Category],
    ) -> anyhow::Result<HashMap<VideoId, Vec<CategoryId>>> {
        categories.iter().try_fold(
            HashMap::new(),
            |mut by_video: HashMap<VideoId, Vec<CategoryId>>, category| {
                self.category_video_collector
                    .downloaded_videos(category)?
                    .into_iter()
                    .for_each(|sourced| {
                        let ids = by_video.entry(sourced.video.youtube_id).or_default();
                        if !ids.contains(&category.id) {
                            ids.push(category.id.clone());
                        }
                    });
                Ok(by_video)
            },
        )
    }

    /// Every downloaded video of every tracked channel, then of every tracked
    /// playlist not excluded from home, once per source.
    fn downloaded_across_sources(&self) -> Result<Vec<SourcedVideo>, ListVideosError> {
        let mut videos = self.downloaded_from_channels()?;
        videos.extend(self.downloaded_from_playlists()?);
        Ok(videos)
    }

    /// In progress videos of `videos` (see `Video::is_in_progress`), last
    /// played first, once per YouTube video, up to `limit`.
    fn continue_watching_in(
        videos: &[SourcedVideo],
        now: DateTime<Utc>,
        limit: usize,
    ) -> Vec<SourcedVideo> {
        Self::pick_once(
            videos,
            |video| video.is_in_progress(now),
            |video| video.last_played_at,
            limit,
        )
    }

    /// Quick watches of `videos` (see `Video::is_quick_watch`), newest sync
    /// first, once per YouTube video, up to `limit`.
    fn quick_watches_in(videos: &[SourcedVideo], limit: usize) -> Vec<SourcedVideo> {
        Self::pick_once(
            videos,
            Video::is_quick_watch,
            |video| video.created_at,
            limit,
        )
    }

    /// `videos` newest sync first, once per source, up to `limit`.
    fn latest_in(mut videos: Vec<SourcedVideo>, limit: usize) -> Vec<SourcedVideo> {
        videos.sort_by_key(|sourced| std::cmp::Reverse(sourced.video.created_at));
        videos.truncate(limit);
        videos
    }

    /// The videos of `videos` that `keep` accepts, once per YouTube video,
    /// highest `newest` first, truncated to `limit`.
    fn pick_once<K: Ord>(
        videos: &[SourcedVideo],
        keep: impl Fn(&Video) -> bool,
        newest: impl Fn(&Video) -> K,
        limit: usize,
    ) -> Vec<SourcedVideo> {
        let mut picked = Self::once_per_youtube_video(
            videos
                .iter()
                .filter(|sourced| keep(&sourced.video))
                .cloned()
                .collect(),
        );

        picked.sort_by_key(|sourced| std::cmp::Reverse(newest(&sourced.video)));
        picked.truncate(limit);
        picked
    }

    /// `videos` without any copy of the YouTube videos in `shown`.
    fn not_shown_in(videos: &[SourcedVideo], shown: &[SourcedVideo]) -> Vec<SourcedVideo> {
        let shown: HashSet<&VideoId> = shown
            .iter()
            .map(|sourced| &sourced.video.youtube_id)
            .collect();
        videos
            .iter()
            .filter(|sourced| !shown.contains(&sourced.video.youtube_id))
            .cloned()
            .collect()
    }

    /// Keeps the first copy of each YouTube video. Applied before sorting,
    /// while channels still come before playlists, so that is the channel
    /// copy when there is one (its card has an avatar), whatever each copy's
    /// own timestamps. Watch state is shared by every copy, so any copy is
    /// correct.
    fn once_per_youtube_video(videos: Vec<SourcedVideo>) -> Vec<SourcedVideo> {
        videos
            .into_iter()
            .fold(
                (HashSet::new(), Vec::new()),
                |(mut seen, mut kept), sourced| {
                    if seen.insert(sourced.video.youtube_id.clone()) {
                        kept.push(sourced);
                    }
                    (seen, kept)
                },
            )
            .1
    }

    fn downloaded_from_playlists(&self) -> Result<Vec<SourcedVideo>, ListVideosError> {
        let playlists = self
            .playlist_repository
            .list()
            .map_err(ListVideosError::Repository)?;

        playlists
            .iter()
            .filter(|playlist| !playlist.exclude_from_home)
            .map(|playlist| -> anyhow::Result<Vec<SourcedVideo>> {
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
                            .map(|video| SourcedVideo {
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
            .collect::<anyhow::Result<Vec<Vec<SourcedVideo>>>>()
            .map(|nested| nested.into_iter().flatten().collect())
            .map_err(ListVideosError::Repository)
    }

    fn downloaded_from_channels(&self) -> Result<Vec<SourcedVideo>, ListVideosError> {
        let channels = self
            .channel_repository
            .list()
            .map_err(ListVideosError::Repository)?;

        channels
            .iter()
            .map(|channel| -> anyhow::Result<Vec<SourcedVideo>> {
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
                            .map(|video| SourcedVideo {
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
            .collect::<anyhow::Result<Vec<Vec<SourcedVideo>>>>()
            .map(|nested| nested.into_iter().flatten().collect())
            .map_err(ListVideosError::Repository)
    }
}
