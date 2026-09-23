use crate::domain::directory::{Directory, DirectoryPath};
use std::path::{Path, PathBuf};

/// Lists the directories under the configured videos root, injected into
/// `DirectorySearcher` so a storage location can be chosen by browsing what
/// is actually on disk. Separate from `VideoFileRepository`
/// (`filesystem_video_file_repository.rs`), which searches and removes a
/// video's own files inside one playlist's output directory: this one only
/// ever reads directory structure, and owns the videos root that confines it.
pub trait DirectoryRepository: Send + Sync {
    /// `Ok(None)` when nothing listable is at `path`: missing, not a
    /// directory, or resolving outside the videos root. Collapsing those
    /// three into one answer is deliberate — it keeps the handler from
    /// disclosing whether a path outside the root exists.
    fn list(&self, path: &DirectoryPath) -> anyhow::Result<Option<Directory>>;
}

pub struct FilesystemDirectoryRepository {
    videos_root: PathBuf,
}

impl FilesystemDirectoryRepository {
    pub fn new(videos_root: PathBuf) -> Self {
        Self { videos_root }
    }

    /// Resolves `path` against the videos root, following every symbolic link
    /// on the way, and returns it only if the fully resolved location is
    /// still inside the root. Checking the resolved location rather than the
    /// requested one is what stops a symlink inside the root from being used
    /// to enumerate a directory outside it — the endpoint is unauthenticated,
    /// so this is the only control preventing that.
    fn resolve(&self, path: &DirectoryPath) -> anyhow::Result<Option<PathBuf>> {
        let root = canonicalize(&self.videos_root)?.ok_or_else(|| {
            anyhow::anyhow!(
                "videos root {:?} is not an existing directory",
                self.videos_root
            )
        })?;
        // The root path is spelled out rather than joined: joining the empty
        // string yields a trailing-separator path that only happens to
        // resolve back to the root.
        let target = if path.is_root() {
            root.clone()
        } else {
            root.join(path.as_str())
        };
        let Some(resolved) = canonicalize(&target)? else {
            return Ok(None);
        };

        Ok(resolved.starts_with(&root).then_some(resolved))
    }
}

impl DirectoryRepository for FilesystemDirectoryRepository {
    fn list(&self, path: &DirectoryPath) -> anyhow::Result<Option<Directory>> {
        let Some(resolved) = self.resolve(path)? else {
            return Ok(None);
        };
        if !resolved.is_dir() {
            return Ok(None);
        }

        let entries = std::fs::read_dir(&resolved).map_err(|e| {
            tracing::error!(directory = ?resolved, error = %e, "failed to read directory");
            anyhow::anyhow!("failed to read directory {resolved:?}: {e}")
        })?;

        let mut subdirectories = entries
            .map(|entry| {
                entry.map_err(|e| {
                    tracing::error!(directory = ?resolved, error = %e, "failed to read directory entry");
                    anyhow::anyhow!("failed to read directory entry: {e}")
                })
            })
            .filter(|entry| entry.as_ref().is_ok_and(|entry| entry.path().is_dir()))
            // A non-UTF-8 name is skipped rather than failing the listing: it
            // could not be rendered in the response nor sent back as a
            // `DirectoryPath`, so it is not a choosable location anyway, and
            // one oddly named directory on a share should not break browsing.
            .filter_map(|entry| entry.map(|entry| entry.file_name().into_string().ok()).transpose())
            .filter(|name| !name.as_ref().is_ok_and(|name| name.starts_with('.')))
            .collect::<anyhow::Result<Vec<String>>>()?;
        subdirectories.sort();

        Ok(Some(Directory {
            path: path.clone(),
            subdirectories,
        }))
    }
}

