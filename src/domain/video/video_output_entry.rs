use crate::domain::video::VideoId;
use std::path::{Path, PathBuf};

/// Where downloads are staged under the videos root; dot-prefixed so Plex
/// and Kodi ignore it.
pub const STAGING_DIRNAME: &str = ".downloads";

/// What every season folder's name starts with, e.g. `Season 2026`.
const SEASON_FOLDER_PREFIX: &str = "Season ";

/// A show's NFO, at the root of its playlist's or channel's directory.
pub const SHOW_NFO_FILENAME: &str = "tvshow.nfo";
/// A show's poster, at the root of its playlist's or channel's directory.
pub const SHOW_POSTER_FILENAME: &str = "poster.jpg";

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

/// The entry of a show directory a recorded path is deleted or protected
/// by: the path itself for an episode file in its season folder, else its
/// legacy top-level folder or flat file.
pub fn owning_entry(relative_path: &str) -> &str {
    if is_episode_path(relative_path) {
        relative_path
    } else {
        top_level_entry(relative_path)
    }
}

/// Resolves a playlist's/channel's output directory: `videos_path` joined
/// with its recorded `path`. Shared by every service that reads or writes
/// to a container's directory on disk.
pub fn resolve_output_dir(videos_path: &str, path: &str) -> PathBuf {
    Path::new(videos_path).join(path)
}

/// Whether a recorded `filename`/`thumbnail_filename` is in the TV-show
/// layout: its first segment is a `Season ` folder. Anything else is a legacy
/// per-video folder or flat file.
pub fn is_episode_path(relative_path: &str) -> bool {
    relative_path
        .split_once('/')
        .is_some_and(|(first, _)| is_season_folder(first))
}

/// Whether a top-level entry of a show directory is one of its `Season `
/// folders.
pub fn is_season_folder(entry: &str) -> bool {
    !entry.contains('/') && entry.starts_with(SEASON_FOLDER_PREFIX)
}

/// The episode NFO sitting next to an episode file: the same path with a
/// `.nfo` extension.
pub fn nfo_path_for(relative_path: &str) -> String {
    Path::new(relative_path)
        .with_extension("nfo")
        .to_string_lossy()
        .into_owned()
}

/// Where a video's download or thumbnail fetch is staged before being moved
/// into its show: `<videos root>/.downloads/<youtube id>`.
pub fn staging_dir(videos_path: &str, youtube_id: &VideoId) -> PathBuf {
    Path::new(videos_path)
        .join(STAGING_DIRNAME)
        .join(youtube_id.as_str())
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
}
