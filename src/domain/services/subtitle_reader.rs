use crate::domain::services::{VideoOwnerFinder, VideoOwnerFinderApi};
use crate::domain::video::{ReadSubtitlesError, Video, VideoId, VideoOwner, WebVtt};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_video_repository::VideoRepository;
use std::sync::Arc;

/// Reads a stored video's subtitles, as WebVTT for the browser player, from
/// the subtitle file stored next to its episode in its show folder.
#[derive(Clone)]
pub struct SubtitleReader {
    video_repository: Arc<dyn VideoRepository>,
    video_owner_finder: Arc<VideoOwnerFinder>,
    video_file_repository: Arc<dyn VideoFileRepository>,
}

impl SubtitleReader {
    pub fn new(
        video_repository: Arc<dyn VideoRepository>,
        video_owner_finder: Arc<VideoOwnerFinder>,
        video_file_repository: Arc<dyn VideoFileRepository>,
    ) -> Self {
        Self {
            video_repository,
            video_owner_finder,
            video_file_repository,
        }
    }
}

pub trait SubtitleReaderApi: Send + Sync {
    /// Reads the fetched subtitles of the stored video `id`, converted to
    /// WebVTT. Fails when the video (or the playlist or channel holding it)
    /// isn't stored, or it has no fetched subtitle file on disk.
    fn read(&self, id: &VideoId) -> Result<WebVtt, ReadSubtitlesError>;
}

impl SubtitleReaderApi for SubtitleReader {
    fn read(&self, id: &VideoId) -> Result<WebVtt, ReadSubtitlesError> {
        let video = self.find_video(id)?;
        let owner = self.find_owner(&video)?;
        self.read_subtitles(&video, &owner)
    }
}

impl SubtitleReader {
    fn find_video(&self, id: &VideoId) -> Result<Video, ReadSubtitlesError> {
        self.video_repository
            .find(id)
            .map_err(ReadSubtitlesError::Repository)?
            .ok_or_else(|| ReadSubtitlesError::VideoNotFound(id.clone()))
    }

    /// The playlist or channel holding `video`; a video whose source is gone
    /// is as good as not stored.
    fn find_owner(&self, video: &Video) -> Result<VideoOwner, ReadSubtitlesError> {
        self.video_owner_finder
            .find(video)
            .map_err(ReadSubtitlesError::Repository)?
            .ok_or_else(|| ReadSubtitlesError::VideoNotFound(video.id.clone()))
    }

    /// Reads `video`'s subtitle file from its owner's show folder and
    /// converts it to WebVTT.
    fn read_subtitles(
        &self,
        video: &Video,
        owner: &VideoOwner,
    ) -> Result<WebVtt, ReadSubtitlesError> {
        let not_found = || ReadSubtitlesError::SubtitlesNotFound(video.id.clone());
        let filename = video.subtitles.filename.as_deref().ok_or_else(not_found)?;
        self.video_file_repository
            .read_to_string(&owner.target.output_dir, filename)
            .map_err(ReadSubtitlesError::Repository)?
            .map(|srt| WebVtt::from_srt(&srt))
            .ok_or_else(not_found)
    }
}
