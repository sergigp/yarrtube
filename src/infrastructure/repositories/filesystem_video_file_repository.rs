use std::path::Path;

/// Locates and deletes a video's downloaded file on disk, injected into
/// `VideoService` for the event-driven cleanup path. Separate from
/// `VideoDownloaderRepository` (`youtube_video_downloader_repository.rs`):
/// downloading shells out to `yt-dlp`, deleting is pure filesystem search
/// and removal.
pub trait VideoFileRepository: Send + Sync {
    /// Deletes every file in `output_dir` whose stem is exactly
    /// `filename_stem` or `{filename_stem} [{video_id}]` (any extension),
    /// accounting for the collision suffix `resolve_collision`
    /// (`src/infrastructure/shared/ytdlp.rs`) may have appended at download
    /// time. Returns whether any file was found and deleted. A missing
    /// `output_dir` is "nothing to delete" (`Ok(false)`), not an error.
    fn delete(
        &self,
        output_dir: &Path,
        filename_stem: &str,
        video_id: &str,
    ) -> anyhow::Result<bool>;
}

pub struct FilesystemVideoFileRepository;

impl VideoFileRepository for FilesystemVideoFileRepository {
    fn delete(
        &self,
        output_dir: &Path,
        filename_stem: &str,
        video_id: &str,
    ) -> anyhow::Result<bool> {
        let collision_stem = format!("{filename_stem} [{video_id}]");

        let entries = match std::fs::read_dir(output_dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "failed to read output directory {output_dir:?}: {e}"
                ));
            }
        };

        let mut deleted_any = false;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let stem = path.file_stem().and_then(|s| s.to_str());
            if stem == Some(filename_stem) || stem == Some(collision_stem.as_str()) {
                std::fs::remove_file(&path)
                    .map_err(|e| anyhow::anyhow!("failed to delete file {path:?}: {e}"))?;
                deleted_any = true;
            }
        }

        Ok(deleted_any)
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeVideoFileRepository {
    #[allow(clippy::type_complexity)]
    pub(crate) calls: std::sync::Mutex<Vec<(std::path::PathBuf, String, String)>>,
    pub(crate) result: std::sync::Mutex<Option<anyhow::Result<bool>>>,
}

#[cfg(test)]
impl FakeVideoFileRepository {
    pub fn new(result: anyhow::Result<bool>) -> Self {
        Self {
            calls: std::sync::Mutex::new(Vec::new()),
            result: std::sync::Mutex::new(Some(result)),
        }
    }
}

#[cfg(test)]
impl VideoFileRepository for FakeVideoFileRepository {
    fn delete(
        &self,
        output_dir: &Path,
        filename_stem: &str,
        video_id: &str,
    ) -> anyhow::Result<bool> {
        self.calls.lock().unwrap().push((
            output_dir.to_path_buf(),
            filename_stem.to_string(),
            video_id.to_string(),
        ));
        match self.result.lock().unwrap().take() {
            Some(result) => result,
            None => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::ytdlp::test_support::unique_temp_dir;

    #[test]
    fn it_should_delete_a_file_matching_the_exact_stem() {
        let dir = unique_temp_dir("video-file-repository-exact");
        std::fs::write(dir.join("My Video.mp4"), b"").unwrap();

        let deleted = FilesystemVideoFileRepository
            .delete(&dir, "My Video", "vid1")
            .unwrap();

        assert!(deleted);
        assert!(!dir.join("My Video.mp4").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_delete_a_file_matching_the_bracketed_video_id_stem() {
        let dir = unique_temp_dir("video-file-repository-bracketed");
        std::fs::write(dir.join("My Video [vid1].mp4"), b"").unwrap();

        let deleted = FilesystemVideoFileRepository
            .delete(&dir, "My Video", "vid1")
            .unwrap();

        assert!(deleted);
        assert!(!dir.join("My Video [vid1].mp4").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_no_op_when_no_file_matches() {
        let dir = unique_temp_dir("video-file-repository-no-match");
        std::fs::write(dir.join("Other Video.mp4"), b"").unwrap();

        let deleted = FilesystemVideoFileRepository
            .delete(&dir, "My Video", "vid1")
            .unwrap();

        assert!(!deleted);
        assert!(dir.join("Other Video.mp4").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_no_op_when_the_output_directory_is_missing() {
        let dir = unique_temp_dir("video-file-repository-missing-parent").join("does-not-exist");

        let deleted = FilesystemVideoFileRepository
            .delete(&dir, "My Video", "vid1")
            .unwrap();

        assert!(!deleted);
    }
}
