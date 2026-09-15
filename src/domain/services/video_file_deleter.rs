use crate::domain::shared::{PlaylistId, VideoId};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use crate::infrastructure::repositories::sqlite_playlist_repository::PlaylistRepository;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// Deletes a video's downloaded file, or a whole playlist's output
/// directory, from disk.
#[derive(Clone)]
pub struct VideoFileDeleter {
    playlist_repository: Arc<dyn PlaylistRepository>,
    video_file_repository: Arc<dyn VideoFileRepository>,
    videos_path: String,
}

impl VideoFileDeleter {
    pub fn new(
        playlist_repository: Arc<dyn PlaylistRepository>,
        video_file_repository: Arc<dyn VideoFileRepository>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            playlist_repository,
            video_file_repository,
            videos_path: videos_path.into(),
        }
    }

    /// Deletes a removed video's downloaded file from disk, scheduled by
    /// `subscribers::delete_video_file_on_video_deleted` whenever a
    /// downloaded video is removed from its playlist. No-ops (without
    /// erroring) if the video has no recorded filename, the playlist no
    /// longer exists, or no matching file is found, so the task is safe to
    /// retry.
    pub fn delete_video_file(
        &self,
        playlist_id: PlaylistId,
        video_id: VideoId,
        filename: Option<String>,
    ) -> anyhow::Result<()> {
        let Some(filename) = filename else {
            debug!(playlist_id = %playlist_id, video_id = %video_id, "video has no recorded filename, skipping file deletion");
            return Ok(());
        };
        let Some(playlist) = self.playlist_repository.find(&playlist_id)? else {
            debug!(playlist_id = %playlist_id, "playlist no longer exists, skipping file deletion");
            return Ok(());
        };

        let output_dir = Path::new(&self.videos_path).join(playlist.path.as_str());
        let deleted = self.video_file_repository.delete(&output_dir, &filename)?;

        if deleted {
            info!(playlist_id = %playlist_id, video_id = %video_id, "deleted video file");
        } else {
            debug!(playlist_id = %playlist_id, video_id = %video_id, "no matching video file found to delete");
        }

        Ok(())
    }

    /// Recursively deletes a deleted playlist's output directory from disk,
    /// scheduled by `subscribers::delete_playlist_files_on_playlist_deleted`
    /// whenever a playlist is deleted. The playlist row is already gone by
    /// the time this runs, so `path` travels with the task instead of being
    /// looked up. Safe to do unconditionally because playlist paths are
    /// unique, so nothing else lives at that path.
    pub fn delete_playlist_video_files(
        &self,
        playlist_id: PlaylistId,
        path: String,
    ) -> anyhow::Result<()> {
        let output_dir = Path::new(&self.videos_path).join(&path);
        self.video_file_repository
            .delete_dir_recursive(&output_dir)?;
        info!(playlist_id = %playlist_id, path = %path, "deleted playlist output directory");
        Ok(())
    }
}
