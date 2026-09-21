use std::path::{Path, PathBuf};

/// Returns the first path component of a stored `filename`/`thumbnail_filename`:
/// the video's own output folder name for a new-style `"folder/file.ext"` path
/// (one video per folder, alongside its thumbnail and `movie.nfo`), or the bare
/// filename unchanged for a legacy flat path with no `/` (downloaded before
/// per-video folders were introduced). Either way, this is "the entry that
/// owns this stored path, at the container's output directory level" — what
/// deletion and reconciliation need to delete or protect as a unit.
pub fn top_level_entry(relative_path: &str) -> &str {
    relative_path
        .split_once('/')
        .map_or(relative_path, |(first, _)| first)
}

/// Resolves the directory `movie.nfo` belongs in for a video whose stored
/// `filename` is relative to `output_dir`: its own per-video folder for a
/// new-style `"folder/file.ext"` path, or `output_dir` itself for a legacy
/// flat path (there is no per-video folder to place it in).
pub fn video_dir_for_filename(output_dir: &Path, filename: &str) -> PathBuf {
    match filename.split_once('/') {
        Some((folder, _)) => output_dir.join(folder),
        None => output_dir.to_path_buf(),
    }
}

/// Resolves a playlist's/channel's output directory: `videos_path` joined
/// with its recorded `path`. Shared by every service that reads or writes
/// to a container's directory on disk.
pub fn resolve_output_dir(videos_path: &str, path: &str) -> PathBuf {
    Path::new(videos_path).join(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_return_the_folder_name_for_a_nested_path() {
        assert_eq!(top_level_entry("My Video/My Video.mp4"), "My Video");
    }

    #[test]
    fn it_should_return_the_filename_unchanged_for_a_legacy_flat_path() {
        assert_eq!(top_level_entry("My Video.mp4"), "My Video.mp4");
    }

    #[test]
    fn it_should_only_split_on_the_first_separator() {
        assert_eq!(
            top_level_entry("My Video [vid1]/nested/file.mp4"),
            "My Video [vid1]"
        );
    }

    #[test]
    fn it_should_resolve_the_video_dir_as_the_owning_folder_for_a_new_style_path() {
        assert_eq!(
            video_dir_for_filename(Path::new("/videos/my-playlist"), "My Video/My Video.mp4"),
            Path::new("/videos/my-playlist/My Video")
        );
    }

    #[test]
    fn it_should_resolve_the_video_dir_as_the_output_dir_for_a_legacy_flat_path() {
        assert_eq!(
            video_dir_for_filename(Path::new("/videos/my-playlist"), "My Video.mp4"),
            Path::new("/videos/my-playlist")
        );
    }
}
