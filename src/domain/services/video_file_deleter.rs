use crate::domain::channel::ChannelHandle;
use crate::domain::shared::PlaylistId;
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info};

/// Deletes a video's downloaded file, or a whole playlist's/channel's output
/// directory, from disk.
#[derive(Clone)]
pub struct VideoFileDeleter {
    video_file_repository: Arc<dyn VideoFileRepository>,
    videos_path: String,
}

impl VideoFileDeleter {
    pub fn new(
        video_file_repository: Arc<dyn VideoFileRepository>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_file_repository,
            videos_path: videos_path.into(),
        }
    }

    /// Deletes a removed video's downloaded file from disk, scheduled by
    /// `subscribers::delete_video_file_on_video_removed_from_playlist`/
    /// `..._channel` whenever a downloaded video is removed from its
    /// container. `output_dir` is the container's already-resolved output
    /// directory. No-ops (without erroring) if the video has no recorded
    /// filename or no matching file is found, so the task is safe to retry.
    pub fn delete_video_file(
        &self,
        filename: Option<String>,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        let Some(filename) = filename else {
            debug!("video has no recorded filename, skipping file deletion");
            return Ok(());
        };

        let deleted = self.video_file_repository.delete(output_dir, &filename)?;

        if deleted {
            info!(filename, "deleted video file");
        } else {
            debug!(filename, "no matching video file found to delete");
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
        let output_dir = self.output_dir(&path);
        self.video_file_repository
            .delete_dir_recursive(&output_dir)?;
        info!(playlist_id = %playlist_id, path = %path, "deleted playlist output directory");
        Ok(())
    }

    /// Recursively deletes a deleted channel's output directory from disk,
    /// scheduled by `subscribers::delete_channel_files_on_channel_deleted`
    /// whenever a channel is deleted. Mirrors `delete_playlist_video_files`.
    pub fn delete_channel_video_files(
        &self,
        channel_id: ChannelHandle,
        path: String,
    ) -> anyhow::Result<()> {
        let output_dir = self.output_dir(&path);
        self.video_file_repository
            .delete_dir_recursive(&output_dir)?;
        info!(channel_id = %channel_id, path = %path, "deleted channel output directory");
        Ok(())
    }

    fn output_dir(&self, path: &str) -> PathBuf {
        Path::new(&self.videos_path).join(path)
    }
}
