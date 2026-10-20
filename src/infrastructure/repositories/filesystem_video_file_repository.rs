use crate::domain::video::is_season_folder;
use std::path::Path;

/// yt-dlp's own in-progress-download marker extensions — never treated as
/// orphaned by reconciliation, even mid-download.
const IN_PROGRESS_EXTENSIONS: &[&str] = &["part", "ytdl"];

/// Locates and deletes a video's downloaded file on disk, and lists a
/// playlist output directory's current entries, injected into
/// `VideoFileDeleter` and `PlaylistVideoReconciler` for the event-driven cleanup
/// path and filesystem reconciliation. Separate from
/// `VideoDownloaderRepository` (`youtube_video_downloader_repository.rs`):
/// downloading shells out to `yt-dlp`, this is pure filesystem search,
/// listing, and removal.
pub trait VideoFileRepository: Send + Sync {
    /// Deletes the entry in `output_dir` named exactly `filename`, if any —
    /// a whole directory (a legacy video's own folder) is removed
    /// recursively, a file is removed directly. Returns whether an entry was
    /// found and deleted. A missing `output_dir` or a missing entry is
    /// "nothing to delete" (`Ok(false)`), not an error.
    fn delete(&self, output_dir: &Path, filename: &str) -> anyhow::Result<bool>;

    /// Lists the entries currently present in `output_dir` plus every file
    /// inside its `Season ` folders (as `Season <s>/<file>`), excluding
    /// yt-dlp's own in-progress-download temporary files. A missing
    /// `output_dir` is an empty listing (`Ok(vec![])`), not an error.
    fn list(&self, output_dir: &Path) -> anyhow::Result<Vec<String>>;

    /// Recursively deletes `dir` and everything in it. A missing `dir` is
    /// "nothing to delete", not an error.
    fn delete_dir_recursive(&self, dir: &Path) -> anyhow::Result<()>;

    /// Returns whether a file exists at `output_dir.join(filename)| —
    /// `filename` may be a bare legacy-flat name or a new-style
    /// `"folder/file.ext"` path. Used by reconciliation's per-video health
    /// check, which needs to test one specific recorded path directly
    /// rather than intersect against a top-level directory listing.
    fn file_exists(&self, output_dir: &Path, filename: &str) -> bool;

    /// Moves the file at `from` to `to`, creating `to`'s parent directory
    /// first. A rename, so it is atomic within one filesystem.
    fn move_file(&self, from: &Path, to: &Path) -> anyhow::Result<()>;

    /// Removes `dir` if it is empty; a non-empty or missing `dir` is left
    /// alone, not an error.
    fn remove_if_empty(&self, dir: &Path) -> anyhow::Result<()>;

    /// Reads the UTF-8 content of the file at `output_dir.join(filename)`.
    /// A missing file is `Ok(None)`, not an error.
    fn read_to_string(&self, output_dir: &Path, filename: &str) -> anyhow::Result<Option<String>>;
}

pub struct FilesystemVideoFileRepository;

impl VideoFileRepository for FilesystemVideoFileRepository {
    fn delete(&self, output_dir: &Path, filename: &str) -> anyhow::Result<bool> {
        let path = output_dir.join(filename);
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => {
                tracing::error!(path = ?path, error = %e, "failed to inspect path for deletion");
                return Err(anyhow::anyhow!("failed to inspect path {path:?}: {e}"));
            }
        };

