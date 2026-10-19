use crate::domain::channel::ChannelHandle;
use crate::domain::playlist::PlaylistId;
use crate::domain::video::{is_episode_path, nfo_path_for, owning_entry, resolve_output_dir};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use std::path::Path;
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
}

pub trait VideoFileDeleterApi: Send + Sync {
    /// Deletes a removed video's downloaded file, and its thumbnail and
    /// subtitle files if any, from disk, scheduled by
    /// `subscribers::delete_video_file_on_video_removed_from_playlist`/
    /// `..._channel` whenever a downloaded video is removed from its
    /// container. `output_dir` is the container's already-resolved output
    /// directory. An episode's media file, its NFO, its thumbnail and its
    /// subtitle file are deleted from its season folder, leaving the show
    /// files and the folder in place; a legacy video's per-video folder is
    /// removed as one unit, a legacy flat video's file and thumbnail
    /// individually. No-ops (without erroring) for any entry it has no
    /// recorded filename for, or no matching entry is found, so the task is
    /// safe to retry.
    fn delete_video_file(
        &self,
        filename: Option<String>,
        thumbnail_filename: Option<String>,
        subtitle_filename: Option<String>,
        output_dir: &Path,
    ) -> anyhow::Result<()>;

    /// Recursively deletes a deleted playlist's output directory from disk,
    /// scheduled by `subscribers::delete_playlist_files_on_playlist_deleted`
    /// whenever a playlist is deleted. The playlist row is already gone by
    /// the time this runs, so `path` travels with the task instead of being
    /// looked up. Safe to do unconditionally because playlist paths are
    /// unique, so nothing else lives at that path.
    fn delete_playlist_video_files(
        &self,
        playlist_id: PlaylistId,
        path: String,
    ) -> anyhow::Result<()>;

    /// Recursively deletes a deleted channel's output directory from disk,
    /// scheduled by `subscribers::delete_channel_files_on_channel_deleted`
    /// whenever a channel is deleted. Mirrors `delete_playlist_video_files`.
    fn delete_channel_video_files(
        &self,
        channel_id: ChannelHandle,
        path: String,
    ) -> anyhow::Result<()>;
}

impl VideoFileDeleterApi for VideoFileDeleter {
    fn delete_video_file(
        &self,
        filename: Option<String>,
        thumbnail_filename: Option<String>,
        subtitle_filename: Option<String>,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        self.delete_video_entry(filename, output_dir)?;
        self.delete_sidecar_entry(thumbnail_filename, "thumbnail", output_dir)?;
        self.delete_sidecar_entry(subtitle_filename, "subtitle", output_dir)
    }

    fn delete_playlist_video_files(
        &self,
        playlist_id: PlaylistId,
        path: String,
    ) -> anyhow::Result<()> {
        let output_dir = resolve_output_dir(&self.videos_path, &path);
        self.video_file_repository
            .delete_dir_recursive(&output_dir)?;
        info!(playlist_id = %playlist_id, path = %path, "deleted playlist output directory");
        Ok(())
    }

    fn delete_channel_video_files(
        &self,
        channel_id: ChannelHandle,
        path: String,
    ) -> anyhow::Result<()> {
        let output_dir = resolve_output_dir(&self.videos_path, &path);
        self.video_file_repository
            .delete_dir_recursive(&output_dir)?;
        info!(channel_id = %channel_id, path = %path, "deleted channel output directory");
        Ok(())
    }
}

impl VideoFileDeleter {
    /// An episode's media file and the NFO next to it, or a legacy video's
    /// whole folder or flat file.
    fn delete_video_entry(
        &self,
        filename: Option<String>,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        let Some(filename) = filename else {
            debug!("video has no recorded filename, skipping file deletion");
            return Ok(());
        };
        let nfo = is_episode_path(&filename).then(|| nfo_path_for(&filename));
        std::iter::once(owning_entry(&filename))
            .chain(nfo.as_deref())
            .try_for_each(|entry| self.delete_entry(output_dir, entry))
    }

    /// A file recorded next to the video's own, e.g. its thumbnail or
    /// subtitle file.
    fn delete_sidecar_entry(
        &self,
        sidecar_filename: Option<String>,
        kind: &str,
        output_dir: &Path,
    ) -> anyhow::Result<()> {
        let Some(sidecar_filename) = sidecar_filename else {
            debug!(
                kind,
                "video has no recorded sidecar filename, skipping file deletion"
            );
            return Ok(());
        };
        self.delete_entry(output_dir, owning_entry(&sidecar_filename))
    }

    fn delete_entry(&self, output_dir: &Path, entry: &str) -> anyhow::Result<()> {
        if self.video_file_repository.delete(output_dir, entry)? {
            info!(entry, "deleted video file");
        } else {
            debug!(entry, "no matching video file found to delete");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FakeVideoFileRepository;
    use std::path::PathBuf;

    fn deleter(repo: FakeVideoFileRepository) -> (VideoFileDeleter, Arc<FakeVideoFileRepository>) {
        let repo = Arc::new(repo);
        (VideoFileDeleter::new(repo.clone(), "/videos"), repo)
    }

    #[test]
    fn it_should_delete_the_whole_folder_once_per_recorded_path_for_a_new_style_video() {
        let (deleter, repo) = deleter(FakeVideoFileRepository::default());

        deleter
            .delete_video_file(
                Some("My Video/My Video.mp4".to_string()),
                Some("My Video/My Video.jpg".to_string()),
                None,
                Path::new("/videos/playlist"),
            )
            .unwrap();

        let calls = repo.deleted_calls.lock().unwrap();
        assert_eq!(
            *calls,
            vec![
                (PathBuf::from("/videos/playlist"), "My Video".to_string()),
                (PathBuf::from("/videos/playlist"), "My Video".to_string()),
            ]
        );
    }

    #[test]
    fn it_should_delete_the_two_named_files_individually_for_a_legacy_flat_video() {
        let (deleter, repo) = deleter(FakeVideoFileRepository::default());

        deleter
            .delete_video_file(
                Some("My Video.mp4".to_string()),
                Some("My Video.jpg".to_string()),
                None,
                Path::new("/videos/playlist"),
            )
            .unwrap();

        let calls = repo.deleted_calls.lock().unwrap();
        assert_eq!(
            *calls,
            vec![
                (
                    PathBuf::from("/videos/playlist"),
                    "My Video.mp4".to_string()
                ),
                (
                    PathBuf::from("/videos/playlist"),
                    "My Video.jpg".to_string()
                ),
            ]
        );
    }

    #[test]
    fn it_should_no_op_when_no_filename_or_thumbnail_is_recorded() {
        let (deleter, repo) = deleter(FakeVideoFileRepository::default());

        deleter
            .delete_video_file(None, None, None, Path::new("/videos/playlist"))
            .unwrap();

        assert!(repo.deleted_calls.lock().unwrap().is_empty());
    }
}
