use crate::domain::video::VideoRecordId;
use std::path::{Path, PathBuf};

/// Where downloads are staged under the videos root; dot-prefixed so Plex
/// and Kodi ignore it.
pub const STAGING_DIRNAME: &str = ".downloads";

/// What every season folder's name starts with, e.g. `Season 2026`.
pub(crate) const SEASON_FOLDER_PREFIX: &str = "Season ";

/// A show's NFO, at the root of its playlist's or channel's directory.
pub const SHOW_NFO_FILENAME: &str = "tvshow.nfo";
/// A show's poster, at the root of its playlist's or channel's directory.
pub const SHOW_POSTER_FILENAME: &str = "poster.jpg";
/// The per-video-folder NFO of the legacy layout, removed when a legacy
/// video is relocated into its season folder.
pub const MOVIE_NFO_FILENAME: &str = "movie.nfo";

/// Returns the first path component of a stored `filename`/`thumbnail_filename`:
/// a legacy video's own folder name for a `"folder/file.ext"` path (one video
/// per folder, alongside its thumbnail and `movie.nfo`), or the bare filename
/// unchanged for a legacy flat path with no `/`. Either way, this is "the
/// entry that owns this stored path, at the container's output directory
/// level" — what deletion and reconciliation delete or protect as a unit for
/// a video not yet in the TV-show layout.
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
/// layout: an `S<season>E<episode> - ` file in a `Season <number>` folder.
/// Anything else is a legacy per-video folder or flat file, including one
/// whose video title happens to start with `Season `.
pub fn is_episode_path(relative_path: &str) -> bool {
    relative_path
        .split_once('/')
        .is_some_and(|(first, rest)| is_season_folder(first) && is_episode_file_name(rest))
}

/// Whether a top-level entry of a show directory is one of its season
/// folders: `Season ` followed by the season number only.
pub fn is_season_folder(entry: &str) -> bool {
    entry
        .strip_prefix(SEASON_FOLDER_PREFIX)
        .is_some_and(is_number)
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
/// into its show: `<videos root>/.downloads/<video record id>`. Keyed by the
/// record, not the YouTube id: the same YouTube video tracked by two shows is
/// two records that may download at once, and each discards its own dir.
pub fn staging_dir(videos_path: &str, video_id: &VideoRecordId) -> PathBuf {
    Path::new(videos_path)
        .join(STAGING_DIRNAME)
        .join(video_id.as_str())
}

/// Where a video's subtitle fetch is staged: a sibling of its download and
/// thumbnail staging dir, `<videos root>/.downloads/<video record id>-subtitles`,
/// since a thumbnail fetch discards its own dir whole, possibly while a
/// subtitle fetch of the same video runs in its own lane.
pub fn subtitle_staging_dir(videos_path: &str, video_id: &VideoRecordId) -> PathBuf {
    Path::new(videos_path)
        .join(STAGING_DIRNAME)
        .join(format!("{}-subtitles", video_id.as_str()))
}

/// Whether a file name starts with an episode code: `S<season>E<episode> - `.
fn is_episode_file_name(file_name: &str) -> bool {
    file_name
        .split_once(" - ")
        .and_then(|(code, _)| code.strip_prefix('S'))
        .and_then(|code| code.split_once('E'))
        .is_some_and(|(season, episode)| is_number(season) && is_number(episode))
}

fn is_number(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
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
    fn it_should_recognize_an_episode_file_in_its_season_folder() {
        assert_eq!(
            [
                "Season 2026/S2026E031501 - My Video.mp4",
                "Season 1/S01E007 - My Video.jpg",
            ]
            .map(is_episode_path),
            [true, true]
        );
    }

    #[test]
    fn it_should_treat_a_legacy_folder_titled_like_a_season_as_legacy() {
        assert_eq!(
            [
                "Season 2 Finale/Season 2 Finale.mp4",
                "Season 2/Season 2.mp4",
                "Season Finale/Season Finale.mp4",
            ]
            .map(is_episode_path),
            [false, false, false]
        );
    }

    #[test]
    fn it_should_only_take_a_season_and_number_as_a_season_folder() {
        assert_eq!(
            [
                "Season 2026",
                "Season 1",
                "Season 2 Finale",
                "Season Finale",
                "Season "
            ]
            .map(is_season_folder),
            [true, true, false, false, false]
        );
    }

    #[test]
    fn it_should_only_split_on_the_first_separator() {
        assert_eq!(
            top_level_entry("My Video [vid1]/nested/file.mp4"),
            "My Video [vid1]"
        );
    }
}
