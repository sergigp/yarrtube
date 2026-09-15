use std::path::Path;

/// yt-dlp's own in-progress-download marker extensions — never treated as
/// orphaned by reconciliation, even mid-download.
const IN_PROGRESS_EXTENSIONS: &[&str] = &["part", "ytdl"];

/// Locates and deletes a video's downloaded file on disk, and lists a
/// playlist output directory's current entries, injected into
/// `VideoFileDeleter` and `VideoReconciler` for the event-driven cleanup
/// path and filesystem reconciliation. Separate from
/// `VideoDownloaderRepository` (`youtube_video_downloader_repository.rs`):
/// downloading shells out to `yt-dlp`, this is pure filesystem search,
/// listing, and removal.
pub trait VideoFileRepository: Send + Sync {
    /// Deletes the file in `output_dir` named exactly `filename`, if any.
    /// Returns whether a file was found and deleted. A missing `output_dir`
    /// or a missing file is "nothing to delete" (`Ok(false)`), not an error.
    fn delete(&self, output_dir: &Path, filename: &str) -> anyhow::Result<bool>;

    /// Lists the filenames currently present in `output_dir`, excluding
    /// yt-dlp's own in-progress-download temporary files. A missing
    /// `output_dir` is an empty listing (`Ok(vec![])`), not an error.
    fn list(&self, output_dir: &Path) -> anyhow::Result<Vec<String>>;

    /// Recursively deletes `dir` and everything in it. A missing `dir` is
    /// "nothing to delete", not an error.
    fn delete_dir_recursive(&self, dir: &Path) -> anyhow::Result<()>;
}

pub struct FilesystemVideoFileRepository;

impl VideoFileRepository for FilesystemVideoFileRepository {
    fn delete(&self, output_dir: &Path, filename: &str) -> anyhow::Result<bool> {
        let path = output_dir.join(filename);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => {
                tracing::error!(path = ?path, error = %e, "failed to delete file");
                Err(anyhow::anyhow!("failed to delete file {path:?}: {e}"))
            }
        }
    }

    fn list(&self, output_dir: &Path) -> anyhow::Result<Vec<String>> {
        let entries = match std::fs::read_dir(output_dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => {
                tracing::error!(output_dir = ?output_dir, error = %e, "failed to read output directory");
                return Err(anyhow::anyhow!(
                    "failed to read output directory {output_dir:?}: {e}"
                ));
            }
        };

        entries
            .filter_map(|entry| {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(e) => {
                        tracing::error!(output_dir = ?output_dir, error = %e, "failed to read directory entry");
                        return Some(Err(anyhow::anyhow!("failed to read directory entry: {e}")));
                    }
                };
                if is_in_progress_temp_file(&entry.path()) {
                    return None;
                }
                Some(entry.file_name().into_string().map_err(|name| {
                    tracing::error!(output_dir = ?output_dir, filename = ?name, "non-UTF-8 filename");
                    anyhow::anyhow!("non-UTF-8 filename: {name:?}")
                }))
            })
            .collect()
    }

    fn delete_dir_recursive(&self, dir: &Path) -> anyhow::Result<()> {
        match std::fs::remove_dir_all(dir) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => {
                tracing::error!(dir = ?dir, error = %e, "failed to delete directory");
                Err(anyhow::anyhow!("failed to delete directory {dir:?}: {e}"))
            }
        }
    }
}

fn is_in_progress_temp_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| IN_PROGRESS_EXTENSIONS.contains(&ext))
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeVideoFileRepository {
    pub(crate) deleted_calls: std::sync::Mutex<Vec<(std::path::PathBuf, String)>>,
    pub(crate) delete_result: std::sync::Mutex<Option<anyhow::Result<bool>>>,
    pub(crate) list_result: std::sync::Mutex<Option<anyhow::Result<Vec<String>>>>,
    pub(crate) deleted_dirs: std::sync::Mutex<Vec<std::path::PathBuf>>,
}

