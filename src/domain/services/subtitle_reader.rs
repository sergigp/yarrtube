use crate::domain::channel::{Channel, ChannelHandle};
use crate::domain::playlist::{Playlist, PlaylistId};
use crate::domain::video::{ReadSubtitlesError, Video, VideoId, WebVtt};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_channel_repository::ChannelRepository;
use crate::infrastructure::repositories::sqlite_channel_video_repository::ChannelVideoRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use crate::infrastructure::repositories::sqlite_playlist_video_repository::PlaylistVideoRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::path::Path;
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
        playlist_id: &PlaylistId,
        video_id: &VideoId,
    ) -> Result<WebVtt, ReadSubtitlesError> {
        let playlist = self.find_playlist(playlist_id)?;
        let video = self.find_playlist_video(playlist_id, video_id)?;
        self.read_subtitles(&video, playlist.path.as_str())
    }

    fn read_for_channel(
        &self,
        channel_id: &ChannelHandle,
        video_id: &VideoId,
    ) -> Result<WebVtt, ReadSubtitlesError> {
        let channel = self.find_channel(channel_id)?;
        let video = self.find_channel_video(channel_id, video_id)?;
        self.read_subtitles(&video, channel.path.as_str())
    }
}

impl SubtitleReader {
    fn find_playlist(&self, playlist_id: &PlaylistId) -> Result<Playlist, ReadSubtitlesError> {
        self.playlist_repository
            .find(playlist_id)
            .map_err(ReadSubtitlesError::Repository)?
            .ok_or_else(|| ReadSubtitlesError::PlaylistNotFound(playlist_id.clone()))
    }

    fn find_channel(&self, channel_id: &ChannelHandle) -> Result<Channel, ReadSubtitlesError> {
        self.channel_repository
            .find(channel_id)
            .map_err(ReadSubtitlesError::Repository)?
            .ok_or_else(|| ReadSubtitlesError::ChannelNotFound(channel_id.clone()))
    }

    fn find_playlist_video(
        &self,
        playlist_id: &PlaylistId,
        video_id: &VideoId,
    ) -> Result<Video, ReadSubtitlesError> {
        self.find_video(video_id, |video| {
            Ok(self
                .playlist_video_repository
                .find_by_video(&video.id)?
                .is_some_and(|playlist_video| playlist_video.playlist_id == *playlist_id))
        })
    }

    fn find_channel_video(
        &self,
        channel_id: &ChannelHandle,
        video_id: &VideoId,
    ) -> Result<Video, ReadSubtitlesError> {
        self.find_video(video_id, |video| {
            Ok(self
                .channel_video_repository
                .find_by_video(&video.id)?
                .is_some_and(|channel_video| channel_video.channel_id == *channel_id))
        })
    }

    /// The video recorded as `video_id` that `is_recorded_for` the source:
    /// one YouTube video can be recorded once per playlist or channel.
    fn find_video(
        &self,
        video_id: &VideoId,
        is_recorded_for: impl Fn(&Video) -> anyhow::Result<bool>,
    ) -> Result<Video, ReadSubtitlesError> {
        self.video_repository
            .find_by_youtube_id(video_id)
            .map_err(ReadSubtitlesError::Repository)?
            .into_iter()
            .map(|video| is_recorded_for(&video).map(|recorded| recorded.then_some(video)))
            .find_map(Result::transpose)
            .transpose()
            .map_err(ReadSubtitlesError::Repository)?
            .ok_or_else(|| ReadSubtitlesError::VideoNotFound(video_id.clone()))
    }

    /// Reads `video`'s subtitle file from its show folder at `source_path`
    /// and converts it to WebVTT.
    fn read_subtitles(
        &self,
        video: &Video,
        source_path: &str,
    ) -> Result<WebVtt, ReadSubtitlesError> {
        let not_found = || ReadSubtitlesError::SubtitlesNotFound(video.youtube_id.clone());
        let filename = video.subtitle_filename.as_deref().ok_or_else(not_found)?;
        let output_dir = Path::new(&self.videos_path).join(source_path);
        self.video_file_repository
            .read_to_string(&output_dir, filename)
            .map_err(ReadSubtitlesError::Repository)?
            .map(|srt| WebVtt::from_srt(&srt))
            .ok_or_else(not_found)
    }
}