        let result = if metadata.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        match result {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => {
                tracing::error!(path = ?path, error = %e, "failed to delete file");
                Err(anyhow::anyhow!("failed to delete file {path:?}: {e}"))
            }
        }
    }

    fn list(&self, output_dir: &Path) -> anyhow::Result<Vec<String>> {
        let entries = list_entries(output_dir)?;
        let season_files = entries
            .iter()
            .filter(|entry| is_season_folder(entry) && output_dir.join(entry).is_dir())
            .map(|season| {
                Ok(list_entries(&output_dir.join(season))?
                    .into_iter()
                    .map(|file| format!("{season}/{file}"))
                    .collect::<Vec<_>>())
            })
            .collect::<anyhow::Result<Vec<_>>>()?
            .concat();
        Ok([entries, season_files].concat())
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

    fn file_exists(&self, output_dir: &Path, filename: &str) -> bool {
        output_dir.join(filename).is_file()
    }

    fn move_file(&self, from: &Path, to: &Path) -> anyhow::Result<()> {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)
                .inspect_err(
                    |e| tracing::error!(dir = ?parent, error = %e, "failed to create directory"),
                )
                .map_err(|e| anyhow::anyhow!("failed to create directory {parent:?}: {e}"))?;
        }
        std::fs::rename(from, to)
            .inspect_err(
                |e| tracing::error!(from = ?from, to = ?to, error = %e, "failed to move file"),
            )
            .map_err(|e| anyhow::anyhow!("failed to move {from:?} to {to:?}: {e}"))
    }

    fn remove_if_empty(&self, dir: &Path) -> anyhow::Result<()> {
        let is_empty = match std::fs::read_dir(dir) {
            Ok(mut entries) => entries.next().is_none(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => {
                tracing::error!(dir = ?dir, error = %e, "failed to read directory");
                return Err(anyhow::anyhow!("failed to read directory {dir:?}: {e}"));
            }
        };
        if !is_empty {
            return Ok(());
        }
        std::fs::remove_dir(dir)
            .inspect_err(|e| tracing::error!(dir = ?dir, error = %e, "failed to remove directory"))
            .map_err(|e| anyhow::anyhow!("failed to remove directory {dir:?}: {e}"))
    }

    fn read_to_string(&self, output_dir: &Path, filename: &str) -> anyhow::Result<Option<String>> {
        let path = output_dir.join(filename);
        match std::fs::read_to_string(&path) {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => {
                tracing::error!(path = ?path, error = %e, "failed to read file");
                Err(anyhow::anyhow!("failed to read {path:?}: {e}"))
            }
        }
    }
}

/// The entries directly in `output_dir`, excluding yt-dlp's in-progress
/// temp files; a missing `output_dir` has none.
fn list_entries(output_dir: &Path) -> anyhow::Result<Vec<String>> {
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
    /// A non-destructive copy of `with_listing`'s files, read by `file_exists`
    /// (unlike `list_result`, which `list()` consumes via `take()`) so a test
    /// that only configures a listing still gets a `file_exists` answer
    /// consistent with it, without needing to separately configure
    /// `file_exists_result`.
    pub(crate) list_snapshot: std::sync::Mutex<Option<Vec<String>>>,
    pub(crate) deleted_dirs: std::sync::Mutex<Vec<std::path::PathBuf>>,
    pub(crate) file_exists_result: std::sync::Mutex<Option<bool>>,
    pub(crate) moved: std::sync::Mutex<Vec<(std::path::PathBuf, std::path::PathBuf)>>,
    pub(crate) removed_if_empty: std::sync::Mutex<Vec<std::path::PathBuf>>,
    /// Runs on every `move_file`, before it is recorded, e.g. to observe
    /// what is already on disk when a file is moved into place.
    #[allow(clippy::type_complexity)]
    pub(crate) on_move: Option<Box<dyn Fn(&Path, &Path) + Send + Sync>>,
    /// Sources whose `move_file` fails, as a permission error would.
    pub(crate) failing_moves: Vec<std::path::PathBuf>,
    /// Files `read_to_string` finds, keyed by their full path.
    pub(crate) files: Vec<(std::path::PathBuf, String)>,
}

#[cfg(test)]
impl FakeVideoFileRepository {
    pub fn new(result: anyhow::Result<bool>) -> Self {
        Self {
            deleted_calls: std::sync::Mutex::new(Vec::new()),
            delete_result: std::sync::Mutex::new(Some(result)),
            list_result: std::sync::Mutex::new(None),
            list_snapshot: std::sync::Mutex::new(None),
            deleted_dirs: std::sync::Mutex::new(Vec::new()),
            file_exists_result: std::sync::Mutex::new(None),
            ..Self::default()
        }
    }

    pub fn with_listing(files: Vec<String>) -> Self {
        Self {
            deleted_calls: std::sync::Mutex::new(Vec::new()),
            delete_result: std::sync::Mutex::new(None),
            list_result: std::sync::Mutex::new(Some(Ok(files.clone()))),
            list_snapshot: std::sync::Mutex::new(Some(files)),
            deleted_dirs: std::sync::Mutex::new(Vec::new()),
            file_exists_result: std::sync::Mutex::new(None),
            ..Self::default()
        }
    }

    pub fn with_on_move(self, hook: impl Fn(&Path, &Path) + Send + Sync + 'static) -> Self {
        Self {
            on_move: Some(Box::new(hook)),
            ..self
        }
    }

    /// The same fake, failing every move of `from`.
    pub fn with_failing_move(self, from: impl Into<std::path::PathBuf>) -> Self {
        Self {
            failing_moves: [self.failing_moves, vec![from.into()]].concat(),
            ..self
        }
    }

    /// The same fake, holding a file at `path` with `content`.
    pub fn with_file(self, path: impl Into<std::path::PathBuf>, content: &str) -> Self {
        Self {
            files: [self.files, vec![(path.into(), content.to_string())]].concat(),
            ..self
        }
    }