/// `Ok(None)` for a path that does not exist, so a missing directory reads as
/// "nothing listable here" rather than an error.
fn canonicalize(path: &Path) -> anyhow::Result<Option<PathBuf>> {
    match std::fs::canonicalize(path) {
        Ok(resolved) => Ok(Some(resolved)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => {
            tracing::error!(path = ?path, error = %e, "failed to resolve path");
            Err(anyhow::anyhow!("failed to resolve path {path:?}: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::ytdlp::test_support::unique_temp_dir;

    fn repository(videos_root: &Path) -> FilesystemDirectoryRepository {
        FilesystemDirectoryRepository::new(videos_root.to_path_buf())
    }

    fn path(path: &str) -> DirectoryPath {
        DirectoryPath::new(path).unwrap()
    }

    #[test]
    fn it_should_list_only_subdirectories_on_a_directory_containing_files_and_directories() {
        let root = unique_temp_dir("directory-repository-files-and-dirs");
        std::fs::create_dir_all(root.join("playlists")).unwrap();
        std::fs::create_dir_all(root.join("channels")).unwrap();
        std::fs::write(root.join("notes.txt"), b"").unwrap();

        let directory = repository(&root).list(&DirectoryPath::root()).unwrap();

        assert_eq!(
            directory.unwrap().subdirectories,
            vec!["channels".to_string(), "playlists".to_string()]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn it_should_omit_dot_prefixed_subdirectories() {
        let root = unique_temp_dir("directory-repository-dot-prefixed");
        std::fs::create_dir_all(root.join("playlists")).unwrap();
        std::fs::create_dir_all(root.join(".hidden")).unwrap();

        let directory = repository(&root).list(&DirectoryPath::root()).unwrap();

        assert_eq!(
            directory.unwrap().subdirectories,
            vec!["playlists".to_string()]
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn it_should_return_entries_in_a_deterministic_order() {
        let root = unique_temp_dir("directory-repository-order");
        for name in ["zulu", "alpha", "mike"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
        }
        let repository = repository(&root);

        let first = repository.list(&DirectoryPath::root()).unwrap().unwrap();
        let second = repository.list(&DirectoryPath::root()).unwrap().unwrap();

        assert_eq!(
            first.subdirectories,
            vec!["alpha".to_string(), "mike".to_string(), "zulu".to_string()]
        );
        assert_eq!(first.subdirectories, second.subdirectories);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn it_should_return_the_subdirectories_of_a_nested_directory_only() {
        let root = unique_temp_dir("directory-repository-nested");
        std::fs::create_dir_all(root.join("playlists/music/chill")).unwrap();
        std::fs::create_dir_all(root.join("channels")).unwrap();

        let directory = repository(&root).list(&path("playlists")).unwrap().unwrap();

        assert_eq!(directory.path, path("playlists"));
        assert_eq!(directory.subdirectories, vec!["music".to_string()]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn it_should_return_an_empty_listing_on_a_directory_with_no_subdirectories() {
        let root = unique_temp_dir("directory-repository-empty");
        std::fs::create_dir_all(root.join("playlists")).unwrap();

        let directory = repository(&root).list(&path("playlists")).unwrap().unwrap();

        assert!(directory.subdirectories.is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn it_should_return_none_on_a_missing_directory() {
        let root = unique_temp_dir("directory-repository-missing");

        let directory = repository(&root).list(&path("does-not-exist")).unwrap();

        assert!(directory.is_none());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn it_should_return_none_on_a_path_that_is_a_regular_file() {
        let root = unique_temp_dir("directory-repository-regular-file");
        std::fs::write(root.join("notes.txt"), b"").unwrap();

        let directory = repository(&root).list(&path("notes.txt")).unwrap();

        assert!(directory.is_none());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn it_should_return_none_on_a_symlink_resolving_outside_the_videos_root() {
        let root = unique_temp_dir("directory-repository-symlink-outside");
        let outside = unique_temp_dir("directory-repository-symlink-outside-target");
        std::fs::create_dir_all(outside.join("secrets")).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("escape")).unwrap();

        let directory = repository(&root).list(&path("escape")).unwrap();

        assert!(directory.is_none());
        std::fs::remove_dir_all(&root).unwrap();
        std::fs::remove_dir_all(&outside).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn it_should_list_the_target_on_a_symlink_resolving_inside_the_videos_root() {
        let root = unique_temp_dir("directory-repository-symlink-inside");
        std::fs::create_dir_all(root.join("playlists/music")).unwrap();
        std::os::unix::fs::symlink(root.join("playlists"), root.join("shortcut")).unwrap();

        let directory = repository(&root).list(&path("shortcut")).unwrap().unwrap();

        assert_eq!(directory.subdirectories, vec!["music".to_string()]);
        std::fs::remove_dir_all(&root).unwrap();
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct FakeDirectoryRepository {
    /// Each listable directory's path mapped to its immediate subdirectory
    /// names. A path absent from the map is "nothing listable here"
    /// (`Ok(None)`), exactly as the filesystem adapter reports a missing
    /// directory, a regular file, or a path escaping the videos root.
    pub directories: std::collections::HashMap<String, Vec<String>>,
    pub failure: Option<String>,
}

#[cfg(test)]
impl FakeDirectoryRepository {
    pub fn with_directories(directories: &[(&str, &[&str])]) -> Self {
        Self {
            directories: directories
                .iter()
                .map(|(path, subdirectories)| {
                    (
                        (*path).to_string(),
                        subdirectories.iter().map(|s| (*s).to_string()).collect(),
                    )
                })
                .collect(),
            failure: None,
        }
    }

    pub fn failing(message: &str) -> Self {
        Self {
            directories: std::collections::HashMap::new(),
            failure: Some(message.to_string()),
        }
    }
}

#[cfg(test)]
impl DirectoryRepository for FakeDirectoryRepository {
    fn list(&self, path: &DirectoryPath) -> anyhow::Result<Option<Directory>> {
        if let Some(message) = &self.failure {
            return Err(anyhow::anyhow!("{message}"));
        }
        Ok(self
            .directories
            .get(path.as_str())
            .map(|subdirectories| Directory {
                path: path.clone(),
                subdirectories: subdirectories.clone(),
            }))
    }
}
