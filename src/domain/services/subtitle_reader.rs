use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::domain::video::{ReadSubtitlesError, VideoId, WebVtt};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::sync::Arc;

/// Reads a playlist's or channel's video subtitles, as WebVTT for the
/// browser player, from the subtitle file stored next to the episode.
#[derive(Clone)]
pub struct SubtitleReader {
    playlist_repository: Arc<dyn PlaylistRepository>,
    playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
    channel_repository: Arc<dyn ChannelRepository>,
    channel_video_repository: Arc<dyn ChannelVideoRepository>,
    video_repository: Arc<dyn VideoRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    videos_path: String,
}

impl SubtitleReader {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        playlist_video_repository: Arc<dyn PlaylistVideoRepository>,
        channel_repository: Arc<dyn ChannelRepository>,
        channel_video_repository: Arc<dyn ChannelVideoRepository>,
        video_repository: Arc<dyn VideoRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            playlist_repository,
            playlist_video_repository,
            channel_repository,
            channel_video_repository,
            video_repository,
            video_file_repository,
            videos_path: videos_path.into(),
        }
    }
}

pub trait SubtitleReaderApi: Send + Sync {
    /// Reads the fetched subtitles of a video recorded for a playlist,
    /// converted to WebVTT. Fails when the playlist isn't tracked, the video
    /// isn't recorded for it, or it has no fetched subtitle file on disk.
    fn read_for_playlist(
        &self,
        playlist_id: &PlaylistId,
        video_id: &VideoId,
    ) -> Result<WebVtt, ReadSubtitlesError>;

    /// Reads the fetched subtitles of a video recorded for a channel,
    /// converted to WebVTT, failing as `read_for_playlist` does.
    fn read_for_channel(
        &self,
        channel_id: &ChannelHandle,
        video_id: &VideoId,
    ) -> Result<WebVtt, ReadSubtitlesError>;
}

impl SubtitleReaderApi for SubtitleReader {
    fn read_for_playlist(
        &self,
        _playlist_id: &PlaylistId,
        video_id: &VideoId,
    ) -> Result<WebVtt, ReadSubtitlesError> {
        Err(ReadSubtitlesError::SubtitlesNotFound(video_id.clone()))
    }

    fn read_for_channel(
        &self,
        _channel_id: &ChannelHandle,
        video_id: &VideoId,
    ) -> Result<WebVtt, ReadSubtitlesError> {
        Err(ReadSubtitlesError::SubtitlesNotFound(video_id.clone()))
    }
}
