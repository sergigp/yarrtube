use crate::domain::video::{VideoRecordId, staging_dir, subtitle_staging_dir};
use crate::infrastructure::repositories::filesystem_video_file_repository::VideoFileRepository;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::warn;

/// The staging root under the videos root, handing out one video's staging
/// dir at a time as a `StagedDir` that discards it when dropped, so every
/// outcome of a download or fetch cleans up after itself.
pub struct StagingArea {
    video_file_repository: Arc<dyn VideoFileRepository>,
    videos_path: String,
}

/// A video's staging dir, removed with everything in it when dropped.
/// Best-effort: a leftover staging dir is harmless, Plex and Kodi ignore the
/// staging root, so a failed removal is only logged.
pub struct StagedDir {
    video_file_repository: Arc<dyn VideoFileRepository>,
    video_id: VideoRecordId,
    path: PathBuf,
}

impl StagingArea {
    pub fn new(
        video_file_repository: Arc<dyn VideoFileRepository>,
        videos_path: impl Into<String>,
    ) -> Self {
        Self {
            video_file_repository,
            videos_path: videos_path.into(),
        }
    }

    /// The staging dir a video's download or thumbnail fetch writes into.
    pub fn for_video(&self, video_id: &VideoRecordId) -> StagedDir {
        self.staged(video_id, staging_dir(&self.videos_path, video_id))
    }

    /// The staging dir a video's subtitle fetch writes into, a sibling of
    /// its download's.
    pub fn for_subtitles(&self, video_id: &VideoRecordId) -> StagedDir {
        self.staged(video_id, subtitle_staging_dir(&self.videos_path, video_id))
    }

    fn staged(&self, video_id: &VideoRecordId, path: PathBuf) -> StagedDir {
        StagedDir {
            video_file_repository: self.video_file_repository.clone(),
            video_id: video_id.clone(),
            path,
        }
    }
}

impl StagedDir {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for StagedDir {
    fn drop(&mut self) {
        if let Err(e) = self.video_file_repository.delete_dir_recursive(&self.path) {
            warn!(video_id = %self.video_id, dir = ?self.path, error = %e, "failed to discard the staging dir");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::filesystem_video_file_repository::FilesystemVideoFileRepository;
    use crate::infrastructure::shared::ytdlp::test_support::unique_temp_dir;

    #[test]
    fn it_should_discard_a_video_staging_dir_when_dropped() {
        let videos_root = unique_temp_dir("staging-area-video");
        let staging_area = StagingArea::new(
            Arc::new(FilesystemVideoFileRepository),
            videos_root.to_string_lossy(),
        );

        let staged = staging_area.for_video(&video_id());
        let path = staged.path().to_path_buf();
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("v1.mp4"), b"").unwrap();
        drop(staged);

        assert_eq!(path, videos_root.join(".downloads").join("v1"));
        assert!(!path.exists());
        std::fs::remove_dir_all(&videos_root).unwrap();
    }

    #[test]
    fn it_should_discard_a_subtitle_staging_dir_when_dropped() {
        let videos_root = unique_temp_dir("staging-area-subtitles");
        let staging_area = StagingArea::new(
            Arc::new(FilesystemVideoFileRepository),
            videos_root.to_string_lossy(),
        );

        let staged = staging_area.for_subtitles(&video_id());
        let path = staged.path().to_path_buf();
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("yt1.en.srt"), b"").unwrap();
        drop(staged);

        assert_eq!(path, videos_root.join(".downloads").join("v1-subtitles"));
        assert!(!path.exists());
        std::fs::remove_dir_all(&videos_root).unwrap();
    }

    #[test]
    fn it_should_ignore_a_staging_dir_never_created() {
        let videos_root = unique_temp_dir("staging-area-missing");
        let staging_area = StagingArea::new(
            Arc::new(FilesystemVideoFileRepository),
            videos_root.to_string_lossy(),
        );

        drop(staging_area.for_video(&video_id()));

        assert!(!videos_root.join(".downloads").join("v1").exists());
        std::fs::remove_dir_all(&videos_root).unwrap();
    }

    fn video_id() -> VideoRecordId {
        VideoRecordId::new("v1").unwrap()
    }
}