    pub fn with_file_exists(exists: bool) -> Self {
        Self {
            deleted_calls: std::sync::Mutex::new(Vec::new()),
            delete_result: std::sync::Mutex::new(None),
            list_result: std::sync::Mutex::new(None),
            list_snapshot: std::sync::Mutex::new(None),
            deleted_dirs: std::sync::Mutex::new(Vec::new()),
            file_exists_result: std::sync::Mutex::new(Some(exists)),
            ..Self::default()
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

    fn file_exists(&self, _output_dir: &Path, filename: &str) -> bool {
        if let Some(explicit) = *self.file_exists_result.lock().unwrap() {
            return explicit;
        }
        self.list_snapshot
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|files| files.iter().any(|f| f == filename))
    }

    fn move_file(&self, from: &Path, to: &Path) -> anyhow::Result<()> {
        if let Some(hook) = &self.on_move {
            hook(from, to);
        }
        if self.failing_moves.iter().any(|failing| failing == from) {
            anyhow::bail!("permission denied moving {from:?}");
        }
        self.moved
            .lock()
            .unwrap()
            .push((from.to_path_buf(), to.to_path_buf()));
        Ok(())
    }

    fn remove_if_empty(&self, dir: &Path) -> anyhow::Result<()> {
        self.removed_if_empty
            .lock()
            .unwrap()
            .push(dir.to_path_buf());
        Ok(())
    }

    fn read_to_string(&self, output_dir: &Path, filename: &str) -> anyhow::Result<Option<String>> {
        let path = output_dir.join(filename);
        Ok(self
            .files
            .iter()
            .find(|(file, _)| *file == path)
            .map(|(_, content)| content.clone()))
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
    fn it_should_list_top_level_entries_and_season_files() {
        let dir = unique_temp_dir("video-file-repository-list-seasons");
        std::fs::write(dir.join("tvshow.nfo"), b"").unwrap();
        std::fs::create_dir_all(dir.join("Legacy")).unwrap();
        std::fs::write(dir.join("Legacy/Legacy.mp4"), b"").unwrap();
        std::fs::create_dir_all(dir.join("Season 2026")).unwrap();
        std::fs::write(dir.join("Season 2026/S2026E031501 - One.mp4"), b"").unwrap();
        std::fs::write(dir.join("Season 2026/S2026E031502 - Two.mp4.part"), b"").unwrap();

        let mut listed = FilesystemVideoFileRepository.list(&dir).unwrap();
        listed.sort();

        assert_eq!(
            listed,
            vec![
                "Legacy".to_string(),
                "Season 2026".to_string(),
                "Season 2026/S2026E031501 - One.mp4".to_string(),
                "tvshow.nfo".to_string(),
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_move_a_file_creating_its_parent() {
        let dir = unique_temp_dir("video-file-repository-move");
        std::fs::create_dir_all(dir.join(".downloads/yt1")).unwrap();
        std::fs::write(dir.join(".downloads/yt1/yt1.mp4"), b"media").unwrap();
        let to = dir.join("show/Season 2026/S2026E031501 - One.mp4");

        FilesystemVideoFileRepository
            .move_file(&dir.join(".downloads/yt1/yt1.mp4"), &to)
            .unwrap();

        assert_eq!(std::fs::read(&to).ok(), Some(b"media".to_vec()));
        assert!(!dir.join(".downloads/yt1/yt1.mp4").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_remove_an_empty_dir_only() {
        let dir = unique_temp_dir("video-file-repository-remove-if-empty");
        std::fs::create_dir_all(dir.join("Empty")).unwrap();
        std::fs::create_dir_all(dir.join("Full")).unwrap();
        std::fs::write(dir.join("Full/leftover.mp4"), b"").unwrap();

        let results = [
            FilesystemVideoFileRepository.remove_if_empty(&dir.join("Empty")),
            FilesystemVideoFileRepository.remove_if_empty(&dir.join("Full")),
            FilesystemVideoFileRepository.remove_if_empty(&dir.join("Missing")),
        ];

        assert_eq!(results.map(|result| result.is_ok()), [true, true, true]);
        assert_eq!(
            [dir.join("Empty"), dir.join("Full/leftover.mp4")].map(|path| path.exists()),
            [false, true]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_fail_to_remove_a_dir_it_cannot_read() {
        let dir = unique_temp_dir("video-file-repository-remove-if-empty-unreadable");
        let not_a_dir = dir.join("leftover.mp4");
        std::fs::write(&not_a_dir, b"").unwrap();

        let result = FilesystemVideoFileRepository.remove_if_empty(&not_a_dir);

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err(format!(
                "failed to read directory {not_a_dir:?}: Not a directory (os error 20)"
            ))
        );
        assert!(not_a_dir.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_fail_to_list_an_output_directory_it_cannot_read() {
        let dir = unique_temp_dir("video-file-repository-list-unreadable");
        let not_a_dir = dir.join("leftover.mp4");
        std::fs::write(&not_a_dir, b"").unwrap();

        let result = FilesystemVideoFileRepository.list(&not_a_dir);

        assert_eq!(
            result.map_err(|e| e.to_string()),
            Err(format!(
                "failed to read output directory {not_a_dir:?}: Not a directory (os error 20)"
            ))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_delete_a_file_or_a_dir_by_relative_path() {
        let dir = unique_temp_dir("video-file-repository-delete-relative");
        std::fs::create_dir_all(dir.join("Season 2026")).unwrap();
        std::fs::write(dir.join("Season 2026/S2026E031501 - One.mp4"), b"").unwrap();
        std::fs::create_dir_all(dir.join("Legacy")).unwrap();
        std::fs::write(dir.join("Legacy/Legacy.mp4"), b"").unwrap();

        let deleted = [
            FilesystemVideoFileRepository
                .delete(&dir, "Season 2026/S2026E031501 - One.mp4")
                .unwrap(),
            FilesystemVideoFileRepository
                .delete(&dir, "Legacy")
                .unwrap(),
        ];

        assert_eq!(deleted, [true, true]);
        assert_eq!(
            [
                dir.join("Season 2026/S2026E031501 - One.mp4"),
                dir.join("Season 2026"),
                dir.join("Legacy"),
            ]
            .map(|path| path.exists()),
            [false, true, false]
        );
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

    #[test]
    fn it_should_delete_a_whole_directory_when_the_named_entry_is_a_folder() {
        let dir = unique_temp_dir("video-file-repository-delete-entry-dir");
        std::fs::create_dir_all(dir.join("My Video")).unwrap();
        std::fs::write(dir.join("My Video").join("My Video.mp4"), b"").unwrap();
        std::fs::write(dir.join("My Video").join("meta.nfo"), b"").unwrap();

        let deleted = FilesystemVideoFileRepository
            .delete(&dir, "My Video")
            .unwrap();

        assert!(deleted);
        assert!(!dir.join("My Video").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_return_true_when_a_nested_file_exists() {
        let dir = unique_temp_dir("video-file-repository-exists-nested");
        std::fs::create_dir_all(dir.join("My Video")).unwrap();
        std::fs::write(dir.join("My Video").join("My Video.mp4"), b"").unwrap();

        let exists = FilesystemVideoFileRepository.file_exists(&dir, "My Video/My Video.mp4");

        assert!(exists);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_return_true_when_a_flat_legacy_file_exists() {
        let dir = unique_temp_dir("video-file-repository-exists-flat");
        std::fs::write(dir.join("My Video.mp4"), b"").unwrap();

        let exists = FilesystemVideoFileRepository.file_exists(&dir, "My Video.mp4");

        assert!(exists);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_return_false_when_the_file_is_missing() {
        let dir = unique_temp_dir("video-file-repository-exists-missing");

        let exists = FilesystemVideoFileRepository.file_exists(&dir, "Nonexistent.mp4");

        assert!(!exists);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_read_a_files_content() {
        let dir = unique_temp_dir("video-file-repository-read");
        std::fs::create_dir_all(dir.join("Season 1")).unwrap();
        std::fs::write(dir.join("Season 1/My Video.en.srt"), "1\nHello there\n").unwrap();

        let content = FilesystemVideoFileRepository
            .read_to_string(&dir, "Season 1/My Video.en.srt")
            .unwrap();

        assert_eq!(content, Some("1\nHello there\n".to_string()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_read_none_when_the_file_is_missing() {
        let dir = unique_temp_dir("video-file-repository-read-missing");

        let content =
            FilesystemVideoFileRepository.read_to_string(&dir, "Season 1/My Video.en.srt");

        assert_eq!(content.map_err(|e| e.to_string()), Ok(None));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn it_should_fail_to_read_a_file_that_is_not_utf8() {
        let dir = unique_temp_dir("video-file-repository-read-not-utf8");
        std::fs::create_dir_all(dir.join("Season 1")).unwrap();
        let path = dir.join("Season 1/My Video.en.srt");
        std::fs::write(&path, [0xff, 0xfe, 0xfd]).unwrap();

        let content =
            FilesystemVideoFileRepository.read_to_string(&dir, "Season 1/My Video.en.srt");

        assert_eq!(
            content.map_err(|e| e.to_string()),
            Err(format!(
                "failed to read {path:?}: stream did not contain valid UTF-8"
            ))
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