#[cfg(test)]
impl FakeVideoFileRepository {
    pub fn new(result: anyhow::Result<bool>) -> Self {
        Self {
            deleted_calls: std::sync::Mutex::new(Vec::new()),
            delete_result: std::sync::Mutex::new(Some(result)),
            list_result: std::sync::Mutex::new(None),
            deleted_dirs: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn with_listing(files: Vec<String>) -> Self {
        Self {
            deleted_calls: std::sync::Mutex::new(Vec::new()),
            delete_result: std::sync::Mutex::new(None),
            list_result: std::sync::Mutex::new(Some(Ok(files))),
            deleted_dirs: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[cfg(test)]
impl VideoFileRepository for FakeVideoFileRepository {
    fn delete(&self, output_dir: &Path, filename: &str) -> anyhow::Result<bool> {
        self.deleted_calls
            .lock()
            .unwrap()
            .push((output_dir.to_path_buf(), filename.to_string()));
        match self.delete_result.lock().unwrap().take() {
            Some(result) => result,
            None => Ok(false),
        }
    }

    fn list(&self, _output_dir: &Path) -> anyhow::Result<Vec<String>> {
        match self.list_result.lock().unwrap().take() {
            Some(result) => result,
            None => Ok(Vec::new()),
        }
    }

    fn delete_dir_recursive(&self, dir: &Path) -> anyhow::Result<()> {
        self.deleted_dirs.lock().unwrap().push(dir.to_path_buf());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::ytdlp::test_support::unique_temp_dir;

    #[test]
    fn it_should_delete_a_file_matching_the_exact_filename() {
        let dir = unique_temp_dir("video-file-repository-exact");
        std::fs::write(dir.join("My Video.mp4"), b"").unwrap();

        let deleted = FilesystemVideoFileRepository
            .delete(&dir, "My Video.mp4")
            .unwrap();

        assert!(deleted);
        assert!(!dir.join("My Video.mp4").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_no_op_when_no_file_matches() {
        let dir = unique_temp_dir("video-file-repository-no-match");
        std::fs::write(dir.join("Other Video.mp4"), b"").unwrap();

        let deleted = FilesystemVideoFileRepository
            .delete(&dir, "My Video.mp4")
            .unwrap();

        assert!(!deleted);
        assert!(dir.join("Other Video.mp4").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_no_op_when_the_output_directory_is_missing() {
        let dir = unique_temp_dir("video-file-repository-missing-parent").join("does-not-exist");

        let deleted = FilesystemVideoFileRepository
            .delete(&dir, "My Video.mp4")
            .unwrap();

        assert!(!deleted);
    }

    #[test]
    fn it_should_list_the_files_present_in_the_output_directory() {
        let dir = unique_temp_dir("video-file-repository-list");
        std::fs::write(dir.join("One.mp4"), b"").unwrap();
        std::fs::write(dir.join("Two.mp4"), b"").unwrap();

        let mut listed = FilesystemVideoFileRepository.list(&dir).unwrap();
        listed.sort();

        assert_eq!(listed, vec!["One.mp4".to_string(), "Two.mp4".to_string()]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_exclude_in_progress_temp_files_from_the_listing() {
        let dir = unique_temp_dir("video-file-repository-list-temp");
        std::fs::write(dir.join("Done.mp4"), b"").unwrap();
        std::fs::write(dir.join("InProgress.mp4.part"), b"").unwrap();
        std::fs::write(dir.join("InProgress.ytdl"), b"").unwrap();

        let listed = FilesystemVideoFileRepository.list(&dir).unwrap();

        assert_eq!(listed, vec!["Done.mp4".to_string()]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_return_an_empty_listing_when_the_output_directory_is_missing() {
        let dir = unique_temp_dir("video-file-repository-list-missing").join("does-not-exist");

        let listed = FilesystemVideoFileRepository.list(&dir).unwrap();

        assert!(listed.is_empty());
    }

    #[test]
    fn it_should_recursively_delete_a_directory_and_everything_in_it() {
        let dir = unique_temp_dir("video-file-repository-delete-dir");
        std::fs::create_dir_all(dir.join("nested")).unwrap();
        std::fs::write(dir.join("One.mp4"), b"").unwrap();
        std::fs::write(dir.join("nested").join("Two.mp4"), b"").unwrap();

        FilesystemVideoFileRepository
            .delete_dir_recursive(&dir)
            .unwrap();

        assert!(!dir.exists());
    }

    #[test]
    fn it_should_no_op_when_the_directory_to_delete_is_already_missing() {
        let dir =
            unique_temp_dir("video-file-repository-delete-dir-missing").join("does-not-exist");

        let result = FilesystemVideoFileRepository.delete_dir_recursive(&dir);

        assert!(result.is_ok());
    }
}
